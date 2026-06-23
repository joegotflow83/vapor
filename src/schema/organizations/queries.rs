use async_graphql::{Context, Object, Result};
use aws_sdk_organizations::types::PolicyType;

use crate::aws::organizations::OrganizationsClient;
use crate::schema::organizations::types::{OrgAccount, OrgPolicy, OrganizationalUnit};

#[derive(Default)]
pub struct OrganizationsQuery;

#[Object]
impl OrganizationsQuery {
    async fn org_accounts(&self, ctx: &Context<'_>) -> Result<Vec<OrgAccount>> {
        let client = ctx.data::<OrganizationsClient>()?;
        let accounts = client.list_accounts().await?;
        Ok(accounts.iter().map(OrgAccount::from).collect())
    }

    async fn org_organizational_units(
        &self,
        ctx: &Context<'_>,
        parent_id: String,
    ) -> Result<Vec<OrganizationalUnit>> {
        let client = ctx.data::<OrganizationsClient>()?;
        let ous = client.list_organizational_units_for_parent(&parent_id).await?;
        Ok(ous.iter().map(OrganizationalUnit::from).collect())
    }

    async fn org_policies(
        &self,
        ctx: &Context<'_>,
        policy_type: String,
    ) -> Result<Vec<OrgPolicy>> {
        let client = ctx.data::<OrganizationsClient>()?;
        let pt = PolicyType::from(policy_type.as_str());
        let policies = client.list_policies(pt).await?;
        Ok(policies.iter().map(OrgPolicy::from).collect())
    }
}
