use async_graphql::{Context, Object, Result};

use crate::aws::redshift::RedshiftClient;
use crate::schema::redshift::types::{RedshiftCluster, RedshiftSnapshot};

#[derive(Default)]
pub struct RedshiftQuery;

#[Object]
impl RedshiftQuery {
    async fn redshift_clusters(&self, ctx: &Context<'_>) -> Result<Vec<RedshiftCluster>> {
        let client = ctx.data::<RedshiftClient>()?;
        let clusters = client.describe_clusters().await?;
        Ok(clusters.into_iter().map(RedshiftCluster::from).collect())
    }

    async fn redshift_snapshots(
        &self,
        ctx: &Context<'_>,
        cluster_identifier: Option<String>,
        snapshot_type: Option<String>,
    ) -> Result<Vec<RedshiftSnapshot>> {
        let client = ctx.data::<RedshiftClient>()?;
        let snapshots = client
            .describe_cluster_snapshots(cluster_identifier, snapshot_type)
            .await?;
        Ok(snapshots.into_iter().map(RedshiftSnapshot::from).collect())
    }
}
