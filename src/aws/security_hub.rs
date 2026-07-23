use aws_config::SdkConfig;
use aws_sdk_securityhub::types::{
    AwsSecurityFinding, AwsSecurityFindingFilters, StringFilter, StringFilterComparison,
};

use crate::error::VaporError;

pub struct SecurityHubClient {
    inner: aws_sdk_securityhub::Client,
}

impl SecurityHubClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_securityhub::Client::new(config),
        }
    }

    /// Lists findings matching the given filters, optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `limit` is handed to AWS via `GetFindingsInput::max_results` so a
    /// capped page boundary lands exactly on the returned token
    /// (kinesis/mq pattern; no documented min/max constraint on
    /// `max_results`, verified against pinned `aws-sdk-securityhub`
    /// 1.115.0's `operation/get_findings/_get_findings_input.rs`).
    pub async fn get_findings(
        &self,
        severity_label: Option<String>,
        workflow_status: Option<String>,
        record_state: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AwsSecurityFinding>, Option<String>), VaporError> {
        let has_filters =
            severity_label.is_some() || workflow_status.is_some() || record_state.is_some();

        let filters = if has_filters {
            let mut fb = AwsSecurityFindingFilters::builder();

            if let Some(sl) = severity_label {
                let f = StringFilter::builder()
                    .value(sl)
                    .comparison(StringFilterComparison::Equals)
                    .build();
                fb = fb.severity_label(f);
            }

            if let Some(ws) = workflow_status {
                let f = StringFilter::builder()
                    .value(ws)
                    .comparison(StringFilterComparison::Equals)
                    .build();
                fb = fb.workflow_status(f);
            }

            if let Some(rs) = record_state {
                let f = StringFilter::builder()
                    .value(rs)
                    .comparison(StringFilterComparison::Equals)
                    .build();
                fb = fb.record_state(f);
            }

            Some(fb.build())
        } else {
            None
        };

        let mut findings = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_findings();
            if let Some(ref f) = filters {
                req = req.filters(f.clone());
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - findings.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            findings.extend(output.findings.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if findings.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((findings, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    // restJson1: POST JSON to a fixed `/findings` path (verified against
    // pinned `aws-sdk-securityhub` 1.115.0's `operation/get_findings.rs`
    // `GetFindingsRequestSerializer::uri_base`, NOT the generic
    // awsJson1.1-to-`/`-root shape used by `ram.rs`/`secrets_manager.rs` —
    // don't assume a fixed-root path from a JSON-body sibling without
    // checking `uri_base` directly). Crate name (`aws-sdk-securityhub`)
    // matches the endpoint hostname (`securityhub.*`, verified against
    // pinned crate's `config/endpoint.rs`). Request/response bodies use
    // PascalCase keys throughout (`Filters`, `SeverityLabel`, `Findings`,
    // `NextToken`, ...) per `ser_get_findings_input_input`/`de_get_findings`
    // codegen. `get_findings`'s aws-layer pagination loop forwards `limit`
    // straight to AWS's `MaxResults` with no client-side truncation (memory
    // gotcha 13), so the capped-pagination test below cans exactly `limit`
    // items. Each of `severity_label`/`workflow_status`/`record_state` maps
    // to a single-element `Vec<StringFilter>` on the wire (`SeverityLabel`/
    // `WorkflowStatus`/`RecordState`), each with `Comparison: "EQUALS"`
    // (verified against `StringFilterComparison::Equals::as_str()` ==
    // `"EQUALS"`, memory gotcha 15). `InvalidInputException` (not a
    // throttling-classified code, memory gotcha 1) is used for the
    // `propagates_errors` test since it's modeled on `GetFindings`.
    const BASE: &str = "https://securityhub.us-east-1.amazonaws.com/findings";

    #[tokio::test]
    async fn get_findings_lists_all_with_no_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Findings":[{"Id":"finding-1","Title":"Exposed S3 bucket","SchemaVersion":"2018-10-08","ProductArn":"arn:aws:securityhub:us-east-1::product/aws/securityhub","GeneratorId":"gen-1","AwsAccountId":"111111111111","CreatedAt":"2026-01-01T00:00:00Z","UpdatedAt":"2026-01-01T00:00:00Z"}]}"#,
            ),
        )]);
        let client = SecurityHubClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client
            .get_findings(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id(), Some("finding-1"));
        assert_eq!(findings[0].title(), Some("Exposed S3 bucket"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_builds_filters_from_severity_workflow_and_record_state() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Filters":{"SeverityLabel":[{"Value":"CRITICAL","Comparison":"EQUALS"}],"WorkflowStatus":[{"Value":"NEW","Comparison":"EQUALS"}],"RecordState":[{"Value":"ACTIVE","Comparison":"EQUALS"}]}}"#,
            ),
            json_response(200, r#"{"Findings":[]}"#),
        )]);
        let client = SecurityHubClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client
            .get_findings(
                Some("CRITICAL".to_string()),
                Some("NEW".to_string()),
                Some("ACTIVE".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(findings.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Findings":[]}"#),
        )]);
        let client = SecurityHubClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client
            .get_findings(None, None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(findings.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Findings":[{"Id":"finding-1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SecurityHubClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client
            .get_findings(None, None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Findings":[{"Id":"finding-1"},{"Id":"finding-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Findings":[{"Id":"finding-3"}]}"#),
            ),
        ]);
        let client = SecurityHubClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client
            .get_findings(None, None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(findings.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidInputException", "bad filter"),
        )]);
        let client = SecurityHubClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_findings(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "bad filter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

