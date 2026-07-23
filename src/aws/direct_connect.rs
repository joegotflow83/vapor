use aws_config::SdkConfig;
use aws_sdk_directconnect::types::{Connection, VirtualInterface};

use crate::error::VaporError;

pub struct DirectConnectClient {
    inner: aws_sdk_directconnect::Client,
}

impl DirectConnectClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_directconnect::Client::new(config),
        }
    }

    /// Lists connections, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeConnections` has
    /// both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-directconnect` 1.102.0's
    /// `operation/describe_connections/_describe_connections_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself,
    /// matching `kinesis.rs`'s `list_streams` pattern.
    pub async fn describe_connections(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Connection>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_connections();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.connections().to_vec());
            token = output.next_token().map(|s| s.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists virtual interfaces (optionally filtered by `connection_id`),
    /// capped at `limit` results (default unlimited) and resumed from
    /// `next_token`. `DescribeVirtualInterfaces` has both `max_results` and
    /// `next_token` (verified against pinned `aws-sdk-directconnect`
    /// 1.102.0's
    /// `operation/describe_virtual_interfaces/_describe_virtual_interfaces_input.rs`),
    /// same capping pattern as `describe_connections` above.
    pub async fn describe_virtual_interfaces(
        &self,
        connection_id: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<VirtualInterface>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_virtual_interfaces();
            if let Some(id) = connection_id {
                req = req.connection_id(id);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.virtual_interfaces().to_vec());
            token = output.next_token().map(|s| s.to_string());

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
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://directconnect.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_connections_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"connections":[{"connectionId":"dxcon-1"},{"connectionId":"dxcon-2"}]}"#,
            ),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let (connections, token) = client.describe_connections(None, None).await.unwrap();

        assert_eq!(connections.len(), 2);
        assert_eq!(connections[0].connection_id(), Some("dxcon-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_connections_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"connections":[{"connectionId":"dxcon-3"}]}"#),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let (connections, token) = client
            .describe_connections(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].connection_id(), Some("dxcon-3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_connections_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"maxResults":2}"#),
            json_response(
                200,
                r#"{"connections":[{"connectionId":"dxcon-1"},{"connectionId":"dxcon-2"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let (connections, token) = client.describe_connections(Some(2), None).await.unwrap();

        assert_eq!(connections.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_connections_exhausts_all_pages_until_no_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"connections":[{"connectionId":"dxcon-1"}],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"page2"}"#),
                json_response(200, r#"{"connections":[{"connectionId":"dxcon-2"}]}"#),
            ),
        ]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let (connections, token) = client.describe_connections(None, None).await.unwrap();

        assert_eq!(connections.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_connections_propagates_service_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("DirectConnectClientException", "invalid connection id"),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_connections(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DirectConnectClientException".to_string()));
                assert_eq!(message, "invalid connection id");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_virtual_interfaces_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"virtualInterfaces":[{"virtualInterfaceId":"dxvif-1"},{"virtualInterfaceId":"dxvif-2"}]}"#,
            ),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let (vifs, token) = client.describe_virtual_interfaces(None, None, None).await.unwrap();

        assert_eq!(vifs.len(), 2);
        assert_eq!(vifs[0].virtual_interface_id(), Some("dxvif-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_virtual_interfaces_filters_by_connection_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"connectionId":"dxcon-1"}"#),
            json_response(200, r#"{"virtualInterfaces":[{"virtualInterfaceId":"dxvif-1","connectionId":"dxcon-1"}]}"#),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let (vifs, token) = client
            .describe_virtual_interfaces(Some("dxcon-1"), None, None)
            .await
            .unwrap();

        assert_eq!(vifs.len(), 1);
        assert_eq!(vifs[0].connection_id(), Some("dxcon-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_virtual_interfaces_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"virtualInterfaces":[{"virtualInterfaceId":"dxvif-1"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let (vifs, token) = client
            .describe_virtual_interfaces(None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(vifs.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_virtual_interfaces_propagates_server_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("DirectConnectServerException", "internal error"),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_virtual_interfaces(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DirectConnectServerException".to_string()));
                assert_eq!(message, "internal error");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
