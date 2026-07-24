use aws_config::SdkConfig;
use aws_sdk_macie2::types::{
    BucketMetadata, CriterionAdditionalProperties, Finding, FindingCriteria,
};

use crate::error::VaporError;

pub struct MacieClient {
    inner: aws_sdk_macie2::Client,
}

impl MacieClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_macie2::Client::new(config),
        }
    }

    /// Lists finding IDs matching `severity`/`finding_type`, optionally capped
    /// at `limit` (default unlimited) and resumed from `next_token`. `limit`
    /// is handed to AWS via `ListFindingsInput::max_results` so a capped page
    /// boundary lands exactly on the returned token.
    pub async fn list_findings(
        &self,
        severity: Option<&str>,
        finding_type: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let finding_criteria = if severity.is_some() || finding_type.is_some() {
            let mut b = FindingCriteria::builder();
            if let Some(sev) = severity {
                b = b.criterion(
                    "severity.description",
                    CriterionAdditionalProperties::builder()
                        .set_eq(Some(vec![sev.to_string()]))
                        .build(),
                );
            }
            if let Some(ft) = finding_type {
                b = b.criterion(
                    "type",
                    CriterionAdditionalProperties::builder()
                        .set_eq(Some(vec![ft.to_string()]))
                        .build(),
                );
            }
            Some(b.build())
        } else {
            None
        };

        let mut ids = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_findings();
            if let Some(ref fc) = finding_criteria {
                req = req.finding_criteria(fc.clone());
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - ids.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            ids.extend(output.finding_ids.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if ids.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((ids, token))
    }

    pub async fn get_findings(&self, ids: Vec<String>) -> Result<Vec<Finding>, VaporError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let mut results = Vec::new();
        for chunk in ids.chunks(25) {
            let output = self
                .inner
                .get_findings()
                .set_finding_ids(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            results.extend(output.findings().iter().cloned());
        }
        Ok(results)
    }

    /// Lists S3 bucket sensitivity/classification summaries, optionally capped
    /// at `limit` (default unlimited) and resumed from `next_token`. `limit`
    /// is handed to AWS via `DescribeBucketsInput::max_results` so a capped
    /// page boundary lands exactly on the returned token.
    pub async fn describe_buckets(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<BucketMetadata>, Option<String>), VaporError> {
        let mut buckets = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_buckets();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - buckets.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            buckets.extend(output.buckets.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if buckets.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((buckets, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use aws_sdk_macie2::types::{FindingType, SeverityDescription};

    const BASE: &str = "https://macie2.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_findings_lists_all_when_no_limit_or_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/findings"), "{}"),
            json_response(200, r#"{"findingIds":["f-1","f-2"]}"#),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client.list_findings(None, None, None, None).await.unwrap();

        assert_eq!(ids, vec!["f-1".to_string(), "f-2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_passes_through_severity_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/findings"),
                r#"{"findingCriteria":{"criterion":{"severity.description":{"eq":["High"]}}}}"#,
            ),
            json_response(200, r#"{"findingIds":["f-1"]}"#),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (ids, _) = client
            .list_findings(Some("High"), None, None, None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["f-1".to_string()]);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_passes_through_finding_type_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/findings"),
                r#"{"findingCriteria":{"criterion":{"type":{"eq":["SensitiveData:S3Object/Multiple"]}}}}"#,
            ),
            json_response(200, r#"{"findingIds":["f-1"]}"#),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (ids, _) = client
            .list_findings(None, Some("SensitiveData:S3Object/Multiple"), None, None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["f-1".to_string()]);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/findings"), r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"findingIds":[]}"#),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_findings(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(ids.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_stops_at_limit_and_returns_resume_token() {
        // `max_results` is forwarded straight to AWS with no client-side
        // truncate (gotcha 13's AWS-side category), so the canned response
        // must return exactly `limit` items, not more.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/findings"), r#"{"maxResults":1}"#),
            json_response(200, r#"{"findingIds":["f-1"],"nextToken":"page2-token"}"#),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_findings(None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["f-1".to_string()]);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/findings"), r#"{"maxResults":10}"#),
                json_response(200, r#"{"findingIds":["f-1","f-2"],"nextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/findings"),
                    r#"{"nextToken":"p2","maxResults":8}"#,
                ),
                json_response(200, r#"{"findingIds":["f-3"]}"#),
            ),
        ]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_findings(None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(ids.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_propagates_errors() {
        // `ValidationException`, not a throttling-classified code (see memory
        // gotcha: those get retried and exhaust the single replay event,
        // surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/findings"), "{}"),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_findings(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_returns_empty_without_request_when_ids_empty() {
        let http_client = StaticReplayClient::new(vec![]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let findings = client.get_findings(vec![]).await.unwrap();

        assert_eq!(findings.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_fetches_findings_for_given_ids() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/findings/describe"),
                r#"{"findingIds":["f-1"]}"#,
            ),
            json_response(
                200,
                r#"{"findings":[{"id":"f-1","title":"Exposed credentials","severity":{"description":"High","score":3},"type":"SensitiveData:S3Object/Multiple"}]}"#,
            ),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let findings = client.get_findings(vec!["f-1".to_string()]).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id(), Some("f-1"));
        assert_eq!(findings[0].title(), Some("Exposed credentials"));
        assert_eq!(
            findings[0].severity().and_then(|s| s.description()),
            Some(&SeverityDescription::High)
        );
        assert_eq!(
            findings[0].r#type(),
            Some(&FindingType::SensitiveDataS3ObjectMultiple)
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_chunks_ids_into_batches_of_25() {
        let ids: Vec<String> = (0..30).map(|i| format!("f-{i}")).collect();
        let first_chunk: Vec<String> = ids[..25].to_vec();
        let second_chunk: Vec<String> = ids[25..].to_vec();

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/findings/describe"),
                    format!(
                        r#"{{"findingIds":{}}}"#,
                        serde_json::to_string(&first_chunk).unwrap()
                    ),
                ),
                json_response(200, r#"{"findings":[{"id":"f-0"}]}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/findings/describe"),
                    format!(
                        r#"{{"findingIds":{}}}"#,
                        serde_json::to_string(&second_chunk).unwrap()
                    ),
                ),
                json_response(200, r#"{"findings":[{"id":"f-25"}]}"#),
            ),
        ]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let findings = client.get_findings(ids).await.unwrap();

        assert_eq!(findings.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/findings/describe"),
                r#"{"findingIds":["f-1"]}"#,
            ),
            json_error_response("ResourceNotFoundException", "no such finding"),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_findings(vec!["f-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "no such finding");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_buckets_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/datasources/s3"), "{}"),
            json_response(
                200,
                r#"{"buckets":[{"bucketName":"my-bucket","accountId":"111122223333","region":"us-east-1"}]}"#,
            ),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (buckets, token) = client.describe_buckets(None, None).await.unwrap();

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].bucket_name(), Some("my-bucket"));
        assert_eq!(buckets[0].account_id(), Some("111122223333"));
        assert_eq!(buckets[0].region(), Some("us-east-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_buckets_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/datasources/s3"),
                r#"{"nextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"buckets":[]}"#),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (buckets, token) = client
            .describe_buckets(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(buckets.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_buckets_stops_at_limit_and_returns_resume_token() {
        // `max_results` is forwarded straight to AWS with no client-side
        // truncate (gotcha 13's AWS-side category), so the canned response
        // must return exactly `limit` items, not more.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/datasources/s3"), r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"buckets":[{"bucketName":"bucket-1"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (buckets, token) = client.describe_buckets(Some(1), None).await.unwrap();

        assert_eq!(buckets.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_buckets_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/datasources/s3"), r#"{"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"buckets":[{"bucketName":"bucket-1"},{"bucketName":"bucket-2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/datasources/s3"),
                    r#"{"nextToken":"p2","maxResults":8}"#,
                ),
                json_response(200, r#"{"buckets":[{"bucketName":"bucket-3"}]}"#),
            ),
        ]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let (buckets, token) = client.describe_buckets(Some(10), None).await.unwrap();

        assert_eq!(buckets.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_buckets_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/datasources/s3"), "{}"),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = MacieClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_buckets(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
