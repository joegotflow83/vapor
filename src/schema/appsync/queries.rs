use async_graphql::{Context, Object, Result};

use crate::aws::appsync::AppSyncClient;
use crate::schema::appsync::types::{AppSyncApi, AppSyncDataSource};

#[derive(Default)]
pub struct AppSyncQuery;

#[Object]
impl AppSyncQuery {
    async fn appsync_apis(&self, ctx: &Context<'_>) -> Result<Vec<AppSyncApi>> {
        let client = ctx.data::<AppSyncClient>()?;
        let apis = client.list_graphql_apis().await?;
        Ok(apis.into_iter().map(AppSyncApi::from).collect())
    }

    async fn appsync_data_sources(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<AppSyncDataSource>> {
        let client = ctx.data::<AppSyncClient>()?;
        let sources = client.list_data_sources(&api_id).await?;
        Ok(sources.into_iter().map(AppSyncDataSource::from).collect())
    }
}
