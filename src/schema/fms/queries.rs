use async_graphql::{Context, Object, Result};

use crate::aws::fms::FmsClient;
use crate::schema::fms::types::{FmsPolicy, FmsPolicyComplianceStatus};

#[derive(Default)]
pub struct FmsQuery;

#[Object]
impl FmsQuery {
    async fn fms_policies(&self, ctx: &Context<'_>) -> Result<Vec<FmsPolicy>> {
        let client = ctx.data::<FmsClient>()?;
        let policies = client.list_policies().await?;
        Ok(policies.into_iter().map(FmsPolicy::from).collect())
    }

    async fn fms_policy_compliance_statuses(
        &self,
        ctx: &Context<'_>,
        policy_id: String,
    ) -> Result<Vec<FmsPolicyComplianceStatus>> {
        let client = ctx.data::<FmsClient>()?;
        let statuses = client.list_compliance_status(&policy_id).await?;
        Ok(statuses
            .into_iter()
            .map(FmsPolicyComplianceStatus::from)
            .collect())
    }

    async fn fms_member_accounts(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let client = ctx.data::<FmsClient>()?;
        Ok(client.list_member_accounts().await?)
    }
}
