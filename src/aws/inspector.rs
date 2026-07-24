use aws_config::SdkConfig;
use aws_sdk_inspector2::types::{
    CoveredResource, FilterCriteria, Finding, StringComparison, StringFilter,
};

use crate::error::VaporError;

pub struct InspectorClient {
    inner: aws_sdk_inspector2::Client,
}

impl InspectorClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_inspector2::Client::new(config),
        }
    }

    /// Lists findings, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListFindings` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-inspector2` 1.116.0's
    /// `operation/list_findings/_list_findings_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_findings(
        &self,
        severity: Option<String>,
        resource_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Finding>, Option<String>), VaporError> {
        let filter = if severity.is_some() || resource_type.is_some() {
            let mut fb = FilterCriteria::builder();
            if let Some(sev) = severity {
                let f = StringFilter::builder()
                    .value(sev)
                    .comparison(StringComparison::Equals)
                    .build()
                    .map_err(|e| VaporError::AwsSdk {
                        code: None,
                        message: e.to_string(),
                    })?;
                fb = fb.severity(f);
            }
            if let Some(rt) = resource_type {
                let f = StringFilter::builder()
                    .value(rt)
                    .comparison(StringComparison::Equals)
                    .build()
                    .map_err(|e| VaporError::AwsSdk {
                        code: None,
                        message: e.to_string(),
                    })?;
                fb = fb.resource_type(f);
            }
            Some(fb.build())
        } else {
            None
        };

        let mut findings = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_findings();
            if let Some(f) = filter.clone() {
                req = req.filter_criteria(f);
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

    /// Lists covered resources, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListCoverage` has
    /// both `max_results` and `next_token`, same shape as `list_findings`
    /// above.
    pub async fn list_coverage(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<CoveredResource>, Option<String>), VaporError> {
        let mut resources = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_coverage();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - resources.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            resources.extend(output.covered_resources.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if resources.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((resources, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use aws_sdk_inspector2::types::Severity;

    const FINDINGS_URL: &str = "https://inspector2.us-east-1.amazonaws.com/findings/list";
    const COVERAGE_URL: &str = "https://inspector2.us-east-1.amazonaws.com/coverage/list";

    #[tokio::test]
    async fn list_findings_lists_all_when_no_limit_or_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FINDINGS_URL, "{}"),
            json_response(
                200,
                r#"{"findings":[{"findingArn":"arn1","awsAccountId":"111122223333","severity":"HIGH","description":"d1","type":"PACKAGE_VULNERABILITY"},{"findingArn":"arn2","severity":"CRITICAL"}]}"#,
            ),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client.list_findings(None, None, None, None).await.unwrap();

        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].finding_arn(), "arn1");
        assert_eq!(findings[0].aws_account_id(), "111122223333");
        assert_eq!(findings[0].severity(), &Severity::High);
        assert_eq!(findings[1].severity(), &Severity::Critical);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_forwards_severity_and_resource_type_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                FINDINGS_URL,
                r#"{"filterCriteria":{"severity":[{"comparison":"EQUALS","value":"HIGH"}],"resourceType":[{"comparison":"EQUALS","value":"AWS_EC2_INSTANCE"}]}}"#,
            ),
            json_response(200, r#"{"findings":[{"findingArn":"arn1"}]}"#),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let (findings, _token) = client
            .list_findings(
                Some("HIGH".to_string()),
                Some("AWS_EC2_INSTANCE".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(findings.len(), 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FINDINGS_URL, r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"findings":[{"findingArn":"arn3"}]}"#),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client
            .list_findings(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FINDINGS_URL, r#"{"maxResults":2}"#),
            json_response(
                200,
                r#"{"findings":[{"findingArn":"arn1"},{"findingArn":"arn2"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client
            .list_findings(None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(findings.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(FINDINGS_URL, r#"{"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"findings":[{"findingArn":"arn1"},{"findingArn":"arn2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(FINDINGS_URL, r#"{"nextToken":"p2","maxResults":8}"#),
                json_response(200, r#"{"findings":[{"findingArn":"arn3"}]}"#),
            ),
        ]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let (findings, token) = client
            .list_findings(None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(findings.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FINDINGS_URL, "{}"),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_findings(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_coverage_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(COVERAGE_URL, "{}"),
            json_response(
                200,
                r#"{"coveredResources":[{"resourceType":"AWS_EC2_INSTANCE","resourceId":"i-0abc","accountId":"111122223333","scanType":"NETWORK"}]}"#,
            ),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client.list_coverage(None, None).await.unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].resource_id(), "i-0abc");
        assert_eq!(resources[0].account_id(), "111122223333");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_coverage_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(COVERAGE_URL, r#"{"nextToken":"cursor-b"}"#),
            json_response(200, r#"{"coveredResources":[{"resourceId":"i-0def"}]}"#),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client
            .list_coverage(None, Some("cursor-b".to_string()))
            .await
            .unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_coverage_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(COVERAGE_URL, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"coveredResources":[{"resourceId":"i-0abc"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client.list_coverage(Some(1), None).await.unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_coverage_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(COVERAGE_URL, "{}"),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = InspectorClient::new(&sdk_config(http_client.clone()));

        let err = client.list_coverage(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
