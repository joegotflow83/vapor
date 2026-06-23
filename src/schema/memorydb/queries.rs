use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::memorydb::MemoryDbClient;
use crate::schema::memorydb::types::{MemoryDbCluster, MemoryDbSubnetGroup};

#[derive(Default)]
pub struct MemoryDbQuery;

#[Object]
impl MemoryDbQuery {
    async fn memorydb_clusters(&self, ctx: &Context<'_>) -> Result<Vec<MemoryDbCluster>> {
        let client = ctx.data::<MemoryDbClient>()?;
        let clusters = client.describe_clusters().await?;
        let results = join_all(clusters.into_iter().map(|c| async {
            let arn = c.arn().unwrap_or_default().to_string();
            let tags = client.list_tags(&arn).await.unwrap_or_default();
            MemoryDbCluster::from_sdk(c, &tags)
        }))
        .await;
        Ok(results)
    }

    async fn memorydb_subnet_groups(&self, ctx: &Context<'_>) -> Result<Vec<MemoryDbSubnetGroup>> {
        let client = ctx.data::<MemoryDbClient>()?;
        let groups = client.describe_subnet_groups().await?;
        let results = join_all(groups.into_iter().map(|sg| async {
            let arn = sg.arn().unwrap_or_default().to_string();
            let tags = client.list_tags(&arn).await.unwrap_or_default();
            MemoryDbSubnetGroup::from_sdk(sg, &tags)
        }))
        .await;
        Ok(results)
    }
}
