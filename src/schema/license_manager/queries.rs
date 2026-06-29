use async_graphql::{Context, Object, Result};

use crate::aws::license_manager::LicenseManagerClient;
use crate::schema::license_manager::types::{License, LicenseConfiguration, LicenseGrant};

#[derive(Default)]
pub struct LicenseManagerQuery;

#[Object]
impl LicenseManagerQuery {
    async fn license_configurations(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<LicenseConfiguration>> {
        let client = ctx.data::<LicenseManagerClient>()?;
        let items = client.list_license_configurations().await?;
        Ok(items.into_iter().map(LicenseConfiguration::from).collect())
    }

    async fn licenses(&self, ctx: &Context<'_>) -> Result<Vec<License>> {
        let client = ctx.data::<LicenseManagerClient>()?;
        let items = client.list_licenses().await?;
        Ok(items.into_iter().map(License::from).collect())
    }

    async fn license_grants(&self, ctx: &Context<'_>) -> Result<Vec<LicenseGrant>> {
        let client = ctx.data::<LicenseManagerClient>()?;
        let items = client.list_received_grants().await?;
        Ok(items.into_iter().map(LicenseGrant::from).collect())
    }
}
