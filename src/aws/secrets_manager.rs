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

    /// List all secrets via next_token pagination.
    pub async fn list_secrets(
        &self,
    ) -> Result<Vec<aws_sdk_secretsmanager::types::SecretListEntry>, VaporError> {
        let mut all = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_secrets();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all.extend(output.secret_list().iter().cloned());

            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(all)
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.resource_policy().filter(|s| !s.is_empty()).map(|s| s.to_string()))
    }
}
