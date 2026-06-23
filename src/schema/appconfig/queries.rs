use async_graphql::{Context, Object, Result};

use crate::aws::appconfig::AppConfigClient;
use crate::schema::appconfig::types::{AppConfigApplication, AppConfigEnvironment, AppConfigProfile};

#[derive(Default)]
pub struct AppConfigQuery;

#[Object]
impl AppConfigQuery {
    async fn appconfig_applications(&self, ctx: &Context<'_>) -> Result<Vec<AppConfigApplication>> {
        let client = ctx.data::<AppConfigClient>()?;
        let apps = client.list_applications().await?;
        Ok(apps.into_iter().map(AppConfigApplication::from).collect())
    }

    async fn appconfig_environments(
        &self,
        ctx: &Context<'_>,
        application_id: String,
    ) -> Result<Vec<AppConfigEnvironment>> {
        let client = ctx.data::<AppConfigClient>()?;
        let envs = client.list_environments(&application_id).await?;
        Ok(envs.into_iter().map(AppConfigEnvironment::from).collect())
    }

    async fn appconfig_profiles(
        &self,
        ctx: &Context<'_>,
        application_id: String,
    ) -> Result<Vec<AppConfigProfile>> {
        let client = ctx.data::<AppConfigClient>()?;
        let profiles = client.list_configuration_profiles(&application_id).await?;
        Ok(profiles.into_iter().map(AppConfigProfile::from).collect())
    }
}
