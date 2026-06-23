use async_graphql::{Context, Object, Result};

use crate::aws::route53::Route53Client;
use crate::schema::route53::types::{R53HealthCheck, R53HostedZone, R53ResourceRecordSet};

#[derive(Default)]
pub struct Route53Query;

#[Object]
impl Route53Query {
    /// List all Route 53 hosted zones.
    async fn r53_hosted_zones(&self, ctx: &Context<'_>) -> Result<Vec<R53HostedZone>> {
        let client = ctx.data::<Route53Client>()?;
        let zones = client.list_hosted_zones().await?;
        Ok(zones.into_iter().map(R53HostedZone::from).collect())
    }

    /// List all DNS resource record sets for the given hosted zone ID.
    async fn r53_records(
        &self,
        ctx: &Context<'_>,
        hosted_zone_id: String,
    ) -> Result<Vec<R53ResourceRecordSet>> {
        let client = ctx.data::<Route53Client>()?;
        let records = client.list_resource_record_sets(&hosted_zone_id).await?;
        Ok(records.into_iter().map(R53ResourceRecordSet::from).collect())
    }

    /// List all Route 53 health checks.
    async fn r53_health_checks(&self, ctx: &Context<'_>) -> Result<Vec<R53HealthCheck>> {
        let client = ctx.data::<Route53Client>()?;
        let checks = client.list_health_checks().await?;
        Ok(checks.into_iter().map(R53HealthCheck::from).collect())
    }
}
