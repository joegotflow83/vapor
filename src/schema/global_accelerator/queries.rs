use async_graphql::{Context, Object, Result};

use crate::aws::global_accelerator::GlobalAcceleratorClient;
use crate::schema::global_accelerator::types::{Accelerator, GaEndpointGroup, GaListener};

#[derive(Default)]
pub struct GlobalAcceleratorQuery;

#[Object]
impl GlobalAcceleratorQuery {
    async fn global_accelerators(&self, ctx: &Context<'_>) -> Result<Vec<Accelerator>> {
        let client = ctx.data::<GlobalAcceleratorClient>()?;
        let accelerators = client.list_accelerators().await?;
        Ok(accelerators.into_iter().map(Accelerator::from).collect())
    }

    async fn global_accelerator_listeners(
        &self,
        ctx: &Context<'_>,
        accelerator_arn: String,
    ) -> Result<Vec<GaListener>> {
        let client = ctx.data::<GlobalAcceleratorClient>()?;
        let listeners = client.list_listeners(&accelerator_arn).await?;
        Ok(listeners.into_iter().map(GaListener::from).collect())
    }

    async fn global_accelerator_endpoint_groups(
        &self,
        ctx: &Context<'_>,
        listener_arn: String,
    ) -> Result<Vec<GaEndpointGroup>> {
        let client = ctx.data::<GlobalAcceleratorClient>()?;
        let groups = client.list_endpoint_groups(&listener_arn).await?;
        Ok(groups.into_iter().map(GaEndpointGroup::from).collect())
    }
}
