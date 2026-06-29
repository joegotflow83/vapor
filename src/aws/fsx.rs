use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct FsxFileSystemInfo {
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
    pub tags: Vec<(String, String)>,
}

pub struct FsxBackupInfo {
    pub backup_id: String,
    pub lifecycle: String,
    pub backup_type: String,
    pub creation_time: Option<String>,
    pub file_system_id: Option<String>,
    pub resource_arn: Option<String>,
    pub tags: Vec<(String, String)>,
}

pub struct FsxStorageVirtualMachineInfo {
    pub storage_virtual_machine_id: String,
    pub name: Option<String>,
    pub file_system_id: String,
    pub lifecycle: String,
    pub subtype: Option<String>,
    pub creation_time: Option<String>,
    pub tags: Vec<(String, String)>,
}

pub struct FsxClient {
    inner: aws_sdk_fsx::Client,
}

impl FsxClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_fsx::Client::new(config),
        }
    }

    pub async fn describe_file_systems(
        &self,
        file_system_ids: Option<Vec<String>>,
    ) -> Result<Vec<FsxFileSystemInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_file_systems();
            if let Some(ref ids) = file_system_ids {
                req = req.set_file_system_ids(Some(ids.clone()));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for fs in output.file_systems() {
                let tags: Vec<(String, String)> = fs
                    .tags()
                    .iter()
                    .map(|t| (
                        t.key().unwrap_or_default().to_string(),
                        t.value().unwrap_or_default().to_string(),
                    ))
                    .collect();
                items.push(FsxFileSystemInfo {
                    file_system_id: fs.file_system_id().unwrap_or_default().to_string(),
                    file_system_type: fs.file_system_type().map(|t| t.as_str()).unwrap_or_default().to_string(),
                    lifecycle: fs.lifecycle().map(|l| l.as_str()).unwrap_or_default().to_string(),
                    storage_capacity_gi_b: fs.storage_capacity(),
                    storage_type: fs.storage_type().map(|s| s.as_str().to_string()),
                    vpc_id: fs.vpc_id().map(|s| s.to_string()),
                    subnet_ids: fs.subnet_ids().to_vec(),
                    dns_name: fs.dns_name().map(|s| s.to_string()),
                    kms_key_id: fs.kms_key_id().map(|s| s.to_string()),
                    creation_time: fs.creation_time().map(|t| t.to_string()),
                    tags,
                });
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_backups(
        &self,
        backup_ids: Option<Vec<String>>,
        file_system_id: Option<String>,
    ) -> Result<Vec<FsxBackupInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_backups();
            if let Some(ref ids) = backup_ids {
                req = req.set_backup_ids(Some(ids.clone()));
            }
            if let Some(ref fs_id) = file_system_id {
                req = req.set_filters(Some(vec![
                    aws_sdk_fsx::types::Filter::builder()
                        .name(aws_sdk_fsx::types::FilterName::FileSystemId)
                        .set_values(Some(vec![fs_id.clone()]))
                        .build(),
                ]));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for backup in output.backups() {
                let tags: Vec<(String, String)> = backup
                    .tags()
                    .iter()
                    .map(|t| (
                        t.key().unwrap_or_default().to_string(),
                        t.value().unwrap_or_default().to_string(),
                    ))
                    .collect();
                items.push(FsxBackupInfo {
                    backup_id: backup.backup_id().unwrap_or_default().to_string(),
                    lifecycle: backup.lifecycle().map(|l| l.as_str()).unwrap_or_default().to_string(),
                    backup_type: backup.r#type().map(|t| t.as_str()).unwrap_or_default().to_string(),
                    creation_time: backup.creation_time().map(|t| t.to_string()),
                    file_system_id: backup.file_system().and_then(|fs| fs.file_system_id()).map(|s| s.to_string()),
                    resource_arn: backup.resource_arn().map(|s| s.to_string()),
                    tags,
                });
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_storage_virtual_machines(
        &self,
        file_system_id: Option<String>,
    ) -> Result<Vec<FsxStorageVirtualMachineInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_storage_virtual_machines();
            if let Some(ref fs_id) = file_system_id {
                req = req.set_filters(Some(vec![
                    aws_sdk_fsx::types::StorageVirtualMachineFilter::builder()
                        .name(aws_sdk_fsx::types::StorageVirtualMachineFilterName::FileSystemId)
                        .set_values(Some(vec![fs_id.clone()]))
                        .build(),
                ]));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for svm in output.storage_virtual_machines() {
                let tags: Vec<(String, String)> = svm
                    .tags()
                    .iter()
                    .map(|t| (
                        t.key().unwrap_or_default().to_string(),
                        t.value().unwrap_or_default().to_string(),
                    ))
                    .collect();
                items.push(FsxStorageVirtualMachineInfo {
                    storage_virtual_machine_id: svm.storage_virtual_machine_id().unwrap_or_default().to_string(),
                    name: svm.name().map(|s| s.to_string()),
                    file_system_id: svm.file_system_id().unwrap_or_default().to_string(),
                    lifecycle: svm.lifecycle().map(|l| l.as_str()).unwrap_or_default().to_string(),
                    subtype: svm.subtype().map(|s| s.as_str().to_string()),
                    creation_time: svm.creation_time().map(|t| t.to_string()),
                    tags,
                });
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
