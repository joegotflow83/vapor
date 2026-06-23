use async_graphql::{Context, Object, Result};

use crate::aws::macie::MacieClient;
use crate::schema::macie::types::{MacieBucketSummary, MacieFinding};

#[derive(Default)]
pub struct MacieQuery;

#[Object]
impl MacieQuery {
    async fn macie_findings(
        &self,
        ctx: &Context<'_>,
        severity: Option<String>,
        finding_type: Option<String>,
    ) -> Result<Vec<MacieFinding>> {
        let client = ctx.data::<MacieClient>()?;
        let ids = client
            .list_findings(severity.as_deref(), finding_type.as_deref())
            .await?;
        let findings = client.get_findings(ids).await?;
        Ok(findings.into_iter().map(MacieFinding::from).collect())
    }

    async fn macie_bucket_summaries(&self, ctx: &Context<'_>) -> Result<Vec<MacieBucketSummary>> {
        let client = ctx.data::<MacieClient>()?;
        let buckets = client.describe_buckets().await?;
        Ok(buckets.into_iter().map(MacieBucketSummary::from).collect())
    }
}
