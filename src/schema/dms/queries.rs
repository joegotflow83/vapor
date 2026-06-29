use async_graphql::{Context, Object, Result};

use crate::aws::dms::DmsClient;
use crate::schema::dms::types::{DmsEndpoint, DmsReplicationInstance, DmsReplicationTask};

#[derive(Default)]
pub struct DmsQuery;

#[Object]
impl DmsQuery {
    async fn dms_replication_instances(&self, ctx: &Context<'_>) -> Result<Vec<DmsReplicationInstance>> {
        let client = ctx.data::<DmsClient>()?;
        let instances = client.describe_replication_instances().await?;
        Ok(instances.into_iter().map(DmsReplicationInstance::from).collect())
    }

    async fn dms_endpoints(
        &self,
        ctx: &Context<'_>,
        endpoint_type: Option<String>,
    ) -> Result<Vec<DmsEndpoint>> {
        let client = ctx.data::<DmsClient>()?;
        let endpoints = client.describe_endpoints(endpoint_type).await?;
        Ok(endpoints.into_iter().map(DmsEndpoint::from).collect())
    }

    async fn dms_replication_tasks(&self, ctx: &Context<'_>) -> Result<Vec<DmsReplicationTask>> {
        let client = ctx.data::<DmsClient>()?;
        let tasks = client.describe_replication_tasks().await?;
        Ok(tasks.into_iter().map(DmsReplicationTask::from).collect())
    }
}
