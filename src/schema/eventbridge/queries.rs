use async_graphql::{Context, Object, Result};

use crate::aws::eventbridge::EventBridgeClient;
use super::types::{EbEventBus, EbRule, EbTarget};

#[derive(Default)]
pub struct EventBridgeQuery;

#[Object]
impl EventBridgeQuery {
    async fn event_bridge_buses(&self, ctx: &Context<'_>) -> Result<Vec<EbEventBus>> {
        let client = ctx.data::<EventBridgeClient>()?;
        let buses = client.list_event_buses().await?;
        Ok(buses.iter().map(EbEventBus::from).collect())
    }

    async fn event_bridge_rules(
        &self,
        ctx: &Context<'_>,
        event_bus_name: Option<String>,
    ) -> Result<Vec<EbRule>> {
        let client = ctx.data::<EventBridgeClient>()?;
        let rules = client.list_rules(event_bus_name.as_deref()).await?;
        Ok(rules.iter().map(EbRule::from).collect())
    }

    async fn event_bridge_targets(
        &self,
        ctx: &Context<'_>,
        rule_name: String,
        event_bus_name: Option<String>,
    ) -> Result<Vec<EbTarget>> {
        let client = ctx.data::<EventBridgeClient>()?;
        let targets = client
            .list_targets_by_rule(&rule_name, event_bus_name.as_deref())
            .await?;
        Ok(targets.iter().map(EbTarget::from).collect())
    }
}
