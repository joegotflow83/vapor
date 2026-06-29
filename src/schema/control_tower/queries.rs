use async_graphql::{Context, Object, Result};

use crate::aws::control_tower::ControlTowerClient;
use crate::schema::control_tower::types::{ControlTowerLandingZone, EnabledControl};

#[derive(Default)]
pub struct ControlTowerQuery;

#[Object]
impl ControlTowerQuery {
    async fn control_tower_landing_zones(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<ControlTowerLandingZone>> {
        let client = ctx.data::<ControlTowerClient>()?;
        let zones = client.list_landing_zones().await?;
        Ok(zones.into_iter().map(ControlTowerLandingZone::from).collect())
    }

    async fn control_tower_enabled_controls(
        &self,
        ctx: &Context<'_>,
        target_identifier: Option<String>,
    ) -> Result<Vec<EnabledControl>> {
        let client = ctx.data::<ControlTowerClient>()?;
        let controls = client.list_enabled_controls(target_identifier).await?;
        Ok(controls.into_iter().map(EnabledControl::from).collect())
    }
}
