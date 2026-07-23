use aws_config::SdkConfig;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;

use crate::error::VaporError;

pub struct AuditManagerClient {
    inner: aws_sdk_auditmanager::Client,
}

impl AuditManagerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_auditmanager::Client::new(config),
        }
    }

    /// Lists assessments, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_assessments(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_auditmanager::types::AssessmentMetadataItem>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;
        loop {
            let mut req = self.inner.list_assessments();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }
            let out = match req.send().await {
                Ok(out) => out,
                Err(e) => {
                    if e.code() == Some("AccessDeniedException") {
                        return Ok((vec![], None));
                    }
                    return Err(crate::error::sdk_err(e));
                }
            };
            items.extend(out.assessment_metadata.unwrap_or_default());
            token = out.next_token;
            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((items, token))
    }

    /// Lists frameworks, optionally capped at `limit` results and resumed via `next_token`.
    ///
    /// `framework_type` filters to a single AWS-side type ("Standard"/"Custom"), in which
    /// case pagination is a genuine single-stream resumable token. With no filter, both
    /// types are merged client-side (AWS has no combined-filter list call, so there's no
    /// single AWS token spanning both streams); the merge always starts fresh and returns
    /// `next_token: None` (documented caveat, cost_explorer precedent) — `limit` still caps
    /// the merged result, just without resumability.
    pub async fn list_frameworks(
        &self,
        framework_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_auditmanager::types::AssessmentFrameworkMetadata>,
            Option<String>,
        ),
        VaporError,
    > {
        match framework_type.as_deref() {
            Some("Standard") => {
                self.list_frameworks_by_type(
                    aws_sdk_auditmanager::types::FrameworkType::Standard,
                    limit,
                    next_token,
                )
                .await
            }
            Some("Custom") => {
                self.list_frameworks_by_type(
                    aws_sdk_auditmanager::types::FrameworkType::Custom,
                    limit,
                    next_token,
                )
                .await
            }
            _ => {
                let mut all_items = Vec::new();
                for ftype in [
                    aws_sdk_auditmanager::types::FrameworkType::Standard,
                    aws_sdk_auditmanager::types::FrameworkType::Custom,
                ] {
                    let remaining = limit.map(|l| (l - all_items.len() as i32).max(0));
                    if remaining == Some(0) {
                        break;
                    }
                    let (items, _) = self.list_frameworks_by_type(ftype, remaining, None).await?;
                    all_items.extend(items);
                }
                Ok((all_items, None))
            }
        }
    }

    async fn list_frameworks_by_type(
        &self,
        framework_type: aws_sdk_auditmanager::types::FrameworkType,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_auditmanager::types::AssessmentFrameworkMetadata>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;
        loop {
            let mut req = self
                .inner
                .list_assessment_frameworks()
                .framework_type(framework_type.clone());
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }
            let out = match req.send().await {
                Ok(out) => out,
                Err(e) => {
                    if e.code() == Some("AccessDeniedException") {
                        return Ok((vec![], None));
                    }
                    return Err(crate::error::sdk_err(e));
                }
            };
            items.extend(out.framework_metadata_list.unwrap_or_default());
            token = out.next_token;
            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((items, token))
    }

    /// Lists controls, optionally capped at `limit` results and resumed via `next_token`.
    ///
    /// Same merge caveat as `list_frameworks`: a `control_type` filter gives a genuine
    /// resumable single-stream token; no filter merges both types client-side and always
    /// returns `next_token: None`.
    pub async fn list_controls(
        &self,
        control_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_auditmanager::types::ControlMetadata>,
            Option<String>,
        ),
        VaporError,
    > {
        match control_type.as_deref() {
            Some("Standard") => {
                self.list_controls_by_type(
                    aws_sdk_auditmanager::types::ControlType::Standard,
                    limit,
                    next_token,
                )
                .await
            }
            Some("Custom") => {
                self.list_controls_by_type(
                    aws_sdk_auditmanager::types::ControlType::Custom,
                    limit,
                    next_token,
                )
                .await
            }
            _ => {
                let mut all_items = Vec::new();
                for ctype in [
                    aws_sdk_auditmanager::types::ControlType::Standard,
                    aws_sdk_auditmanager::types::ControlType::Custom,
                ] {
                    let remaining = limit.map(|l| (l - all_items.len() as i32).max(0));
                    if remaining == Some(0) {
                        break;
                    }
                    let (items, _) = self.list_controls_by_type(ctype, remaining, None).await?;
                    all_items.extend(items);
                }
                Ok((all_items, None))
            }
        }
    }

    async fn list_controls_by_type(
        &self,
        control_type: aws_sdk_auditmanager::types::ControlType,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_auditmanager::types::ControlMetadata>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;
        loop {
            let mut req = self.inner.list_controls().control_type(control_type.clone());
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }
            let out = match req.send().await {
                Ok(out) => out,
                Err(e) => {
                    if e.code() == Some("AccessDeniedException") {
                        return Ok((vec![], None));
                    }
                    return Err(crate::error::sdk_err(e));
                }
            };
            items.extend(out.control_metadata_list.unwrap_or_default());
            token = out.next_token;
            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
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

    const ASSESSMENTS: &str = "https://auditmanager.us-east-1.amazonaws.com/assessments";
    const FRAMEWORKS: &str = "https://auditmanager.us-east-1.amazonaws.com/assessmentFrameworks";
    const CONTROLS: &str = "https://auditmanager.us-east-1.amazonaws.com/controls";

    #[tokio::test]
    async fn list_assessments_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ASSESSMENTS, ""),
            json_response(
                200,
                r#"{"assessmentMetadata":[{"id":"a1","name":"Assessment One"},{"id":"a2","name":"Assessment Two"}]}"#,
            ),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_assessments(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id(), Some("a1"));
        assert_eq!(items[1].id(), Some("a2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_assessments_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{ASSESSMENTS}?nextToken=cursor-a"), ""),
            json_response(200, r#"{"assessmentMetadata":[{"id":"a3","name":"Assessment Three"}]}"#),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_assessments(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_assessments_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{ASSESSMENTS}?maxResults=2"), ""),
            json_response(
                200,
                r#"{"assessmentMetadata":[{"id":"a1"},{"id":"a2"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_assessments(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_assessments_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{ASSESSMENTS}?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"assessmentMetadata":[{"id":"a1"},{"id":"a2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{ASSESSMENTS}?nextToken=p2&maxResults=8"), ""),
                json_response(200, r#"{"assessmentMetadata":[{"id":"a3"}]}"#),
            ),
        ]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_assessments(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_assessments_propagates_non_access_denied_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ASSESSMENTS, ""),
            json_error_response("ValidationException", "invalid input"),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_assessments(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "invalid input");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_assessments_swallows_access_denied_and_returns_empty() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ASSESSMENTS, ""),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_assessments(None, None).await.unwrap();

        assert!(items.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_frameworks_with_standard_type_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FRAMEWORKS}?frameworkType=Standard&maxResults=2"), ""),
            json_response(
                200,
                r#"{"frameworkMetadataList":[{"id":"f1","name":"Framework One","type":"Standard"},{"id":"f2","name":"Framework Two","type":"Standard"}],"nextToken":"fpage2"}"#,
            ),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_frameworks(Some("Standard".to_string()), Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id(), Some("f1"));
        assert_eq!(token, Some("fpage2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_frameworks_with_custom_type_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FRAMEWORKS}?frameworkType=Custom&nextToken=cursor-b"), ""),
            json_response(
                200,
                r#"{"frameworkMetadataList":[{"id":"f3","name":"Custom Framework","type":"Custom"}]}"#,
            ),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_frameworks(Some("Custom".to_string()), None, Some("cursor-b".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_frameworks_merges_standard_and_custom_when_type_not_specified() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{FRAMEWORKS}?frameworkType=Standard"), ""),
                json_response(
                    200,
                    r#"{"frameworkMetadataList":[{"id":"f1","name":"Std1","type":"Standard"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{FRAMEWORKS}?frameworkType=Custom"), ""),
                json_response(
                    200,
                    r#"{"frameworkMetadataList":[{"id":"f2","name":"Cus1","type":"Custom"}]}"#,
                ),
            ),
        ]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_frameworks(None, None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id(), Some("f1"));
        assert_eq!(items[1].id(), Some("f2"));
        // Merge mode always returns `next_token: None`, even though a genuine
        // single-type call could carry one — there's no AWS token spanning
        // both merged streams.
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_frameworks_by_type_swallows_access_denied_and_returns_empty() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FRAMEWORKS}?frameworkType=Standard"), ""),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_frameworks(Some("Standard".to_string()), None, None)
            .await
            .unwrap();

        assert!(items.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_controls_with_standard_type_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{CONTROLS}?controlType=Standard&maxResults=2"), ""),
            json_response(
                200,
                r#"{"controlMetadataList":[{"id":"c1","name":"Control One"},{"id":"c2","name":"Control Two"}],"nextToken":"cpage2"}"#,
            ),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_controls(Some("Standard".to_string()), Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id(), Some("c1"));
        assert_eq!(token, Some("cpage2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_controls_merges_standard_and_custom_and_splits_remaining_limit() {
        // File-specific logic: the merge loop computes `remaining = (limit -
        // all_items.len()).max(0)` before each type's call, so the second
        // (Custom) request's `maxResults` reflects what the first (Standard)
        // request already consumed, not the original `limit`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{CONTROLS}?controlType=Standard&maxResults=3"), ""),
                json_response(200, r#"{"controlMetadataList":[{"id":"c1"},{"id":"c2"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{CONTROLS}?controlType=Custom&maxResults=1"), ""),
                json_response(200, r#"{"controlMetadataList":[{"id":"c3"}]}"#),
            ),
        ]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_controls(None, Some(3), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[2].id(), Some("c3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_controls_by_type_propagates_non_access_denied_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{CONTROLS}?controlType=Standard"), ""),
            json_error_response("ValidationException", "invalid input"),
        )]);
        let client = AuditManagerClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_controls(Some("Standard".to_string()), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "invalid input");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
