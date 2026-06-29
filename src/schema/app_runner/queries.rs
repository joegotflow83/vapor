use async_graphql::{Context, Object, Result};

use crate::aws::app_runner::AppRunnerClient;
use crate::schema::app_runner::types::{AppRunnerService, AppRunnerVpcConnector};

#[derive(Default)]
pub struct AppRunnerQuery;

#[Object]
impl AppRunnerQuery {
    async fn app_runner_services(&self, ctx: &Context<'_>) -> Result<Vec<AppRunnerService>> {
        let client = ctx.data::<AppRunnerClient>()?;
        let services = client.list_services().await?;
        Ok(services.into_iter().map(AppRunnerService::from).collect())
    }

    async fn app_runner_service(
        &self,
        ctx: &Context<'_>,
        service_arn: String,
    ) -> Result<Option<AppRunnerService>> {
        let client = ctx.data::<AppRunnerClient>()?;
        let svc = client.describe_service(&service_arn).await?;
        Ok(svc.map(AppRunnerService::from))
    }

    async fn app_runner_vpc_connectors(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<AppRunnerVpcConnector>> {
        let client = ctx.data::<AppRunnerClient>()?;
        let connectors = client.list_vpc_connectors().await?;
        Ok(connectors.into_iter().map(AppRunnerVpcConnector::from).collect())
    }
}
