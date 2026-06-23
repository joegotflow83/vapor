use async_graphql::{Context, Object, Result};

use crate::aws::backup::BackupClient;
use crate::schema::backup::types::{BackupPlan, BackupVault, RecoveryPoint};

#[derive(Default)]
pub struct BackupQuery;

#[Object]
impl BackupQuery {
    async fn backup_vaults(&self, ctx: &Context<'_>) -> Result<Vec<BackupVault>> {
        let client = ctx.data::<BackupClient>()?;
        let vaults = client.list_backup_vaults().await?;
        Ok(vaults.into_iter().map(BackupVault::from).collect())
    }

    async fn backup_plans(&self, ctx: &Context<'_>) -> Result<Vec<BackupPlan>> {
        let client = ctx.data::<BackupClient>()?;
        let plans = client.list_backup_plans().await?;
        Ok(plans.into_iter().map(BackupPlan::from).collect())
    }

    async fn backup_recovery_points(
        &self,
        ctx: &Context<'_>,
        vault_name: String,
    ) -> Result<Vec<RecoveryPoint>> {
        let client = ctx.data::<BackupClient>()?;
        let points = client
            .list_recovery_points_by_backup_vault(&vault_name)
            .await?;
        Ok(points.into_iter().map(RecoveryPoint::from).collect())
    }
}
