use async_graphql::{Context, Object, Result};

use crate::aws::xray::XRayClient;
use crate::schema::xray::types::{XRayEncryptionConfig, XRayGroup, XRaySamplingRule};

#[derive(Default)]
pub struct XRayQuery;

#[Object]
impl XRayQuery {
    async fn xray_groups(&self, ctx: &Context<'_>) -> Result<Vec<XRayGroup>> {
        let client = ctx.data::<XRayClient>()?;
        let groups = client.get_groups().await?;
        Ok(groups.into_iter().map(XRayGroup::from).collect())
    }

    async fn xray_sampling_rules(&self, ctx: &Context<'_>) -> Result<Vec<XRaySamplingRule>> {
        let client = ctx.data::<XRayClient>()?;
        let rules = client.list_sampling_rules().await?;
        Ok(rules.into_iter().map(XRaySamplingRule::from).collect())
    }

    async fn xray_encryption_config(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<XRayEncryptionConfig>> {
        let client = ctx.data::<XRayClient>()?;
        let config = client.get_encryption_config().await?;
        Ok(config.map(XRayEncryptionConfig::from))
    }
}
