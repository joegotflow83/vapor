use async_graphql::{Context, Object, Result};

use crate::aws::efs::EfsClient;
use crate::schema::efs::types::{EfsAccessPoint, EfsFileSystem, EfsMountTarget};

#[derive(Default)]
pub struct EfsQuery;

#[Object]
impl EfsQuery {
    async fn efs_file_systems(&self, ctx: &Context<'_>) -> Result<Vec<EfsFileSystem>> {
        let client = ctx.data::<EfsClient>()?;
        let file_systems = client.describe_file_systems().await?;
        Ok(file_systems.iter().map(EfsFileSystem::from).collect())
    }

    async fn efs_mount_targets(
        &self,
        ctx: &Context<'_>,
        file_system_id: String,
    ) -> Result<Vec<EfsMountTarget>> {
        let client = ctx.data::<EfsClient>()?;
        let targets = client.describe_mount_targets(&file_system_id).await?;
        Ok(targets.iter().map(EfsMountTarget::from).collect())
    }

    async fn efs_access_points(
        &self,
        ctx: &Context<'_>,
        file_system_id: Option<String>,
    ) -> Result<Vec<EfsAccessPoint>> {
        let client = ctx.data::<EfsClient>()?;
        let points = client
            .describe_access_points(file_system_id.as_deref())
            .await?;
        Ok(points.iter().map(EfsAccessPoint::from).collect())
    }
}
