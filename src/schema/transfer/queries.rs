use async_graphql::{Context, Object, Result};

use crate::aws::transfer::TransferClient;
use crate::schema::transfer::types::{TransferServer, TransferUser};

#[derive(Default)]
pub struct TransferQuery;

#[Object]
impl TransferQuery {
    async fn transfer_servers(&self, ctx: &Context<'_>) -> Result<Vec<TransferServer>> {
        let client = ctx.data::<TransferClient>()?;
        let servers = client.list_servers().await?;
        Ok(servers.into_iter().map(TransferServer::from).collect())
    }

    async fn transfer_users(
        &self,
        ctx: &Context<'_>,
        server_id: String,
    ) -> Result<Vec<TransferUser>> {
        let client = ctx.data::<TransferClient>()?;
        let users = client.list_users(&server_id).await?;
        Ok(users
            .into_iter()
            .map(|u| TransferUser::from_listed(u, server_id.clone()))
            .collect())
    }
}
