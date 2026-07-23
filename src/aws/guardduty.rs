use aws_config::SdkConfig;
use aws_sdk_guardduty::types::FindingCriteria;

use crate::error::VaporError;

pub struct GuardDutyClient {
    inner: aws_sdk_guardduty::Client,
}

impl GuardDutyClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_guardduty::Client::new(config),
        }
    }

    /// Lists detector IDs, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS
    /// via `ListDetectorsInput::max_results` so a capped page boundary
    /// lands exactly on the returned token.
    pub async fn list_detectors(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_detectors();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.detector_ids.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn get_detector(
        &self,
        detector_id: &str,
    ) -> Result<aws_sdk_guardduty::operation::get_detector::GetDetectorOutput, VaporError> {
        self.inner
            .get_detector()
            .detector_id(detector_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Lists finding IDs matching `criteria`, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`. `limit`
    /// is handed to AWS via `ListFindingsInput::max_results` so a capped
    /// page boundary lands exactly on the returned token.
    pub async fn list_findings(
        &self,
        detector_id: &str,
        criteria: Option<FindingCriteria>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_findings().detector_id(detector_id);
            if let Some(ref c) = criteria {
                req = req.finding_criteria(c.clone());
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.finding_ids.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn get_findings(
        &self,
        detector_id: &str,
        finding_ids: Vec<String>,
    ) -> Result<Vec<aws_sdk_guardduty::types::Finding>, VaporError> {
        let output = self
            .inner
            .get_findings()
            .detector_id(detector_id)
            .set_finding_ids(Some(finding_ids))
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.findings().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use aws_sdk_guardduty::types::{Condition, DetectorStatus, FindingPublishingFrequency};

    const BASE: &str = "https://guardduty.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_detectors_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector"), ""),
            json_response(200, r#"{"detectorIds":["d1","d2"]}"#),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_detectors(None, None).await.unwrap();

        assert_eq!(items, vec!["d1".to_string(), "d2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_detectors_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector?nextToken=cursor-a"), ""),
            json_response(200, r#"{"detectorIds":["d3"]}"#),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_detectors(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items, vec!["d3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_detectors_stops_at_limit_and_returns_resume_token() {
        // ListDetectors forwards `maxResults` straight to AWS with no
        // client-side truncation, so the canned response must return
        // exactly the requested count, not more (durable gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector?maxResults=2"), ""),
            json_response(200, r#"{"detectorIds":["d1","d2"],"nextToken":"page2"}"#),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_detectors(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_detectors_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/detector?maxResults=10"), ""),
                json_response(200, r#"{"detectorIds":["d1","d2"],"nextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/detector?maxResults=8&nextToken=p2"), ""),
                json_response(200, r#"{"detectorIds":["d3"]}"#),
            ),
        ]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_detectors(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_detectors_propagates_errors() {
        // `BadRequestException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector"), ""),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let err = client.list_detectors(None, None).await.unwrap_err();

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
    async fn get_detector_returns_detector_details() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector/d1"), ""),
            json_response(
                200,
                r#"{"createdAt":"2024-01-01T00:00:00Z","findingPublishingFrequency":"SIX_HOURS","serviceRole":"arn:aws:iam::123456789012:role/svc","status":"ENABLED","updatedAt":"2024-02-01T00:00:00Z"}"#,
            ),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let output = client.get_detector("d1").await.unwrap();

        assert_eq!(output.created_at(), Some("2024-01-01T00:00:00Z"));
        assert_eq!(
            output.finding_publishing_frequency(),
            Some(&FindingPublishingFrequency::SixHours)
        );
        assert_eq!(
            output.service_role(),
            Some("arn:aws:iam::123456789012:role/svc")
        );
        assert_eq!(output.status(), Some(&DetectorStatus::Enabled));
        assert_eq!(output.updated_at(), Some("2024-02-01T00:00:00Z"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_detector_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector/missing"), ""),
            json_error_response("BadRequestException", "detector not found"),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let err = client.get_detector("missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "detector not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_lists_all_when_no_limit_or_criteria() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector/d1/findings"), "{}"),
            json_response(200, r#"{"findingIds":["f1","f2"]}"#),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_findings("d1", None, None, None).await.unwrap();

        assert_eq!(items, vec!["f1".to_string(), "f2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_forwards_finding_criteria() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/detector/d1/findings"),
                r#"{"findingCriteria":{"criterion":{"type":{"equals":["Trojan"]}}}}"#,
            ),
            json_response(200, r#"{"findingIds":["f1"]}"#),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let criteria = FindingCriteria::builder()
            .criterion(
                "type",
                Condition::builder().equals("Trojan".to_string()).build(),
            )
            .build();

        let (items, _token) = client
            .list_findings("d1", Some(criteria), None, None)
            .await
            .unwrap();

        assert_eq!(items, vec!["f1".to_string()]);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/detector/d1/findings"),
                r#"{"nextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"findingIds":["f3"]}"#),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_findings("d1", None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items, vec!["f3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_stops_at_limit_and_returns_resume_token() {
        // ListFindings forwards `maxResults` straight to AWS with no
        // client-side truncation, so the canned response must return
        // exactly the requested count, not more (durable gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/detector/d1/findings"),
                r#"{"maxResults":2}"#,
            ),
            json_response(200, r#"{"findingIds":["f1","f2"],"nextToken":"page2"}"#),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_findings("d1", None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings"),
                    r#"{"maxResults":10}"#,
                ),
                json_response(200, r#"{"findingIds":["f1","f2"],"nextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings"),
                    r#"{"nextToken":"p2","maxResults":8}"#,
                ),
                json_response(200, r#"{"findingIds":["f3"]}"#),
            ),
        ]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_findings("d1", None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_findings_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector/d1/findings"), "{}"),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_findings("d1", None, None, None)
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
    async fn get_findings_returns_findings() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/detector/d1/findings/get"),
                r#"{"findingIds":["f1","f2"]}"#,
            ),
            json_response(
                200,
                r#"{"findings":[{"accountId":"111111111111","id":"f1","type":"Trojan:EC2/DNSDataExfiltration","severity":8.5,"title":"first"},{"accountId":"111111111111","id":"f2","type":"Recon:EC2/PortProbeUnprotectedPort"}]}"#,
            ),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let findings = client
            .get_findings("d1", vec!["f1".to_string(), "f2".to_string()])
            .await
            .unwrap();

        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].id(), Some("f1"));
        assert_eq!(
            findings[0].r#type(),
            Some("Trojan:EC2/DNSDataExfiltration")
        );
        assert_eq!(findings[0].severity(), Some(8.5));
        assert_eq!(findings[0].title(), Some("first"));
        assert_eq!(findings[1].id(), Some("f2"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_findings_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/detector/d1/findings/get"),
                r#"{"findingIds":["f1"]}"#,
            ),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = GuardDutyClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_findings("d1", vec!["f1".to_string()])
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
}

