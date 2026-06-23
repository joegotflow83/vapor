use async_graphql::{Context, Object, Result};

use crate::aws::shield::ShieldClient;
use crate::schema::shield::types::{AttackSummary, ProtectionGroup, ShieldProtection, ShieldSubscription};

#[derive(Default)]
pub struct ShieldQuery;

#[Object]
impl ShieldQuery {
    async fn shield_subscription(&self, ctx: &Context<'_>) -> Result<Option<ShieldSubscription>> {
        let client = ctx.data::<ShieldClient>()?;
        let sub = client.describe_subscription().await?;
        Ok(sub.map(ShieldSubscription::from))
    }

    async fn shield_protections(
        &self,
        ctx: &Context<'_>,
        resource_arn: Option<String>,
    ) -> Result<Vec<ShieldProtection>> {
        let client = ctx.data::<ShieldClient>()?;
        let protections = client.list_protections(resource_arn.as_deref()).await?;
        Ok(protections.into_iter().map(ShieldProtection::from).collect())
    }

    async fn shield_protection_groups(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<ProtectionGroup>> {
        let client = ctx.data::<ShieldClient>()?;
        let groups = client.list_protection_groups().await?;
        Ok(groups.into_iter().map(ProtectionGroup::from).collect())
    }

    async fn shield_attacks(
        &self,
        ctx: &Context<'_>,
        resource_arns: Option<Vec<String>>,
        start_time: Option<String>,
        end_time: Option<String>,
    ) -> Result<Vec<AttackSummary>> {
        let client = ctx.data::<ShieldClient>()?;
        let attacks = client.list_attacks(resource_arns, start_time, end_time).await?;
        Ok(attacks.into_iter().map(AttackSummary::from).collect())
    }
}
