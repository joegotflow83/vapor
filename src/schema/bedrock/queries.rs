use async_graphql::{Context, Object, Result};

use crate::aws::bedrock::BedrockClient;
use crate::schema::bedrock::types::{
    BedrockCustomModel, BedrockFoundationModel, BedrockGuardrail,
    BedrockModelInvocationLoggingConfig,
};

#[derive(Default)]
pub struct BedrockQuery;

#[Object]
impl BedrockQuery {
    async fn bedrock_foundation_models(
        &self,
        ctx: &Context<'_>,
        provider: Option<String>,
        by_output_modality: Option<String>,
        by_inference_type: Option<String>,
    ) -> Result<Vec<BedrockFoundationModel>> {
        let client = ctx.data::<BedrockClient>()?;
        let models = client
            .list_foundation_models(provider, by_output_modality, by_inference_type)
            .await?;
        Ok(models.into_iter().map(BedrockFoundationModel::from).collect())
    }

    async fn bedrock_custom_models(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<BedrockCustomModel>> {
        let client = ctx.data::<BedrockClient>()?;
        let models = client.list_custom_models().await?;
        Ok(models.into_iter().map(BedrockCustomModel::from).collect())
    }

    async fn bedrock_guardrails(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<BedrockGuardrail>> {
        let client = ctx.data::<BedrockClient>()?;
        let guardrails = client.list_guardrails().await?;
        Ok(guardrails.into_iter().map(BedrockGuardrail::from).collect())
    }

    async fn bedrock_model_invocation_logging_config(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<BedrockModelInvocationLoggingConfig>> {
        let client = ctx.data::<BedrockClient>()?;
        let config = client.get_model_invocation_logging_config().await?;
        Ok(config.map(BedrockModelInvocationLoggingConfig::from))
    }
}
