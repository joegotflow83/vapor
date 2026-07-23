use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct SecretsManagerClient {
    inner: aws_sdk_secretsmanager::Client,
}

impl SecretsManagerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_secretsmanager::Client::new(config),
        }
    }

    /// List all secrets, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListSecrets` has both
    /// `max_results` (i32) and `next_token` (verified against pinned
    /// `aws-sdk-secretsmanager` 1.108.0's
    /// `operation/list_secrets/_list_secrets_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_secrets(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_secretsmanager::types::SecretListEntry>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_secrets();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.secret_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Describe a single secret by ID or ARN.
    pub async fn describe_secret(
        &self,
        secret_id: &str,
    ) -> Result<aws_sdk_secretsmanager::operation::describe_secret::DescribeSecretOutput, VaporError>
    {
        self.inner
            .describe_secret()
            .secret_id(secret_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Get the value of a secret by ID or ARN.
    pub async fn get_secret_value(
        &self,
        secret_id: &str,
    ) -> Result<
        aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueOutput,
        VaporError,
    > {
        self.inner
            .get_secret_value()
            .secret_id(secret_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Get the resource-based policy document attached to a secret.
    /// Returns None when no resource policy is attached (empty body response).
    /// Reveals cross-account access grants — critical for secrets storing credentials or API keys.
    pub async fn get_resource_policy(
        &self,
        secret_id: &str,
    ) -> Result<Option<String>, VaporError> {
        let output = self
            .inner
            .get_resource_policy()
            .secret_id(secret_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.resource_policy().filter(|s| !s.is_empty()).map(|s| s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    // awsJson1.1: POST JSON to a fixed `/` path, differentiated only by the
    // `x-amz-target` header (which `test_util::request` doesn't compare) —
    // same shape as `ram.rs`/`sagemaker.rs`. Crate name
    // (`aws-sdk-secretsmanager`) matches the endpoint hostname
    // (`secretsmanager.*`, verified against pinned `aws-sdk-secretsmanager`
    // 1.108.0's `config/endpoint.rs`). Request/response bodies use
    // PascalCase keys throughout (`SecretId`, `NextToken`, `MaxResults`,
    // `SecretList`, ...) per each op's `ser_*_input_input`/`de_*` codegen.
    // This crate ships no `serde_util.rs` at all (grepped, no hits) — every
    // `Option<T>` field genuinely stays `None` on a missing key, no gotcha
    // 14/20 default-fill surprise anywhere in this file. `list_secrets`'
    // aws-layer pagination loop forwards `limit` straight to AWS's
    // `MaxResults` with no client-side truncation (memory gotcha 13), so
    // the capped-pagination test below cans exactly `limit` items.
    // `ResourceNotFoundException` (not a throttling-classified code, memory
    // gotcha 1) is used for all `propagates_errors` tests since it's
    // modeled on every one of this file's 4 ops.
    const BASE: &str = "https://secretsmanager.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_secrets_lists_all() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"SecretList":[{"ARN":"arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1","Name":"s-1"}]}"#,
            ),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_secrets(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].arn(),
            Some("arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1")
        );
        assert_eq!(items[0].name(), Some("s-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_secrets_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"SecretList":[]}"#),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_secrets(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_secrets_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"SecretList":[{"Name":"s-1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_secrets(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_secrets_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"SecretList":[{"Name":"s-1"},{"Name":"s-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"SecretList":[{"Name":"s-3"}]}"#),
            ),
        ]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_secrets(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_secrets_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("ResourceNotFoundException", "not found"),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_secrets(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_secret_returns_output() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"s-1"}"#),
            json_response(
                200,
                r#"{"ARN":"arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1","Name":"s-1","RotationEnabled":true}"#,
            ),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let output = client.describe_secret("s-1").await.unwrap();

        assert_eq!(
            output.arn(),
            Some("arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1")
        );
        assert_eq!(output.name(), Some("s-1"));
        assert_eq!(output.rotation_enabled(), Some(true));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_secret_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"missing"}"#),
            json_error_response("ResourceNotFoundException", "secret not found"),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_secret("missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "secret not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_secret_value_returns_output() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"s-1"}"#),
            json_response(
                200,
                r#"{"ARN":"arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1","Name":"s-1","SecretString":"super-secret-value","VersionId":"v-1"}"#,
            ),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let output = client.get_secret_value("s-1").await.unwrap();

        assert_eq!(output.name(), Some("s-1"));
        assert_eq!(output.secret_string(), Some("super-secret-value"));
        assert_eq!(output.version_id(), Some("v-1"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_secret_value_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"missing"}"#),
            json_error_response("ResourceNotFoundException", "secret not found"),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let err = client.get_secret_value("missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "secret not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_resource_policy_returns_some_when_present() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"s-1"}"#),
            json_response(
                200,
                r#"{"ARN":"arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1","Name":"s-1","ResourcePolicy":"{\"Version\":\"2012-10-17\"}"}"#,
            ),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let policy = client.get_resource_policy("s-1").await.unwrap();

        assert_eq!(policy, Some(r#"{"Version":"2012-10-17"}"#.to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_resource_policy_returns_none_when_field_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"s-1"}"#),
            json_response(200, r#"{"ARN":"arn:s-1","Name":"s-1"}"#),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let policy = client.get_resource_policy("s-1").await.unwrap();

        assert_eq!(policy, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_resource_policy_returns_none_when_empty_string() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"s-1"}"#),
            json_response(
                200,
                r#"{"ARN":"arn:s-1","Name":"s-1","ResourcePolicy":""}"#,
            ),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let policy = client.get_resource_policy("s-1").await.unwrap();

        assert_eq!(policy, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_resource_policy_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"missing"}"#),
            json_error_response("ResourceNotFoundException", "secret not found"),
        )]);
        let client = SecretsManagerClient::new(&sdk_config(http_client.clone()));

        let err = client.get_resource_policy("missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "secret not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

