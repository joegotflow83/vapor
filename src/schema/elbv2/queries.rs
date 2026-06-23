use async_graphql::{Context, Object, Result};

use crate::aws::elbv2::Elbv2Client;
use crate::schema::elbv2::types::{Listener, ListenerRule, LoadBalancer, TargetGroup, TargetHealthInfo};

#[derive(Default)]
pub struct Elbv2Query;

#[Object]
impl Elbv2Query {
    async fn load_balancers(
        &self,
        ctx: &Context<'_>,
        arns: Option<Vec<String>>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<LoadBalancer>> {
        let client = ctx.data::<Elbv2Client>()?;
        let results = client.describe_load_balancers(arns, names).await?;
        Ok(results.into_iter().map(LoadBalancer::from).collect())
    }

    async fn target_groups(
        &self,
        ctx: &Context<'_>,
        arns: Option<Vec<String>>,
        load_balancer_arn: Option<String>,
    ) -> Result<Vec<TargetGroup>> {
        let client = ctx.data::<Elbv2Client>()?;
        let results = client.describe_target_groups(arns, load_balancer_arn).await?;
        Ok(results.into_iter().map(TargetGroup::from).collect())
    }

    async fn target_health(
        &self,
        ctx: &Context<'_>,
        target_group_arn: String,
    ) -> Result<Vec<TargetHealthInfo>> {
        let client = ctx.data::<Elbv2Client>()?;
        let results = client.describe_target_health(target_group_arn).await?;
        Ok(results.into_iter().map(TargetHealthInfo::from).collect())
    }

    async fn listeners(
        &self,
        ctx: &Context<'_>,
        load_balancer_arn: String,
    ) -> Result<Vec<Listener>> {
        let client = ctx.data::<Elbv2Client>()?;
        let results = client.describe_listeners(load_balancer_arn).await?;
        Ok(results.into_iter().map(Listener::from).collect())
    }

    async fn listener_rules(
        &self,
        ctx: &Context<'_>,
        listener_arn: String,
    ) -> Result<Vec<ListenerRule>> {
        let client = ctx.data::<Elbv2Client>()?;
        let results = client.describe_rules(listener_arn).await?;
        Ok(results.into_iter().map(ListenerRule::from).collect())
    }
}
