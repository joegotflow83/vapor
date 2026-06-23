use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::cloudformation::CloudFormationClient;
use crate::schema::cloudformation::types::{CfnExport, CfnStack, CfnStackResource};

#[derive(Default)]
pub struct CloudFormationQuery;

#[Object]
impl CloudFormationQuery {
    /// List CloudFormation stacks. Optionally filter by names and/or status strings.
    /// Without arguments returns all non-DELETE_COMPLETE stacks.
    async fn cfn_stacks(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
        status_filter: Option<Vec<String>>,
    ) -> Result<Vec<CfnStack>> {
        let client = ctx.data::<CloudFormationClient>()?;

        let raw_stacks = match names {
            Some(ref ns) if !ns.is_empty() => {
                let futs = ns.iter().map(|n| client.describe_stacks(Some(n.as_str())));
                let results = join_all(futs).await;
                results
                    .into_iter()
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
            }
            _ => client.describe_stacks(None).await?,
        };

        // Client-side status filter (describe_stacks doesn't accept a status param)
        let filtered = match status_filter {
            Some(ref statuses) if !statuses.is_empty() => raw_stacks
                .into_iter()
                .filter(|s| {
                    s.stack_status()
                        .map(|st| statuses.contains(&st.as_str().to_string()))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>(),
            _ => raw_stacks,
        };

        Ok(filtered.iter().map(CfnStack::from).collect())
    }

    /// List all resources in a CloudFormation stack.
    async fn cfn_stack_resources(
        &self,
        ctx: &Context<'_>,
        stack_name: String,
    ) -> Result<Vec<CfnStackResource>> {
        let client = ctx.data::<CloudFormationClient>()?;
        let resources = client.list_stack_resources(&stack_name).await?;
        Ok(resources.iter().map(CfnStackResource::from).collect())
    }

    /// List all CloudFormation exports (cross-stack references) in the account/region.
    async fn cfn_exports(&self, ctx: &Context<'_>) -> Result<Vec<CfnExport>> {
        let client = ctx.data::<CloudFormationClient>()?;
        let exports = client.list_exports().await?;
        Ok(exports.iter().map(CfnExport::from).collect())
    }
}
