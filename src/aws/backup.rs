use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct BackupClient {
    inner: aws_sdk_backup::Client,
}

impl BackupClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_backup::Client::new(config),
        }
    }

    pub async fn list_backup_vaults(
        &self,
    ) -> Result<Vec<aws_sdk_backup::types::BackupVaultListMember>, VaporError> {
        let mut vaults = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_backup_vaults();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            vaults.extend(output.backup_vault_list().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(vaults)
    }

    pub async fn list_backup_plans(
        &self,
    ) -> Result<Vec<aws_sdk_backup::types::BackupPlansListMember>, VaporError> {
        let mut plans = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_backup_plans();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            plans.extend(output.backup_plans_list().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(plans)
    }

    pub async fn list_recovery_points_by_backup_vault(
        &self,
        vault_name: &str,
    ) -> Result<Vec<aws_sdk_backup::types::RecoveryPointByBackupVault>, VaporError> {
        let mut points = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_recovery_points_by_backup_vault()
                .backup_vault_name(vault_name);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            points.extend(output.recovery_points().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(points)
    }
}
