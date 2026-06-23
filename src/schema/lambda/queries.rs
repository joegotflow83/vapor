use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::lambda::LambdaClient;
use crate::schema::lambda::types::{
    LambdaAlias, LambdaEventSourceMapping, LambdaFunction, LambdaLayer,
};

#[derive(Default)]
pub struct LambdaQuery;

#[Object]
impl LambdaQuery {
    /// List all Lambda functions with metadata. Environment variable values are
    /// intentionally omitted — only key names are returned.
    async fn lambda_functions(&self, ctx: &Context<'_>) -> Result<Vec<LambdaFunction>> {
        let lambda = ctx.data::<LambdaClient>()?;
        let configs = lambda.list_functions().await?;

        let futures: Vec<_> = configs
            .into_iter()
            .map(|cfg| async move {
                let tags = if let Some(arn) = cfg.function_arn() {
                    lambda.list_tags(arn).await.unwrap_or_default()
                } else {
                    std::collections::HashMap::new()
                };
                LambdaFunction::from_config_and_tags(cfg, tags)
            })
            .collect();

        Ok(join_all(futures).await)
    }

    /// List aliases for a Lambda function.
    async fn lambda_aliases(
        &self,
        ctx: &Context<'_>,
        function_name: String,
    ) -> Result<Vec<LambdaAlias>> {
        let lambda = ctx.data::<LambdaClient>()?;
        let aliases = lambda.list_aliases(&function_name).await?;
        Ok(aliases.into_iter().map(LambdaAlias::from).collect())
    }

    /// List event source mappings. If functionName is provided, filter by that function.
    async fn lambda_event_source_mappings(
        &self,
        ctx: &Context<'_>,
        function_name: Option<String>,
    ) -> Result<Vec<LambdaEventSourceMapping>> {
        let lambda = ctx.data::<LambdaClient>()?;
        let mappings = lambda
            .list_event_source_mappings(function_name.as_deref())
            .await?;
        Ok(mappings
            .into_iter()
            .map(LambdaEventSourceMapping::from)
            .collect())
    }

    /// List all Lambda layers with their latest published version metadata.
    async fn lambda_layers(&self, ctx: &Context<'_>) -> Result<Vec<LambdaLayer>> {
        let lambda = ctx.data::<LambdaClient>()?;
        let layers = lambda.list_layers().await?;
        Ok(layers.into_iter().map(LambdaLayer::from).collect())
    }

    /// Fetch the resource-based policy document for a Lambda function.
    /// Returns the raw JSON policy string, or null if no policy is attached.
    /// Reveals which principals (AWS services, accounts, organizations) have
    /// permission to invoke the function — essential for detecting unintended
    /// cross-account access or public invocability.
    async fn lambda_function_policy(
        &self,
        ctx: &Context<'_>,
        function_name: String,
    ) -> Result<Option<String>> {
        let lambda = ctx.data::<LambdaClient>()?;
        Ok(lambda.get_function_policy(&function_name).await?)
    }
}
