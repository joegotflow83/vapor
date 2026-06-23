use async_graphql::{Context, Object, Result};

use crate::aws::direct_connect::DirectConnectClient;
use crate::schema::direct_connect::types::{DxConnection, DxVirtualInterface};

#[derive(Default)]
pub struct DirectConnectQuery;

#[Object]
impl DirectConnectQuery {
    async fn dx_connections(&self, ctx: &Context<'_>) -> Result<Vec<DxConnection>> {
        let client = ctx.data::<DirectConnectClient>()?;
        let connections = client.describe_connections().await?;
        Ok(connections.into_iter().map(DxConnection::from).collect())
    }

    async fn dx_virtual_interfaces(
        &self,
        ctx: &Context<'_>,
        connection_id: Option<String>,
    ) -> Result<Vec<DxVirtualInterface>> {
        let client = ctx.data::<DirectConnectClient>()?;
        let vifs = client
            .describe_virtual_interfaces(connection_id.as_deref())
            .await?;
        Ok(vifs.into_iter().map(DxVirtualInterface::from).collect())
    }
}
