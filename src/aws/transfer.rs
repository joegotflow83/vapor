use aws_config::SdkConfig;
use aws_sdk_transfer::types::{ListedServer, ListedUser};

use crate::error::VaporError;

pub struct TransferClient {
    inner: aws_sdk_transfer::Client,
}

impl TransferClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_transfer::Client::new(config),
        }
    }

    /// Lists servers, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `limit` is handed to AWS via
    /// `ListServersInput::max_results` so a capped page boundary lands exactly
    /// on the returned token.
    pub async fn list_servers(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ListedServer>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_servers();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.servers);
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists users for a server, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `ListUsersInput::max_results` so a capped page boundary lands exactly
    /// on the returned token.
    pub async fn list_users(
        &self,
        server_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ListedUser>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_users().server_id(server_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.users);
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
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const BASE: &str = "https://transfer.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_servers_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"Servers":[{"Arn":"arn:s1","Domain":"S3","IdentityProviderType":"SERVICE_MANAGED","EndpointType":"PUBLIC","LoggingRole":"arn:role1","ServerId":"s1","State":"ONLINE","UserCount":3},{"Arn":"arn:s2","ServerId":"s2"}]}"#,
            ),
        )]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_servers(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let s1 = &items[0];
        assert_eq!(s1.arn(), "arn:s1");
        assert_eq!(s1.domain().map(|d| d.as_str()), Some("S3"));
        assert_eq!(
            s1.identity_provider_type().map(|t| t.as_str()),
            Some("SERVICE_MANAGED")
        );
        assert_eq!(s1.endpoint_type().map(|t| t.as_str()), Some("PUBLIC"));
        assert_eq!(s1.logging_role(), Some("arn:role1"));
        assert_eq!(s1.server_id(), Some("s1"));
        assert_eq!(s1.state().map(|s| s.as_str()), Some("ONLINE"));
        assert_eq!(s1.user_count(), Some(3));

        let s2 = &items[1];
        assert_eq!(s2.arn(), "arn:s2");
        assert_eq!(s2.server_id(), Some("s2"));
        assert_eq!(s2.domain(), None);
        assert_eq!(s2.state(), None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_servers_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Servers":[{"Arn":"arn:s3","ServerId":"s3"}]}"#),
        )]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_servers(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_servers_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Servers":[{"Arn":"arn:s1","ServerId":"s1"},{"Arn":"arn:s2","ServerId":"s2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_servers(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_servers_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Servers":[{"Arn":"arn:s1","ServerId":"s1"},{"Arn":"arn:s2","ServerId":"s2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":8,"NextToken":"p2"}"#),
                json_response(200, r#"{"Servers":[{"Arn":"arn:s3","ServerId":"s3"}]}"#),
            ),
        ]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_servers(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_servers_propagates_errors() {
        // `InvalidRequestException`, not a throttling-classified code (see
        // memory gotcha 1: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InvalidRequestException", "malformed request"),
        )]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let err = client.list_servers(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "malformed request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"ServerId":"s-123"}"#),
            json_response(
                200,
                r#"{"ServerId":"s-123","Users":[{"Arn":"arn:u1","HomeDirectory":"/bucket/home","HomeDirectoryType":"PATH","Role":"arn:role1","SshPublicKeyCount":2,"UserName":"alice"},{"Arn":"arn:u2","UserName":"bob"}]}"#,
            ),
        )]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_users("s-123", None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let u1 = &items[0];
        assert_eq!(u1.arn(), "arn:u1");
        assert_eq!(u1.home_directory(), Some("/bucket/home"));
        assert_eq!(u1.home_directory_type().map(|t| t.as_str()), Some("PATH"));
        assert_eq!(u1.role(), Some("arn:role1"));
        assert_eq!(u1.ssh_public_key_count(), Some(2));
        assert_eq!(u1.user_name(), Some("alice"));

        let u2 = &items[1];
        assert_eq!(u2.arn(), "arn:u2");
        assert_eq!(u2.user_name(), Some("bob"));
        assert_eq!(u2.home_directory(), None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a","ServerId":"s-123"}"#),
            json_response(
                200,
                r#"{"ServerId":"s-123","Users":[{"Arn":"arn:u3","UserName":"carol"}]}"#,
            ),
        )]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_users("s-123", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":2,"ServerId":"s-123"}"#),
            json_response(
                200,
                r#"{"ServerId":"s-123","Users":[{"Arn":"arn:u1","UserName":"alice"},{"Arn":"arn:u2","UserName":"bob"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_users("s-123", Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10,"ServerId":"s-123"}"#),
                json_response(
                    200,
                    r#"{"ServerId":"s-123","Users":[{"Arn":"arn:u1","UserName":"alice"},{"Arn":"arn:u2","UserName":"bob"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":8,"NextToken":"p2","ServerId":"s-123"}"#),
                json_response(
                    200,
                    r#"{"ServerId":"s-123","Users":[{"Arn":"arn:u3","UserName":"carol"}]}"#,
                ),
            ),
        ]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_users("s-123", Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"ServerId":"s-123"}"#),
            json_error_response("InvalidRequestException", "malformed request"),
        )]);
        let client = TransferClient::new(&sdk_config(http_client.clone()));

        let err = client.list_users("s-123", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "malformed request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
