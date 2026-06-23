use async_graphql::{Context, Object, Result};

use crate::aws::elasticache::ElastiCacheClient;
use crate::schema::elasticache::types::{
    ElastiCacheCluster, ElastiCacheReplicationGroup, ElastiCacheSubnetGroup,
};

#[derive(Default)]
pub struct ElastiCacheQuery;

#[Object]
impl ElastiCacheQuery {
    /// List ElastiCache clusters. Optionally filter by clusterId.
    async fn elasticache_clusters(
        &self,
        ctx: &Context<'_>,
        cluster_id: Option<String>,
    ) -> Result<Vec<ElastiCacheCluster>> {
        let client = ctx.data::<ElastiCacheClient>()?;
        let results = client.describe_cache_clusters(cluster_id.as_deref()).await?;
        Ok(results
            .iter()
            .map(|(c, tags)| ElastiCacheCluster::from_sdk(c, tags))
            .collect())
    }

    /// List ElastiCache replication groups. Optionally filter by replicationGroupId.
    async fn elasticache_replication_groups(
        &self,
        ctx: &Context<'_>,
        replication_group_id: Option<String>,
    ) -> Result<Vec<ElastiCacheReplicationGroup>> {
        let client = ctx.data::<ElastiCacheClient>()?;
        let results = client
            .describe_replication_groups(replication_group_id.as_deref())
            .await?;
        Ok(results.iter().map(ElastiCacheReplicationGroup::from).collect())
    }

    /// List all ElastiCache subnet groups.
    async fn elasticache_subnet_groups(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<ElastiCacheSubnetGroup>> {
        let client = ctx.data::<ElastiCacheClient>()?;
        let results = client.describe_cache_subnet_groups().await?;
        Ok(results.iter().map(ElastiCacheSubnetGroup::from).collect())
    }
}
