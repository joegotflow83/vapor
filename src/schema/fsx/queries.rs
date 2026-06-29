use async_graphql::{Context, Object, Result};

use crate::aws::fsx::FsxClient;
use crate::schema::fsx::types::{FsxBackup, FsxFileSystem, FsxStorageVirtualMachine};

#[derive(Default)]
pub struct FsxQuery;

#[Object]
impl FsxQuery {
    async fn fsx_file_systems(
        &self,
        ctx: &Context<'_>,
        file_system_ids: Option<Vec<String>>,
    ) -> Result<Vec<FsxFileSystem>> {
        let client = ctx.data::<FsxClient>()?;
        let items = client.describe_file_systems(file_system_ids).await?;
        Ok(items.into_iter().map(FsxFileSystem::from).collect())
    }

    async fn fsx_backups(
        &self,
        ctx: &Context<'_>,
        backup_ids: Option<Vec<String>>,
        file_system_id: Option<String>,
    ) -> Result<Vec<FsxBackup>> {
        let client = ctx.data::<FsxClient>()?;
        let items = client.describe_backups(backup_ids, file_system_id).await?;
        Ok(items.into_iter().map(FsxBackup::from).collect())
    }

    async fn fsx_storage_virtual_machines(
        &self,
        ctx: &Context<'_>,
        file_system_id: Option<String>,
    ) -> Result<Vec<FsxStorageVirtualMachine>> {
        let client = ctx.data::<FsxClient>()?;
        let items = client.describe_storage_virtual_machines(file_system_id).await?;
        Ok(items.into_iter().map(FsxStorageVirtualMachine::from).collect())
    }
}
