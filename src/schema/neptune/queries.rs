use async_graphql::{Context, Object, Result};

use crate::aws::neptune::NeptuneClient;
use crate::schema::neptune::types::{NeptuneCluster, NeptuneInstance};

#[derive(Default)]
pub struct NeptuneQuery;

#[Object]
impl NeptuneQuery {
    async fn neptune_clusters(&self, ctx: &Context<'_>) -> Result<Vec<NeptuneCluster>> {
        let client = ctx.data::<NeptuneClient>()?;
        let clusters = client.describe_db_clusters().await?;
        Ok(clusters.into_iter().map(NeptuneCluster::from).collect())
    }

    async fn neptune_instances(
        &self,
        ctx: &Context<'_>,
        cluster_id: Option<String>,
    ) -> Result<Vec<NeptuneInstance>> {
        let client = ctx.data::<NeptuneClient>()?;
        let instances = client.describe_db_instances(cluster_id).await?;
        Ok(instances.into_iter().map(NeptuneInstance::from).collect())
    }
}
