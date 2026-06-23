use async_graphql::{Context, Object, Result};

use crate::aws::documentdb::DocumentDbClient;
use crate::schema::documentdb::types::{DocDbCluster, DocDbInstance};

#[derive(Default)]
pub struct DocumentDbQuery;

#[Object]
impl DocumentDbQuery {
    async fn docdb_clusters(&self, ctx: &Context<'_>) -> Result<Vec<DocDbCluster>> {
        let client = ctx.data::<DocumentDbClient>()?;
        let clusters = client.describe_db_clusters().await?;
        Ok(clusters.into_iter().map(DocDbCluster::from).collect())
    }

    async fn docdb_instances(
        &self,
        ctx: &Context<'_>,
        cluster_id: Option<String>,
    ) -> Result<Vec<DocDbInstance>> {
        let client = ctx.data::<DocumentDbClient>()?;
        let instances = client.describe_db_instances(cluster_id).await?;
        Ok(instances.into_iter().map(DocDbInstance::from).collect())
    }
}
