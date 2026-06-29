use async_graphql::{Context, Object, Result};

use crate::aws::storage_gateway::StorageGatewayClient;
use crate::schema::storage_gateway::types::{
    StorageGatewayFileShare, StorageGatewayGateway, StorageGatewayVolume,
};

#[derive(Default)]
pub struct StorageGatewayQuery;

#[Object]
impl StorageGatewayQuery {
    async fn storage_gateways(&self, ctx: &Context<'_>) -> Result<Vec<StorageGatewayGateway>> {
        let client = ctx.data::<StorageGatewayClient>()?;
        let gateways = client.list_gateways().await?;
        Ok(gateways.into_iter().map(StorageGatewayGateway::from).collect())
    }

    async fn storage_gateway_volumes(
        &self,
        ctx: &Context<'_>,
        gateway_arn: String,
    ) -> Result<Vec<StorageGatewayVolume>> {
        let client = ctx.data::<StorageGatewayClient>()?;
        let volumes = client.list_volumes(gateway_arn).await?;
        Ok(volumes.into_iter().map(StorageGatewayVolume::from).collect())
    }

    async fn storage_gateway_file_shares(
        &self,
        ctx: &Context<'_>,
        gateway_arn: String,
    ) -> Result<Vec<StorageGatewayFileShare>> {
        let client = ctx.data::<StorageGatewayClient>()?;
        let shares = client.list_file_shares(gateway_arn).await?;
        Ok(shares.into_iter().map(StorageGatewayFileShare::from).collect())
    }
}
