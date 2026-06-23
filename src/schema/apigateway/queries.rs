use async_graphql::{Context, Object, Result};

use crate::aws::apigateway::ApiGatewayClient;
use crate::schema::apigateway::types::{
    ApigwDeployment, ApigwHttpApi, ApigwHttpRoute, ApigwHttpStage, ApigwResource, ApigwRestApi,
    ApigwRestStage,
};

#[derive(Default)]
pub struct ApiGatewayQuery;

#[Object]
impl ApiGatewayQuery {
    /// List all REST APIs (v1) in the region.
    async fn apigw_rest_apis(&self, ctx: &Context<'_>) -> Result<Vec<ApigwRestApi>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let apis = client.list_rest_apis().await?;
        Ok(apis.into_iter().map(ApigwRestApi::from).collect())
    }

    /// List all stages for the given REST API (v1).
    async fn apigw_rest_stages(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<ApigwRestStage>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let stages = client.list_rest_stages(&api_id).await?;
        Ok(stages.into_iter().map(ApigwRestStage::from).collect())
    }

    /// List all resources (path nodes) for the given REST API (v1).
    async fn apigw_rest_resources(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<ApigwResource>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let resources = client.list_rest_resources(&api_id).await?;
        Ok(resources.into_iter().map(ApigwResource::from).collect())
    }

    /// List all deployments for the given REST API (v1).
    async fn apigw_rest_deployments(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<ApigwDeployment>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let deployments = client.list_rest_deployments(&api_id).await?;
        Ok(deployments.into_iter().map(ApigwDeployment::from).collect())
    }

    /// List all HTTP/WebSocket APIs (v2) in the region.
    async fn apigw_http_apis(&self, ctx: &Context<'_>) -> Result<Vec<ApigwHttpApi>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let apis = client.list_http_apis().await?;
        Ok(apis.into_iter().map(ApigwHttpApi::from).collect())
    }

    /// List all stages for the given HTTP/WebSocket API (v2).
    async fn apigw_http_stages(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<ApigwHttpStage>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let stages = client.list_http_stages(&api_id).await?;
        Ok(stages.into_iter().map(ApigwHttpStage::from).collect())
    }

    /// List all routes for the given HTTP/WebSocket API (v2).
    async fn apigw_http_routes(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<ApigwHttpRoute>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let routes = client.list_http_routes(&api_id).await?;
        Ok(routes.into_iter().map(ApigwHttpRoute::from).collect())
    }
}
