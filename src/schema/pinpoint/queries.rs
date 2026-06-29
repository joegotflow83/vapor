use async_graphql::{Context, Object, Result};

use crate::aws::pinpoint::PinpointClient;
use crate::schema::pinpoint::types::{PinpointApp, PinpointCampaign, PinpointSegment};

#[derive(Default)]
pub struct PinpointQuery;

#[Object]
impl PinpointQuery {
    async fn pinpoint_apps(&self, ctx: &Context<'_>) -> Result<Vec<PinpointApp>> {
        let client = ctx.data::<PinpointClient>()?;
        let items = client.get_apps().await?;
        Ok(items.into_iter().map(PinpointApp::from).collect())
    }

    async fn pinpoint_campaigns(
        &self,
        ctx: &Context<'_>,
        application_id: String,
    ) -> Result<Vec<PinpointCampaign>> {
        let client = ctx.data::<PinpointClient>()?;
        let items = client.get_campaigns(&application_id).await?;
        Ok(items.into_iter().map(PinpointCampaign::from).collect())
    }

    async fn pinpoint_segments(
        &self,
        ctx: &Context<'_>,
        application_id: String,
    ) -> Result<Vec<PinpointSegment>> {
        let client = ctx.data::<PinpointClient>()?;
        let items = client.get_segments(&application_id).await?;
        Ok(items.into_iter().map(PinpointSegment::from).collect())
    }
}
