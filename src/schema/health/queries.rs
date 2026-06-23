use async_graphql::{Context, Object, Result};

use crate::aws::health::HealthClient;
use crate::schema::health::types::HealthEvent;

#[derive(Default)]
pub struct HealthQuery;

#[Object]
impl HealthQuery {
    async fn health_events(
        &self,
        ctx: &Context<'_>,
        status_codes: Option<Vec<String>>,
        services: Option<Vec<String>>,
    ) -> Result<Vec<HealthEvent>> {
        let client = ctx.data::<HealthClient>()?;
        let events = client.describe_events(status_codes, services).await?;
        Ok(events.into_iter().map(HealthEvent::from).collect())
    }
}
