use async_graphql::{Context, Object, Result};

use crate::aws::lightsail::LightsailClient;
use crate::schema::lightsail::types::{
    LightsailDatabase, LightsailInstance, LightsailLoadBalancer, LightsailStaticIp,
};

#[derive(Default)]
pub struct LightsailQuery;

#[Object]
impl LightsailQuery {
    async fn lightsail_instances(&self, ctx: &Context<'_>) -> Result<Vec<LightsailInstance>> {
        let client = ctx.data::<LightsailClient>()?;
        let instances = client.get_instances().await?;
        Ok(instances.into_iter().map(LightsailInstance::from).collect())
    }

    async fn lightsail_databases(&self, ctx: &Context<'_>) -> Result<Vec<LightsailDatabase>> {
        let client = ctx.data::<LightsailClient>()?;
        let dbs = client.get_relational_databases().await?;
        Ok(dbs.into_iter().map(LightsailDatabase::from).collect())
    }

    async fn lightsail_load_balancers(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<LightsailLoadBalancer>> {
        let client = ctx.data::<LightsailClient>()?;
        let lbs = client.get_load_balancers().await?;
        Ok(lbs.into_iter().map(LightsailLoadBalancer::from).collect())
    }

    async fn lightsail_static_ips(&self, ctx: &Context<'_>) -> Result<Vec<LightsailStaticIp>> {
        let client = ctx.data::<LightsailClient>()?;
        let ips = client.get_static_ips().await?;
        Ok(ips.into_iter().map(LightsailStaticIp::from).collect())
    }
}
