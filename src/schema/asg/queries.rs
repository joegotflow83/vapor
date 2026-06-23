use async_graphql::{Context, Object, Result};

use crate::aws::autoscaling::AutoscalingClient;
use crate::schema::asg::types::{AutoScalingGroup, ScalingActivity};

#[derive(Default)]
pub struct AsgQuery;

#[Object]
impl AsgQuery {
    async fn auto_scaling_groups(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<AutoScalingGroup>> {
        let client = ctx.data::<AutoscalingClient>()?;
        let results = client.describe_auto_scaling_groups(names).await?;
        Ok(results.into_iter().map(AutoScalingGroup::from).collect())
    }

    async fn scaling_activities(
        &self,
        ctx: &Context<'_>,
        auto_scaling_group_name: Option<String>,
    ) -> Result<Vec<ScalingActivity>> {
        let client = ctx.data::<AutoscalingClient>()?;
        let results = client
            .describe_scaling_activities(auto_scaling_group_name)
            .await?;
        Ok(results.into_iter().map(ScalingActivity::from).collect())
    }
}
