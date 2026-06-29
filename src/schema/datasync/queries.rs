use async_graphql::{Context, Object, Result};

use crate::aws::datasync::DataSyncClient;
use crate::schema::datasync::types::{DataSyncAgent, DataSyncLocation, DataSyncTask, DataSyncTaskExecution};

#[derive(Default)]
pub struct DataSyncQuery;

#[Object]
impl DataSyncQuery {
    async fn data_sync_agents(&self, ctx: &Context<'_>) -> Result<Vec<DataSyncAgent>> {
        let client = ctx.data::<DataSyncClient>()?;
        let agents = client.list_agents().await?;
        Ok(agents.into_iter().map(DataSyncAgent::from).collect())
    }

    async fn data_sync_locations(&self, ctx: &Context<'_>) -> Result<Vec<DataSyncLocation>> {
        let client = ctx.data::<DataSyncClient>()?;
        let locations = client.list_locations().await?;
        Ok(locations.into_iter().map(DataSyncLocation::from).collect())
    }

    async fn data_sync_tasks(&self, ctx: &Context<'_>) -> Result<Vec<DataSyncTask>> {
        let client = ctx.data::<DataSyncClient>()?;
        let tasks = client.list_tasks().await?;
        Ok(tasks.into_iter().map(DataSyncTask::from).collect())
    }

    async fn data_sync_task_executions(
        &self,
        ctx: &Context<'_>,
        task_arn: String,
    ) -> Result<Vec<DataSyncTaskExecution>> {
        let client = ctx.data::<DataSyncClient>()?;
        let executions = client.list_task_executions(task_arn).await?;
        Ok(executions.into_iter().map(DataSyncTaskExecution::from).collect())
    }
}
