use async_graphql::{Context, Object, Result};

use crate::aws::acm_pca::AcmPcaClient;
use crate::schema::acm_pca::types::PrivateCa;

#[derive(Default)]
pub struct AcmPcaQuery;

#[Object]
impl AcmPcaQuery {
    async fn private_certificate_authorities(&self, ctx: &Context<'_>) -> Result<Vec<PrivateCa>> {
        let client = ctx.data::<AcmPcaClient>()?;
        let cas = client.list_certificate_authorities().await?;
        Ok(cas.into_iter().map(PrivateCa::from).collect())
    }

    async fn private_certificate_authority(
        &self,
        ctx: &Context<'_>,
        certificate_authority_arn: String,
    ) -> Result<Option<PrivateCa>> {
        let client = ctx.data::<AcmPcaClient>()?;
        let ca = client
            .describe_certificate_authority(&certificate_authority_arn)
            .await?;
        Ok(ca.map(PrivateCa::from))
    }
}
