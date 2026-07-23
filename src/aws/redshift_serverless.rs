use aws_config::SdkConfig;
use aws_sdk_redshiftserverless::types::{Namespace, Workgroup};

use crate::error::VaporError;

pub struct RedshiftServerlessClient {
    inner: aws_sdk_redshiftserverless::Client,
}

impl RedshiftServerlessClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_redshiftserverless::Client::new(config),
        }
    }

    /// Lists namespaces, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListNamespaces` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-redshiftserverless` 1.110.0's
    /// `operation/list_namespaces/_list_namespaces_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_namespaces(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Namespace>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_namespaces();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.namespaces);
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists workgroups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListWorkgroups` has both
    /// `max_results` and `next_token`, same shape as `list_namespaces` above.
    pub async fn list_workgroups(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Workgroup>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_workgroups();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.workgroups);
            token = output.next_token;

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

    // Crate name (`aws-sdk-redshiftserverless`) doesn't match the endpoint
    // hostname (`redshift-serverless.*`) — verified against pinned
    // `aws-sdk-redshiftserverless` 1.110.0's `config/endpoint.rs` (memory
    // gotcha 3). Both `ListNamespaces`/`ListWorkgroups` POST JSON to a fixed
    // `/` path (awsJson1.1, differentiated only by the `x-amz-target`
    // header, which `test_util::request` doesn't compare).
    const BASE: &str = "https://redshift-serverless.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_namespaces_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"namespaces":[{"namespaceArn":"arn:aws:redshift-serverless:us-east-1:111111111111:namespace/ns-1","namespaceName":"ns-1"}]}"#,
            ),
        )]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let (namespaces, token) = client.list_namespaces(None, None).await.unwrap();

        assert_eq!(namespaces.len(), 1);
        assert_eq!(
            namespaces[0].namespace_arn(),
            Some("arn:aws:redshift-serverless:us-east-1:111111111111:namespace/ns-1")
        );
        assert_eq!(namespaces[0].namespace_name(), Some("ns-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_namespaces_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"namespaces":[]}"#),
        )]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let (namespaces, token) = client
            .list_namespaces(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(namespaces.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_namespaces_stops_at_limit_and_returns_resume_token() {
        // `list_namespaces` forwards the remaining budget straight to
        // `maxResults` with no client-side truncation (memory gotcha 13),
        // so the canned page must return exactly `limit` items.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"namespaces":[{"namespaceId":"ns-1"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let (namespaces, token) = client.list_namespaces(Some(1), None).await.unwrap();

        assert_eq!(namespaces.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_namespaces_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"namespaces":[{"namespaceId":"ns-1"},{"namespaceId":"ns-2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"nextToken":"p2","maxResults":8}"#),
                json_response(200, r#"{"namespaces":[{"namespaceId":"ns-3"}]}"#),
            ),
        ]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let (namespaces, token) = client.list_namespaces(Some(10), None).await.unwrap();

        assert_eq!(namespaces.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_namespaces_propagates_errors() {
        // `Namespace` doesn't derive `Debug` (only `Clone`/`PartialEq`,
        // verified against the pinned SDK's `types/_namespace.rs` — memory
        // gotcha 6), so the `Result` is matched directly rather than via
        // `.unwrap_err()`. `InvalidParameterException`, not a
        // throttling-classified code (memory gotcha 1).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidParameterException", "bad request"),
        )]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        match client.list_namespaces(None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_workgroups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"workgroups":[{"workgroupArn":"arn:aws:redshift-serverless:us-east-1:111111111111:workgroup/wg-1","workgroupName":"wg-1"}]}"#,
            ),
        )]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let (workgroups, token) = client.list_workgroups(None, None).await.unwrap();

        assert_eq!(workgroups.len(), 1);
        assert_eq!(
            workgroups[0].workgroup_arn(),
            Some("arn:aws:redshift-serverless:us-east-1:111111111111:workgroup/wg-1")
        );
        assert_eq!(workgroups[0].workgroup_name(), Some("wg-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_workgroups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"workgroups":[]}"#),
        )]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let (workgroups, token) = client
            .list_workgroups(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(workgroups.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_workgroups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"workgroups":[{"workgroupId":"wg-1"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let (workgroups, token) = client.list_workgroups(Some(1), None).await.unwrap();

        assert_eq!(workgroups.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_workgroups_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"workgroups":[{"workgroupId":"wg-1"},{"workgroupId":"wg-2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"nextToken":"p2","maxResults":8}"#),
                json_response(200, r#"{"workgroups":[{"workgroupId":"wg-3"}]}"#),
            ),
        ]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let (workgroups, token) = client.list_workgroups(Some(10), None).await.unwrap();

        assert_eq!(workgroups.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_workgroups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidParameterException", "bad request"),
        )]);
        let client = RedshiftServerlessClient::new(&sdk_config(http_client.clone()));

        let err = client.list_workgroups(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

