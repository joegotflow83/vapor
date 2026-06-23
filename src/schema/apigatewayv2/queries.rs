use async_graphql::{Context, Object, Result};

use crate::aws::apigatewayv2::ApiGatewayV2Client;
use crate::schema::apigatewayv2::types::{
    ApiV2, ApiV2DomainName, ApiV2Route, ApiV2Stage, ApiV2VpcLink,
};

#[derive(Default)]
pub struct ApiGatewayV2Query;

#[Object]
impl ApiGatewayV2Query {
    /// List all HTTP and WebSocket APIs (v2).
    async fn api_v2_apis(&self, ctx: &Context<'_>) -> Result<Vec<ApiV2>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let apis = client.get_apis().await?;
        Ok(apis.into_iter().map(ApiV2::from).collect())
    }

    /// List all stages for the given API (v2).
    async fn api_v2_stages(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<ApiV2Stage>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let stages = client.get_stages(&api_id).await?;
        Ok(stages.into_iter().map(ApiV2Stage::from).collect())
    }

    /// List all routes for the given API (v2).
    async fn api_v2_routes(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<ApiV2Route>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let routes = client.get_routes(&api_id).await?;
        Ok(routes.into_iter().map(ApiV2Route::from).collect())
    }

    /// List all custom domain names across APIs.
    async fn api_v2_domain_names(&self, ctx: &Context<'_>) -> Result<Vec<ApiV2DomainName>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let domains = client.get_domain_names().await?;
        Ok(domains.into_iter().map(ApiV2DomainName::from).collect())
    }

    /// List all VPC links used by private integrations.
    async fn api_v2_vpc_links(&self, ctx: &Context<'_>) -> Result<Vec<ApiV2VpcLink>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let links = client.get_vpc_links().await?;
        Ok(links.into_iter().map(ApiV2VpcLink::from).collect())
    }
}
