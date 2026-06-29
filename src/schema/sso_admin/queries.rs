use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::sso_admin::SsoAdminClient;
use crate::schema::sso_admin::types::{SsoAccountAssignment, SsoInstance, SsoPermissionSet};

#[derive(Default)]
pub struct SsoAdminQuery;

#[Object]
impl SsoAdminQuery {
    async fn sso_instances(&self, ctx: &Context<'_>) -> Result<Vec<SsoInstance>> {
        let client = ctx.data::<SsoAdminClient>()?;
        let instances = client.list_instances().await?;
        Ok(instances.into_iter().map(SsoInstance::from).collect())
    }

    async fn sso_permission_sets(
        &self,
        ctx: &Context<'_>,
        instance_arn: String,
    ) -> Result<Vec<SsoPermissionSet>> {
        let client = ctx.data::<SsoAdminClient>()?;
        let arns = client.list_permission_sets(&instance_arn).await?;

        let futures: Vec<_> = arns
            .iter()
            .map(|arn| {
                let instance_arn = instance_arn.clone();
                let arn = arn.clone();
                async move {
                    let result = client.describe_permission_set(&instance_arn, &arn).await;
                    (arn, result)
                }
            })
            .collect();

        let results = join_all(futures).await;
        let mut permission_sets = Vec::new();
        for (arn, result) in results {
            match result {
                Ok(Some(ps)) => permission_sets.push(SsoPermissionSet::from(ps)),
                Ok(None) => {}
                Err(e) => {
                    return Err(async_graphql::Error::new(format!(
                        "Failed to describe permission set {arn}: {e}"
                    )));
                }
            }
        }
        Ok(permission_sets)
    }

    async fn sso_account_assignments(
        &self,
        ctx: &Context<'_>,
        instance_arn: String,
        account_id: String,
        permission_set_arn: String,
    ) -> Result<Vec<SsoAccountAssignment>> {
        let client = ctx.data::<SsoAdminClient>()?;
        let assignments = client
            .list_account_assignments(&instance_arn, &account_id, &permission_set_arn)
            .await?;
        Ok(assignments
            .into_iter()
            .map(SsoAccountAssignment::from)
            .collect())
    }
}
