use aws_config::SdkConfig;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;

use crate::error::VaporError;

#[derive(Debug)]
pub struct QldbLedgerInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub state: Option<String>,
    pub creation_date_time: Option<aws_smithy_types::DateTime>,
    pub permissions_mode: Option<String>,
    pub deletion_protection: Option<bool>,
    pub kms_key_arn: Option<String>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct QldbJournalExportInfo {
    pub ledger_name: String,
    pub export_id: String,
    pub export_creation_time: Option<aws_smithy_types::DateTime>,
    pub status: Option<String>,
    pub inclusive_start_time: Option<aws_smithy_types::DateTime>,
    pub exclusive_end_time: Option<aws_smithy_types::DateTime>,
    pub output_format: Option<String>,
}

pub struct QldbClient {
    inner: aws_sdk_qldb::Client,
}

impl QldbClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_qldb::Client::new(config),
        }
    }

    async fn get_ledger_details(&self, name: &str) -> Result<QldbLedgerInfo, VaporError> {
        let desc = self
            .inner
            .describe_ledger()
            .name(name)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        self.ledger_info_from_output(desc).await
    }

    async fn ledger_info_from_output(
        &self,
        desc: aws_sdk_qldb::operation::describe_ledger::DescribeLedgerOutput,
    ) -> Result<QldbLedgerInfo, VaporError> {
        let arn = desc.arn().map(|s| s.to_string());

        let tags = if let Some(ref arn_str) = arn {
            let tags_output = self
                .inner
                .list_tags_for_resource()
                .resource_arn(arn_str)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            tags_output
                .tags()
                .into_iter()
                .flat_map(|map| {
                    map.iter()
                        .filter_map(|(k, v)| v.as_ref().map(|val| (k.clone(), val.clone())))
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            vec![]
        };

        Ok(QldbLedgerInfo {
            name: desc.name().map(|s| s.to_string()),
            arn,
            state: desc.state().map(|s| s.as_str().to_string()),
            creation_date_time: desc.creation_date_time().cloned(),
            permissions_mode: desc.permissions_mode().map(|s| s.as_str().to_string()),
            deletion_protection: desc.deletion_protection(),
            kms_key_arn: desc
                .encryption_description()
                .map(|e| e.kms_key_arn().to_string()),
            tags,
        })
    }

    /// Lists ledgers, optionally capped at `limit` results (default unlimited), returning a
    /// resumption token when more results remain.
    pub async fn list_ledgers(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<QldbLedgerInfo>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_ledgers();
            if let Some(l) = limit {
                req = req.max_results(l - names.len() as i32);
            }
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for ledger in output.ledgers.unwrap_or_default() {
                if let Some(name) = ledger.name {
                    names.push(name);
                }
            }
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| names.len() as i32 >= l) {
                break;
            }
        }

        let mut results = Vec::new();
        for name in names {
            results.push(self.get_ledger_details(&name).await?);
        }
        Ok((results, token))
    }

    pub async fn describe_ledger(&self, name: &str) -> Result<Option<QldbLedgerInfo>, VaporError> {
        match self.inner.describe_ledger().name(name).send().await {
            Ok(desc) => Ok(Some(self.ledger_info_from_output(desc).await?)),
            Err(e) => {
                if matches!(e.code(), Some("ResourceNotFoundException")) {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }

    /// Lists journal S3 exports for a ledger, optionally capped at `limit` results (default
    /// unlimited), returning a resumption token when more results remain.
    pub async fn list_journal_s3_exports(
        &self,
        ledger_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<QldbJournalExportInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_journal_s3_exports_for_ledger()
                .name(ledger_name);
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for export in output.journal_s3_exports.unwrap_or_default() {
                items.push(QldbJournalExportInfo {
                    ledger_name: export.ledger_name,
                    export_id: export.export_id,
                    export_creation_time: Some(export.export_creation_time),
                    status: Some(export.status.as_str().to_string()),
                    inclusive_start_time: Some(export.inclusive_start_time),
                    exclusive_end_time: Some(export.exclusive_end_time),
                    output_format: export.output_format.map(|f| f.as_str().to_string()),
                });
            }
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use aws_smithy_types::DateTime;

    const BASE: &str = "https://qldb.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_ledgers_lists_all_when_no_limit() {
        // `list_ledgers` is a discovery-loop-then-fan-out method (memory
        // gotcha 9): one `ListLedgers` call collects names, then each name
        // gets its own `DescribeLedger` call (+ a `ListTagsForResource` call
        // if the description has an `Arn`).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers"), ""),
                json_response(200, r#"{"Ledgers":[{"Name":"ledger-a","State":"ACTIVE","CreationDateTime":1700000000}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-a"), ""),
                json_response(
                    200,
                    r#"{"Name":"ledger-a","Arn":"arn:aws:qldb:us-east-1:123456789012:ledger/ledger-a","State":"ACTIVE","CreationDateTime":1700000000,"PermissionsMode":"STANDARD","DeletionProtection":true,"EncryptionDescription":{"KmsKeyArn":"arn:aws:kms:us-east-1:123456789012:key/abc","EncryptionStatus":"ENABLED"}}"#,
                ),
            ),
            ReplayEvent::new(
                // `ListTagsForResource`'s `resource_arn` is an `httpLabel`
                // param with `EncodingStrategy::Default`, which percent-encodes
                // both `:` and `/` (memory gotcha 18).
                request(
                    &format!("{BASE}/tags/arn%3Aaws%3Aqldb%3Aus-east-1%3A123456789012%3Aledger%2Fledger-a"),
                    "",
                ),
                json_response(200, r#"{"Tags":{"env":"prod"}}"#),
            ),
        ]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (ledgers, token) = client.list_ledgers(None, None).await.unwrap();

        assert_eq!(ledgers.len(), 1);
        let l = &ledgers[0];
        assert_eq!(l.name, Some("ledger-a".to_string()));
        assert_eq!(
            l.arn,
            Some("arn:aws:qldb:us-east-1:123456789012:ledger/ledger-a".to_string())
        );
        assert_eq!(l.state, Some("ACTIVE".to_string()));
        assert_eq!(
            l.creation_date_time,
            Some(DateTime::from_secs(1_700_000_000))
        );
        assert_eq!(l.permissions_mode, Some("STANDARD".to_string()));
        assert_eq!(l.deletion_protection, Some(true));
        assert_eq!(
            l.kms_key_arn,
            Some("arn:aws:kms:us-east-1:123456789012:key/abc".to_string())
        );
        assert_eq!(l.tags, vec![("env".to_string(), "prod".to_string())]);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ledgers_skips_tag_fetch_when_arn_missing() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers"), ""),
                json_response(200, r#"{"Ledgers":[{"Name":"ledger-b"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-b"), ""),
                json_response(200, r#"{"Name":"ledger-b"}"#),
            ),
        ]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (ledgers, token) = client.list_ledgers(None, None).await.unwrap();

        assert_eq!(ledgers.len(), 1);
        let l = &ledgers[0];
        assert_eq!(l.arn, None);
        assert_eq!(l.tags, Vec::new());

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ledgers_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers?next_token=cursor-a"), ""),
                json_response(200, r#"{"Ledgers":[{"Name":"ledger-c"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-c"), ""),
                json_response(200, r#"{"Name":"ledger-c"}"#),
            ),
        ]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (ledgers, token) = client
            .list_ledgers(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(ledgers.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ledgers_stops_at_limit_and_returns_upstream_next_token() {
        // `ListLedgersInput` has a real `max_results` field (query param,
        // snake_case per this crate's codegen — not the PascalCase/camelCase
        // seen in most other services, extends memory gotcha 2) and the
        // wrapper forwards `limit` straight to it with no client-side
        // truncate, so the canned page must return exactly `limit` names
        // (memory gotcha 13).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers?max_results=1"), ""),
                json_response(
                    200,
                    r#"{"Ledgers":[{"Name":"ledger-d"}],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-d"), ""),
                json_response(200, r#"{"Name":"ledger-d"}"#),
            ),
        ]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (ledgers, token) = client.list_ledgers(Some(1), None).await.unwrap();

        assert_eq!(ledgers.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ledgers_pages_through_until_exhausted_when_limit_not_reached() {
        // Unlike some discovery-loop shapes, `list_ledgers` finishes paging
        // *before* fanning out: both `ListLedgers` pages happen first, then
        // both `DescribeLedger` fan-out calls.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers?max_results=10"), ""),
                json_response(200, r#"{"Ledgers":[{"Name":"ledger-e"}],"NextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers?max_results=9&next_token=p2"), ""),
                json_response(200, r#"{"Ledgers":[{"Name":"ledger-f"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-e"), ""),
                json_response(200, r#"{"Name":"ledger-e"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-f"), ""),
                json_response(200, r#"{"Name":"ledger-f"}"#),
            ),
        ]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (ledgers, token) = client.list_ledgers(Some(10), None).await.unwrap();

        assert_eq!(ledgers.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ledgers_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ledgers"), ""),
            json_error_response("InvalidParameterException", "bad request"),
        )]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let err = client.list_ledgers(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ledgers_propagates_fan_out_describe_errors() {
        // Unlike `elasticache.rs`'s tag fan-out (memory gotcha 10, which
        // folds a failed fan-out call into an empty default via `.ok()`/
        // `.unwrap_or_default()`), `get_ledger_details` propagates a failed
        // `DescribeLedger` fan-out call via `?`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers"), ""),
                json_response(200, r#"{"Ledgers":[{"Name":"ledger-g"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-g"), ""),
                json_error_response("ResourceNotFoundException", "gone"),
            ),
        ]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let err = client.list_ledgers(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_ledger_returns_details_when_found() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-h"), ""),
                json_response(
                    200,
                    r#"{"Name":"ledger-h","Arn":"arn:aws:qldb:us-east-1:123456789012:ledger/ledger-h","State":"ACTIVE"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/tags/arn%3Aaws%3Aqldb%3Aus-east-1%3A123456789012%3Aledger%2Fledger-h"),
                    "",
                ),
                json_response(200, r#"{"Tags":{}}"#),
            ),
        ]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let ledger = client.describe_ledger("ledger-h").await.unwrap();

        let l = ledger.unwrap();
        assert_eq!(l.name, Some("ledger-h".to_string()));
        assert_eq!(l.state, Some("ACTIVE".to_string()));
        assert_eq!(l.tags, Vec::new());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_ledger_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ledgers/missing-ledger"), ""),
            json_error_response("ResourceNotFoundException", "no such ledger"),
        )]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let ledger = client.describe_ledger("missing-ledger").await.unwrap();

        assert!(ledger.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_ledger_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ledgers/ledger-i"), ""),
            json_error_response("InvalidParameterException", "bad name"),
        )]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_ledger("ledger-i").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "bad name");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_journal_s3_exports_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ledgers/ledger-j/journal-s3-exports"), ""),
            json_response(
                200,
                r#"{"JournalS3Exports":[{"LedgerName":"ledger-j","ExportId":"export-1","ExportCreationTime":1700000000,"Status":"COMPLETED","InclusiveStartTime":1699990000,"ExclusiveEndTime":1700000000,"OutputFormat":"JSON"}]}"#,
            ),
        )]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_journal_s3_exports("ledger-j", None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        let e = &items[0];
        assert_eq!(e.ledger_name, "ledger-j");
        assert_eq!(e.export_id, "export-1");
        assert_eq!(
            e.export_creation_time,
            Some(DateTime::from_secs(1_700_000_000))
        );
        assert_eq!(e.status, Some("COMPLETED".to_string()));
        assert_eq!(
            e.inclusive_start_time,
            Some(DateTime::from_secs(1_699_990_000))
        );
        assert_eq!(
            e.exclusive_end_time,
            Some(DateTime::from_secs(1_700_000_000))
        );
        assert_eq!(e.output_format, Some("JSON".to_string()));

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_journal_s3_exports_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/ledgers/ledger-k/journal-s3-exports?next_token=cursor-a"),
                "",
            ),
            json_response(
                200,
                r#"{"JournalS3Exports":[{"LedgerName":"ledger-k","ExportId":"export-2"}]}"#,
            ),
        )]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_journal_s3_exports("ledger-k", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_journal_s3_exports_stops_at_limit_and_returns_upstream_next_token() {
        // Same no-client-truncate shape as `list_ledgers` above:
        // `ListJournalS3ExportsForLedgerInput` forwards `limit` straight to
        // `max_results`, so the canned page must return exactly `limit`
        // items.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/ledgers/ledger-l/journal-s3-exports?max_results=1"),
                "",
            ),
            json_response(
                200,
                r#"{"JournalS3Exports":[{"LedgerName":"ledger-l","ExportId":"export-3"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_journal_s3_exports("ledger-l", Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_journal_s3_exports_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/ledgers/ledger-m/journal-s3-exports?max_results=10"),
                    "",
                ),
                json_response(
                    200,
                    r#"{"JournalS3Exports":[{"LedgerName":"ledger-m","ExportId":"export-4"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!(
                        "{BASE}/ledgers/ledger-m/journal-s3-exports?max_results=9&next_token=p2"
                    ),
                    "",
                ),
                json_response(
                    200,
                    r#"{"JournalS3Exports":[{"LedgerName":"ledger-m","ExportId":"export-5"}]}"#,
                ),
            ),
        ]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_journal_s3_exports("ledger-m", Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_journal_s3_exports_defaults_missing_fields() {
        // `journal_s3_export_description_correct_errors` (this pinned SDK's
        // `serde_util.rs`) default-fills every field the response object
        // leaves unset: `LedgerName`/`ExportId`/`RoleArn` become `""` (bare
        // `String` on the type, not `Option` — memory gotcha 16's shape,
        // confirmed by reading `_journal_s3_export_description.rs`), the
        // three timestamps default to epoch 0, and a missing `Status` parses
        // the literal string `"no value was set"` into `ExportStatus::Unknown`
        // (memory gotcha 20's shape, not a real modeled variant).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ledgers/ledger-o/journal-s3-exports"), ""),
            json_response(200, r#"{"JournalS3Exports":[{}]}"#),
        )]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .list_journal_s3_exports("ledger-o", None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        let e = &items[0];
        assert_eq!(e.ledger_name, "");
        assert_eq!(e.export_id, "");
        assert_eq!(e.status, Some("no value was set".to_string()));
        assert_eq!(e.export_creation_time, Some(DateTime::from_secs(0)));
        assert_eq!(e.output_format, None);

        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_journal_s3_exports_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ledgers/ledger-n/journal-s3-exports"), ""),
            json_error_response("ResourceNotFoundException", "no such ledger"),
        )]);
        let client = QldbClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_journal_s3_exports("ledger-n", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "no such ledger");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
