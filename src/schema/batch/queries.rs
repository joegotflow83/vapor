use async_graphql::{Context, Object, Result};

use crate::aws::batch::BatchClient;
use crate::schema::batch::types::{BatchComputeEnvironment, BatchJobDefinition, BatchJobQueue};

#[derive(Default)]
pub struct BatchQuery;

#[Object]
impl BatchQuery {
    async fn batch_job_queues(&self, ctx: &Context<'_>) -> Result<Vec<BatchJobQueue>> {
        let client = ctx.data::<BatchClient>()?;
        let queues = client.describe_job_queues().await?;
        Ok(queues.into_iter().map(BatchJobQueue::from).collect())
    }

    async fn batch_compute_environments(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<BatchComputeEnvironment>> {
        let client = ctx.data::<BatchClient>()?;
        let envs = client.describe_compute_environments().await?;
        Ok(envs.into_iter().map(BatchComputeEnvironment::from).collect())
    }

    async fn batch_job_definitions(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
    ) -> Result<Vec<BatchJobDefinition>> {
        let client = ctx.data::<BatchClient>()?;
        let defs = client.describe_job_definitions(status.as_deref()).await?;
        Ok(defs.into_iter().map(BatchJobDefinition::from).collect())
    }
}
