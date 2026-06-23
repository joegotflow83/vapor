use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct BackupVault {
    pub name: String,
    pub arn: Option<String>,
    pub recovery_points: i64,
    pub encryption_key_arn: Option<String>,
    pub creation_date: Option<String>,
    pub locked: bool,
}

#[derive(SimpleObject, Clone)]
pub struct BackupPlan {
    pub plan_id: String,
    pub plan_name: Option<String>,
    pub arn: Option<String>,
    pub version_id: Option<String>,
    pub creation_date: Option<String>,
    pub last_execution_date: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct RecoveryPoint {
    pub recovery_point_arn: String,
    pub resource_arn: Option<String>,
    pub resource_type: Option<String>,
    pub status: Option<String>,
    pub creation_date: Option<String>,
    pub completion_date: Option<String>,
    pub backup_size_bytes: Option<i64>,
    pub encrypted: bool,
}

impl From<aws_sdk_backup::types::BackupVaultListMember> for BackupVault {
    fn from(v: aws_sdk_backup::types::BackupVaultListMember) -> Self {
        Self {
            name: v.backup_vault_name().unwrap_or_default().to_string(),
            arn: v.backup_vault_arn().map(|s| s.to_string()),
            recovery_points: v.number_of_recovery_points(),
            encryption_key_arn: v.encryption_key_arn().map(|s| s.to_string()),
            creation_date: v.creation_date().map(|t| t.to_string()),
            locked: v.locked().unwrap_or(false),
        }
    }
}

impl From<aws_sdk_backup::types::BackupPlansListMember> for BackupPlan {
    fn from(p: aws_sdk_backup::types::BackupPlansListMember) -> Self {
        Self {
            plan_id: p.backup_plan_id().unwrap_or_default().to_string(),
            plan_name: p.backup_plan_name().map(|s| s.to_string()),
            arn: p.backup_plan_arn().map(|s| s.to_string()),
            version_id: p.version_id().map(|s| s.to_string()),
            creation_date: p.creation_date().map(|t| t.to_string()),
            last_execution_date: p.last_execution_date().map(|t| t.to_string()),
        }
    }
}

impl From<aws_sdk_backup::types::RecoveryPointByBackupVault> for RecoveryPoint {
    fn from(rp: aws_sdk_backup::types::RecoveryPointByBackupVault) -> Self {
        Self {
            recovery_point_arn: rp.recovery_point_arn().unwrap_or_default().to_string(),
            resource_arn: rp.resource_arn().map(|s| s.to_string()),
            resource_type: rp.resource_type().map(|s| s.to_string()),
            status: rp.status().map(|s| s.as_str().to_string()),
            creation_date: rp.creation_date().map(|t| t.to_string()),
            completion_date: rp.completion_date().map(|t| t.to_string()),
            backup_size_bytes: rp.backup_size_in_bytes(),
            encrypted: rp.is_encrypted(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_vault_from_sdk_minimal() {
        let v = aws_sdk_backup::types::BackupVaultListMember::builder().build();
        let result = BackupVault::from(v);
        assert_eq!(result.name, "");
        assert!(result.arn.is_none());
        assert_eq!(result.recovery_points, 0);
        assert!(result.encryption_key_arn.is_none());
        assert!(result.creation_date.is_none());
        assert!(!result.locked);
    }

    #[test]
    fn test_backup_vault_from_sdk_full() {
        let v = aws_sdk_backup::types::BackupVaultListMember::builder()
            .backup_vault_name("my-vault")
            .backup_vault_arn("arn:aws:backup:us-east-1:123456789012:backup-vault:my-vault")
            .number_of_recovery_points(42)
            .encryption_key_arn("arn:aws:kms:us-east-1:123456789012:key/abc")
            .locked(true)
            .build();
        let result = BackupVault::from(v);
        assert_eq!(result.name, "my-vault");
        assert_eq!(
            result.arn,
            Some("arn:aws:backup:us-east-1:123456789012:backup-vault:my-vault".to_string())
        );
        assert_eq!(result.recovery_points, 42);
        assert_eq!(
            result.encryption_key_arn,
            Some("arn:aws:kms:us-east-1:123456789012:key/abc".to_string())
        );
        assert!(result.locked);
    }

    #[test]
    fn test_backup_plan_from_sdk_minimal() {
        let p = aws_sdk_backup::types::BackupPlansListMember::builder().build();
        let result = BackupPlan::from(p);
        assert_eq!(result.plan_id, "");
        assert!(result.plan_name.is_none());
        assert!(result.arn.is_none());
        assert!(result.version_id.is_none());
        assert!(result.creation_date.is_none());
        assert!(result.last_execution_date.is_none());
    }

    #[test]
    fn test_backup_plan_from_sdk_full() {
        let p = aws_sdk_backup::types::BackupPlansListMember::builder()
            .backup_plan_id("plan-123")
            .backup_plan_name("DailyBackup")
            .backup_plan_arn("arn:aws:backup:us-east-1:123456789012:backup-plan:plan-123")
            .version_id("v1")
            .build();
        let result = BackupPlan::from(p);
        assert_eq!(result.plan_id, "plan-123");
        assert_eq!(result.plan_name, Some("DailyBackup".to_string()));
        assert_eq!(
            result.arn,
            Some("arn:aws:backup:us-east-1:123456789012:backup-plan:plan-123".to_string())
        );
        assert_eq!(result.version_id, Some("v1".to_string()));
    }

    #[test]
    fn test_recovery_point_from_sdk_minimal() {
        let rp = aws_sdk_backup::types::RecoveryPointByBackupVault::builder().build();
        let result = RecoveryPoint::from(rp);
        assert_eq!(result.recovery_point_arn, "");
        assert!(result.resource_arn.is_none());
        assert!(result.resource_type.is_none());
        assert!(result.status.is_none());
        assert!(result.creation_date.is_none());
        assert!(result.completion_date.is_none());
        assert!(result.backup_size_bytes.is_none());
        assert!(!result.encrypted);
    }

    #[test]
    fn test_recovery_point_from_sdk_full() {
        let rp = aws_sdk_backup::types::RecoveryPointByBackupVault::builder()
            .recovery_point_arn("arn:aws:backup:us-east-1:123456789012:recovery-point:rp-abc")
            .resource_arn("arn:aws:ec2:us-east-1:123456789012:volume/vol-abc")
            .resource_type("EBS")
            .status(aws_sdk_backup::types::RecoveryPointStatus::Completed)
            .backup_size_in_bytes(1073741824)
            .is_encrypted(true)
            .build();
        let result = RecoveryPoint::from(rp);
        assert_eq!(
            result.recovery_point_arn,
            "arn:aws:backup:us-east-1:123456789012:recovery-point:rp-abc"
        );
        assert_eq!(
            result.resource_arn,
            Some("arn:aws:ec2:us-east-1:123456789012:volume/vol-abc".to_string())
        );
        assert_eq!(result.resource_type, Some("EBS".to_string()));
        assert_eq!(result.status, Some("COMPLETED".to_string()));
        assert_eq!(result.backup_size_bytes, Some(1073741824));
        assert!(result.encrypted);
    }
}
