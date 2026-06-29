use async_graphql::SimpleObject;

use crate::aws::fsx::{FsxBackupInfo, FsxFileSystemInfo, FsxStorageVirtualMachineInfo};
use crate::schema::ec2::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct FsxFileSystem {
    pub file_system_id: String,
    pub file_system_type: String,
    pub lifecycle: String,
    pub storage_capacity_gi_b: Option<i32>,
    pub storage_type: Option<String>,
    pub vpc_id: Option<String>,
    pub subnet_ids: Vec<String>,
    pub dns_name: Option<String>,
    pub kms_key_id: Option<String>,
    pub creation_time: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<FsxFileSystemInfo> for FsxFileSystem {
    fn from(info: FsxFileSystemInfo) -> Self {
        Self {
            file_system_id: info.file_system_id,
            file_system_type: info.file_system_type,
            lifecycle: info.lifecycle,
            storage_capacity_gi_b: info.storage_capacity_gi_b,
            storage_type: info.storage_type,
            vpc_id: info.vpc_id,
            subnet_ids: info.subnet_ids,
            dns_name: info.dns_name,
            kms_key_id: info.kms_key_id,
            creation_time: info.creation_time,
            tags: info.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct FsxBackup {
    pub backup_id: String,
    pub lifecycle: String,
    pub backup_type: String,
    pub creation_time: Option<String>,
    pub file_system_id: Option<String>,
    pub resource_arn: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<FsxBackupInfo> for FsxBackup {
    fn from(info: FsxBackupInfo) -> Self {
        Self {
            backup_id: info.backup_id,
            lifecycle: info.lifecycle,
            backup_type: info.backup_type,
            creation_time: info.creation_time,
            file_system_id: info.file_system_id,
            resource_arn: info.resource_arn,
            tags: info.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct FsxStorageVirtualMachine {
    pub storage_virtual_machine_id: String,
    pub name: Option<String>,
    pub file_system_id: String,
    pub lifecycle: String,
    pub subtype: Option<String>,
    pub creation_time: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<FsxStorageVirtualMachineInfo> for FsxStorageVirtualMachine {
    fn from(info: FsxStorageVirtualMachineInfo) -> Self {
        Self {
            storage_virtual_machine_id: info.storage_virtual_machine_id,
            name: info.name,
            file_system_id: info.file_system_id,
            lifecycle: info.lifecycle,
            subtype: info.subtype,
            creation_time: info.creation_time,
            tags: info.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::fsx::{FsxBackupInfo, FsxFileSystemInfo, FsxStorageVirtualMachineInfo};

    #[test]
    fn test_fsx_file_system_from_minimal() {
        let info = FsxFileSystemInfo {
            file_system_id: "fs-0123456789abcdef0".to_string(),
            file_system_type: "WINDOWS".to_string(),
            lifecycle: "AVAILABLE".to_string(),
            storage_capacity_gi_b: None,
            storage_type: None,
            vpc_id: None,
            subnet_ids: vec![],
            dns_name: None,
            kms_key_id: None,
            creation_time: None,
            tags: vec![],
        };
        let result = FsxFileSystem::from(info);
        assert_eq!(result.file_system_id, "fs-0123456789abcdef0");
        assert_eq!(result.file_system_type, "WINDOWS");
        assert_eq!(result.lifecycle, "AVAILABLE");
        assert!(result.storage_capacity_gi_b.is_none());
        assert!(result.vpc_id.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_fsx_file_system_from_full() {
        let info = FsxFileSystemInfo {
            file_system_id: "fs-abc123".to_string(),
            file_system_type: "LUSTRE".to_string(),
            lifecycle: "AVAILABLE".to_string(),
            storage_capacity_gi_b: Some(1200),
            storage_type: Some("SSD".to_string()),
            vpc_id: Some("vpc-12345678".to_string()),
            subnet_ids: vec!["subnet-aaa".to_string()],
            dns_name: Some("fs-abc123.fsx.us-east-1.amazonaws.com".to_string()),
            kms_key_id: Some("arn:aws:kms:us-east-1:123456789012:key/key-id".to_string()),
            creation_time: Some("2024-01-01T00:00:00Z".to_string()),
            tags: vec![("Name".to_string(), "my-fsx".to_string())],
        };
        let result = FsxFileSystem::from(info);
        assert_eq!(result.storage_capacity_gi_b, Some(1200));
        assert_eq!(result.storage_type, Some("SSD".to_string()));
        assert_eq!(result.vpc_id, Some("vpc-12345678".to_string()));
        assert_eq!(result.subnet_ids, vec!["subnet-aaa"]);
        assert_eq!(result.dns_name, Some("fs-abc123.fsx.us-east-1.amazonaws.com".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Name");
        assert_eq!(result.tags[0].value, "my-fsx");
    }

    #[test]
    fn test_fsx_backup_from_minimal() {
        let info = FsxBackupInfo {
            backup_id: "backup-0123456789abcdef0".to_string(),
            lifecycle: "AVAILABLE".to_string(),
            backup_type: "AUTOMATIC".to_string(),
            creation_time: None,
            file_system_id: None,
            resource_arn: None,
            tags: vec![],
        };
        let result = FsxBackup::from(info);
        assert_eq!(result.backup_id, "backup-0123456789abcdef0");
        assert_eq!(result.lifecycle, "AVAILABLE");
        assert_eq!(result.backup_type, "AUTOMATIC");
        assert!(result.file_system_id.is_none());
        assert!(result.resource_arn.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_fsx_backup_from_full() {
        let info = FsxBackupInfo {
            backup_id: "backup-abc123".to_string(),
            lifecycle: "AVAILABLE".to_string(),
            backup_type: "USER_INITIATED".to_string(),
            creation_time: Some("2024-06-01T00:00:00Z".to_string()),
            file_system_id: Some("fs-abc123".to_string()),
            resource_arn: Some("arn:aws:fsx:us-east-1:123456789012:backup/backup-abc123".to_string()),
            tags: vec![("Env".to_string(), "prod".to_string())],
        };
        let result = FsxBackup::from(info);
        assert_eq!(result.creation_time, Some("2024-06-01T00:00:00Z".to_string()));
        assert_eq!(result.file_system_id, Some("fs-abc123".to_string()));
        assert_eq!(result.resource_arn, Some("arn:aws:fsx:us-east-1:123456789012:backup/backup-abc123".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Env");
    }

    #[test]
    fn test_fsx_storage_virtual_machine_from() {
        let info = FsxStorageVirtualMachineInfo {
            storage_virtual_machine_id: "svm-0123456789abcdef0".to_string(),
            name: Some("my-svm".to_string()),
            file_system_id: "fs-abc123".to_string(),
            lifecycle: "CREATED".to_string(),
            subtype: Some("DEFAULT".to_string()),
            creation_time: Some("2024-01-01T00:00:00Z".to_string()),
            tags: vec![("Project".to_string(), "vapor".to_string())],
        };
        let result = FsxStorageVirtualMachine::from(info);
        assert_eq!(result.storage_virtual_machine_id, "svm-0123456789abcdef0");
        assert_eq!(result.name, Some("my-svm".to_string()));
        assert_eq!(result.file_system_id, "fs-abc123");
        assert_eq!(result.lifecycle, "CREATED");
        assert_eq!(result.subtype, Some("DEFAULT".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Project");
        assert_eq!(result.tags[0].value, "vapor");
    }

    #[test]
    fn test_fsx_storage_virtual_machine_minimal() {
        let info = FsxStorageVirtualMachineInfo {
            storage_virtual_machine_id: "svm-abc".to_string(),
            name: None,
            file_system_id: "fs-abc".to_string(),
            lifecycle: "PENDING".to_string(),
            subtype: None,
            creation_time: None,
            tags: vec![],
        };
        let result = FsxStorageVirtualMachine::from(info);
        assert!(result.name.is_none());
        assert!(result.subtype.is_none());
        assert!(result.creation_time.is_none());
        assert!(result.tags.is_empty());
    }
}
