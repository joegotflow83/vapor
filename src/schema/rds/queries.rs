use async_graphql::{Context, Object, Result};

use crate::aws::rds::RdsClient;
use crate::schema::rds::types::{DbCluster, DbInstance, DbParameterGroup, DbSnapshot, DbSubnetGroup};

#[derive(Default)]
pub struct RdsQuery;

#[Object]
impl RdsQuery {
    async fn db_instances(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<DbInstance>> {
        let client = ctx.data::<RdsClient>()?;
        let results = client.describe_db_instances(ids).await?;
        Ok(results.into_iter().map(DbInstance::from).collect())
    }

    async fn db_clusters(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<DbCluster>> {
        let client = ctx.data::<RdsClient>()?;
        let results = client.describe_db_clusters(ids).await?;
        Ok(results.into_iter().map(DbCluster::from).collect())
    }

    async fn db_snapshots(
        &self,
        ctx: &Context<'_>,
        db_instance_id: Option<String>,
        snapshot_type: Option<String>,
    ) -> Result<Vec<DbSnapshot>> {
        let client = ctx.data::<RdsClient>()?;
        let results = client.describe_db_snapshots(db_instance_id, snapshot_type).await?;
        Ok(results.into_iter().map(DbSnapshot::from).collect())
    }

    async fn rds_parameter_groups(
        &self,
        ctx: &Context<'_>,
        name: Option<String>,
    ) -> Result<Vec<DbParameterGroup>> {
        let client = ctx.data::<RdsClient>()?;
        let results = client.describe_db_parameter_groups(name).await?;
        Ok(results.into_iter().map(DbParameterGroup::from).collect())
    }

    async fn rds_subnet_groups(
        &self,
        ctx: &Context<'_>,
        name: Option<String>,
    ) -> Result<Vec<DbSubnetGroup>> {
        let client = ctx.data::<RdsClient>()?;
        let results = client.describe_db_subnet_groups(name).await?;
        Ok(results.into_iter().map(DbSubnetGroup::from).collect())
    }
}
