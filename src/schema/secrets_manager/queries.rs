use async_graphql::{Context, Object, Result};

use crate::aws::secrets_manager::SecretsManagerClient;
use crate::schema::secrets_manager::types::{Secret, SecretValue};

#[derive(Default)]
pub struct SecretsManagerQuery;

#[Object]
impl SecretsManagerQuery {
    /// List all Secrets Manager secrets with their metadata.
    async fn secrets_list(&self, ctx: &Context<'_>) -> Result<Vec<Secret>> {
        let client = ctx.data::<SecretsManagerClient>()?;
        let entries = client.list_secrets().await?;
        Ok(entries.into_iter().map(Secret::from).collect())
    }

    /// Describe a single secret by ID or ARN.
    async fn secret_describe(
        &self,
        ctx: &Context<'_>,
        secret_id: String,
    ) -> Result<Option<Secret>> {
        let client = ctx.data::<SecretsManagerClient>()?;
        let output = client.describe_secret(&secret_id).await?;
        Ok(Some(Secret::from(output)))
    }

    /// Retrieve the value of a secret by ID or ARN.
    async fn secret_value(
        &self,
        ctx: &Context<'_>,
        secret_id: String,
    ) -> Result<Option<SecretValue>> {
        let client = ctx.data::<SecretsManagerClient>()?;
        let output = client.get_secret_value(&secret_id).await?;
        Ok(Some(SecretValue::from(output)))
    }

    /// Fetch the resource-based policy document attached to a secret.
    /// Returns the raw JSON policy string, or null if no policy is attached.
    /// Reveals cross-account access grants beyond the secret owner — use to audit
    /// which principals (accounts, services, roles) can access credentials or API keys.
    async fn secret_resource_policy(
        &self,
        ctx: &Context<'_>,
        secret_id: String,
    ) -> Result<Option<String>> {
        let client = ctx.data::<SecretsManagerClient>()?;
        Ok(client.get_resource_policy(&secret_id).await?)
    }
}
