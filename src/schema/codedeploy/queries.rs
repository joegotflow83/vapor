use async_graphql::{Context, Object, Result};

use crate::aws::codedeploy::CodeDeployClient;
use crate::schema::codedeploy::types::{DeployApplication, Deployment, DeploymentGroup};

#[derive(Default)]
pub struct CodeDeployQuery;

#[Object]
impl CodeDeployQuery {
    async fn deploy_applications(&self, ctx: &Context<'_>) -> Result<Vec<DeployApplication>> {
        let client = ctx.data::<CodeDeployClient>()?;
        let names = client.list_applications().await?;
        if names.is_empty() {
            return Ok(Vec::new());
        }
        let apps = client.batch_get_applications(names).await?;
        Ok(apps.into_iter().map(DeployApplication::from).collect())
    }

    async fn deployment_groups(
        &self,
        ctx: &Context<'_>,
        application_name: String,
    ) -> Result<Vec<DeploymentGroup>> {
        let client = ctx.data::<CodeDeployClient>()?;
        let names = client.list_deployment_groups(&application_name).await?;
        if names.is_empty() {
            return Ok(Vec::new());
        }
        let groups = client.batch_get_deployment_groups(&application_name, names).await?;
        Ok(groups.into_iter().map(DeploymentGroup::from).collect())
    }

    async fn deployments(
        &self,
        ctx: &Context<'_>,
        application_name: Option<String>,
        deployment_group_name: Option<String>,
    ) -> Result<Vec<Deployment>> {
        let client = ctx.data::<CodeDeployClient>()?;
        let ids = client.list_deployments(application_name.as_deref(), deployment_group_name.as_deref()).await?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let deployments = client.batch_get_deployments(ids).await?;
        Ok(deployments.into_iter().map(Deployment::from).collect())
    }
}
