use async_graphql::{Context, Object, Result};

use crate::aws::service_quotas::ServiceQuotasClient;
use crate::schema::service_quotas::types::ServiceQuota;

#[derive(Default)]
pub struct ServiceQuotasQuery;

#[Object]
impl ServiceQuotasQuery {
    async fn service_quotas(
        &self,
        ctx: &Context<'_>,
        service_code: String,
    ) -> Result<Vec<ServiceQuota>> {
        let client = ctx.data::<ServiceQuotasClient>()?;
        let quotas = client.list_service_quotas(&service_code).await?;
        Ok(quotas.iter().map(ServiceQuota::from).collect())
    }

    async fn service_quota_services(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let client = ctx.data::<ServiceQuotasClient>()?;
        let services = client.list_services().await?;
        Ok(services
            .iter()
            .map(|s| s.service_code().unwrap_or_default().to_string())
            .collect())
    }
}
