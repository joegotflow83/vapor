use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct CognitoClient {
    inner: aws_sdk_cognitoidentityprovider::Client,
}

impl CognitoClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cognitoidentityprovider::Client::new(config),
        }
    }

    /// Lists Cognito user pools, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListUserPools` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-cognitoidentityprovider` 1.122.0's
    /// `operation/list_user_pools/_list_user_pools_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_user_pools(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_cognitoidentityprovider::types::UserPoolDescriptionType>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut pools = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_user_pools();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - pools.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            pools.extend(output.user_pools.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if pools.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((pools, token))
    }

    pub async fn describe_user_pool(
        &self,
        user_pool_id: &str,
    ) -> Result<aws_sdk_cognitoidentityprovider::types::UserPoolType, VaporError> {
        let output = self
            .inner
            .describe_user_pool()
            .user_pool_id(user_pool_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        output.user_pool.ok_or_else(|| VaporError::AwsSdk {
            code: None,
            message: "No user pool returned".to_string(),
        })
    }

    /// Lists app clients for a user pool, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `ListUserPoolClients` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-cognitoidentityprovider` 1.122.0's
    /// `operation/list_user_pool_clients/_list_user_pool_clients_input.rs`),
    /// same kinesis/mq loop shape as `list_user_pools`.
    pub async fn list_user_pool_clients(
        &self,
        user_pool_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_cognitoidentityprovider::types::UserPoolClientDescription>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut clients = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_user_pool_clients()
                .user_pool_id(user_pool_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - clients.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            clients.extend(output.user_pool_clients.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if clients.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((clients, token))
    }

    pub async fn describe_user_pool_client(
        &self,
        user_pool_id: &str,
        client_id: &str,
    ) -> Result<aws_sdk_cognitoidentityprovider::types::UserPoolClientType, VaporError> {
        let output = self
            .inner
            .describe_user_pool_client()
            .user_pool_id(user_pool_id)
            .client_id(client_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        output.user_pool_client.ok_or_else(|| VaporError::AwsSdk {
            code: None,
            message: "No user pool client returned".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://cognito-idp.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_user_pools_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"UserPools":[{"Id":"pool-1","Name":"Pool One"},{"Id":"pool-2","Name":"Pool Two"}]}"#,
            ),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let (pools, token) = client.list_user_pools(None, None).await.unwrap();

        assert_eq!(pools.len(), 2);
        assert_eq!(pools[0].id(), Some("pool-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pools_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(
                200,
                r#"{"UserPools":[{"Id":"pool-3","Name":"Pool Three"}]}"#,
            ),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let (pools, token) = client
            .list_user_pools(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0].id(), Some("pool-3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pools_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"UserPools":[{"Id":"pool-1"},{"Id":"pool-2"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let (pools, token) = client.list_user_pools(Some(2), None).await.unwrap();

        assert_eq!(pools.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pools_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(200, r#"{"UserPools":[{"Id":"pool-1"}],"NextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":9}"#),
                json_response(200, r#"{"UserPools":[{"Id":"pool-2"}]}"#),
            ),
        ]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let (pools, token) = client.list_user_pools(Some(10), None).await.unwrap();

        assert_eq!(pools.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pools_propagates_errors() {
        // Uses `InvalidParameterException` rather than the more-obvious
        // `TooManyRequestsException` — the latter is on the AWS SDK's
        // built-in throttling-retry list, so the retry strategy consumes a
        // *second* replay event that a single-event `StaticReplayClient`
        // doesn't have, surfacing as `SdkError::DispatchFailure` (code
        // `None`) instead of exercising this file's error-mapping path
        // (same gotcha as apigateway.rs's `list_rest_apis_propagates_errors`).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidParameterException", "bad request"),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let err = client.list_user_pools(None, None).await.unwrap_err();

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
    async fn describe_user_pool_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
            json_response(200, r#"{"UserPool":{"Id":"pool-1","Name":"Pool One"}}"#),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let pool = client.describe_user_pool("pool-1").await.unwrap();

        assert_eq!(pool.id(), Some("pool-1"));
        assert_eq!(pool.name(), Some("Pool One"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_user_pool_errors_when_field_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
            json_response(200, "{}"),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_user_pool("pool-1").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, None);
                assert_eq!(message, "No user pool returned");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_user_pool_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
            json_error_response("ResourceNotFoundException", "user pool not found"),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_user_pool("pool-1").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "user pool not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pool_clients_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
            json_response(
                200,
                r#"{"UserPoolClients":[{"UserPoolId":"pool-1","ClientId":"client-1","ClientName":"App One"}]}"#,
            ),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let (clients, token) = client
            .list_user_pool_clients("pool-1", None, None)
            .await
            .unwrap();

        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].client_id(), Some("client-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pool_clients_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"UserPoolId":"pool-1","NextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"UserPoolClients":[{"ClientId":"client-2"}]}"#),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let (clients, token) = client
            .list_user_pool_clients("pool-1", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].client_id(), Some("client-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pool_clients_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1","MaxResults":1}"#),
            json_response(
                200,
                r#"{"UserPoolClients":[{"ClientId":"client-1"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let (clients, token) = client
            .list_user_pool_clients("pool-1", Some(1), None)
            .await
            .unwrap();

        assert_eq!(clients.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pool_clients_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"UserPoolId":"pool-1","MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"UserPoolClients":[{"ClientId":"client-1"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"UserPoolId":"pool-1","NextToken":"p2","MaxResults":9}"#,
                ),
                json_response(200, r#"{"UserPoolClients":[{"ClientId":"client-2"}]}"#),
            ),
        ]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let (clients, token) = client
            .list_user_pool_clients("pool-1", Some(10), None)
            .await
            .unwrap();

        assert_eq!(clients.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_user_pool_clients_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
            json_error_response("ResourceNotFoundException", "user pool not found"),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_user_pool_clients("pool-1", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "user pool not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_user_pool_client_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1","ClientId":"client-1"}"#),
            json_response(
                200,
                r#"{"UserPoolClient":{"UserPoolId":"pool-1","ClientId":"client-1","ClientName":"App One"}}"#,
            ),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let app_client = client
            .describe_user_pool_client("pool-1", "client-1")
            .await
            .unwrap();

        assert_eq!(app_client.client_id(), Some("client-1"));
        assert_eq!(app_client.client_name(), Some("App One"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_user_pool_client_errors_when_field_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1","ClientId":"client-1"}"#),
            json_response(200, "{}"),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_user_pool_client("pool-1", "client-1")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, None);
                assert_eq!(message, "No user pool client returned");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_user_pool_client_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"UserPoolId":"pool-1","ClientId":"client-1"}"#),
            json_error_response("ResourceNotFoundException", "app client not found"),
        )]);
        let client = CognitoClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_user_pool_client("pool-1", "client-1")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "app client not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
