use async_graphql::{Context, Object, Result};

use crate::aws::connect::ConnectClient;
use crate::schema::connect::types::{ConnectContactFlow, ConnectInstance, ConnectQueue, ConnectUser};

#[derive(Default)]
pub struct ConnectQuery;

#[Object]
impl ConnectQuery {
    async fn connect_instances(&self, ctx: &Context<'_>) -> Result<Vec<ConnectInstance>> {
        let client = ctx.data::<ConnectClient>()?;
        let items = client.list_instances().await?;
        Ok(items.into_iter().map(ConnectInstance::from).collect())
    }

    async fn connect_queues(
        &self,
        ctx: &Context<'_>,
        instance_id: String,
        queue_types: Option<Vec<String>>,
    ) -> Result<Vec<ConnectQueue>> {
        let client = ctx.data::<ConnectClient>()?;
        let items = client.list_queues(&instance_id, queue_types).await?;
        Ok(items.into_iter().map(ConnectQueue::from).collect())
    }

    async fn connect_contact_flows(
        &self,
        ctx: &Context<'_>,
        instance_id: String,
        contact_flow_types: Option<Vec<String>>,
    ) -> Result<Vec<ConnectContactFlow>> {
        let client = ctx.data::<ConnectClient>()?;
        let items = client
            .list_contact_flows(&instance_id, contact_flow_types)
            .await?;
        Ok(items.into_iter().map(ConnectContactFlow::from).collect())
    }

    async fn connect_users(
        &self,
        ctx: &Context<'_>,
        instance_id: String,
    ) -> Result<Vec<ConnectUser>> {
        let client = ctx.data::<ConnectClient>()?;
        let items = client.list_users(&instance_id).await?;
        Ok(items.into_iter().map(ConnectUser::from).collect())
    }
}
