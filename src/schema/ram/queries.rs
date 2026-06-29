use async_graphql::{Context, Object, Result};

use crate::aws::ram::RamClient;
use crate::schema::ram::types::{RamPrincipal, RamResource, RamResourceShare};

#[derive(Default)]
pub struct RamQuery;

#[Object]
impl RamQuery {
    async fn ram_resource_shares(
        &self,
        ctx: &Context<'_>,
        resource_owner: Option<String>,
    ) -> Result<Vec<RamResourceShare>> {
        let client = ctx.data::<RamClient>()?;
        let shares = client
            .list_resource_shares(resource_owner.as_deref())
            .await?;
        Ok(shares.into_iter().map(RamResourceShare::from).collect())
    }

    async fn ram_resources(
        &self,
        ctx: &Context<'_>,
        resource_owner: String,
        resource_share_arns: Option<Vec<String>>,
        resource_type: Option<String>,
    ) -> Result<Vec<RamResource>> {
        let client = ctx.data::<RamClient>()?;
        let resources = client
            .list_resources(&resource_owner, resource_share_arns, resource_type)
            .await?;
        Ok(resources.into_iter().map(RamResource::from).collect())
    }

    async fn ram_principals(
        &self,
        ctx: &Context<'_>,
        resource_owner: String,
        resource_share_arns: Option<Vec<String>>,
    ) -> Result<Vec<RamPrincipal>> {
        let client = ctx.data::<RamClient>()?;
        let principals = client
            .list_principals(&resource_owner, resource_share_arns)
            .await?;
        Ok(principals.into_iter().map(RamPrincipal::from).collect())
    }
}
