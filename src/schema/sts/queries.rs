use async_graphql::{Context, Object, Result};

use crate::aws::sts::StsClient;
use crate::schema::sts::types::CallerIdentity;

#[derive(Default)]
pub struct StsQuery;

#[Object]
impl StsQuery {
    async fn sts_caller_identity(&self, ctx: &Context<'_>) -> Result<CallerIdentity> {
        let client = ctx.data::<StsClient>()?;
        let output = client.get_caller_identity().await?;
        Ok(CallerIdentity::from(output))
    }
}
