use aws_config::SdkConfig;

use crate::error::VaporError;

#[derive(Debug)]
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
    pub creation_time: Option<aws_smithy_types::DateTime>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct FsxBackupInfo {
    pub backup_id: String,
    pub lifecycle: String,
    pub backup_type: String,
    pub creation_time: Option<aws_smithy_types::DateTime>,
    pub file_system_id: Option<String>,
    pub resource_arn: Option<String>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct FsxStorageVirtualMachineInfo {
    pub storage_virtual_machine_id: String,
    pub name: Option<String>,
    pub file_system_id: String,
    pub lifecycle: String,
    pub subtype: Option<String>,
    pub creation_time: Option<aws_smithy_types::DateTime>,
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

    /// Describes file systems, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeFileSystems` has
    /// both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-fsx` 1.115.0's
    /// `operation/describe_file_systems/_describe_file_systems_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself,
    /// matching `mq.rs`'s `list_configurations` pattern.
    pub async fn describe_file_systems(
        &self,
        file_system_ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<FsxFileSystemInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_file_systems();
            if let Some(ref ids) = file_system_ids {
                req = req.set_file_system_ids(Some(ids.clone()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for fs in output.file_systems.unwrap_or_default() {
                let tags: Vec<(String, String)> = fs
                    .tags
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| (t.key.unwrap_or_default(), t.value.unwrap_or_default()))
                    .collect();
                items.push(FsxFileSystemInfo {
                    file_system_id: fs.file_system_id.unwrap_or_default(),
                    file_system_type: fs.file_system_type.map(|t| t.as_str().to_string()).unwrap_or_default(),
                    lifecycle: fs.lifecycle.map(|l| l.as_str().to_string()).unwrap_or_default(),
                    storage_capacity_gi_b: fs.storage_capacity,
                    storage_type: fs.storage_type.map(|s| s.as_str().to_string()),
                    vpc_id: fs.vpc_id,
                    subnet_ids: fs.subnet_ids.unwrap_or_default(),
                    dns_name: fs.dns_name,
                    kms_key_id: fs.kms_key_id,
                    creation_time: fs.creation_time,
                    tags,
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Describes backups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeBackups` has both
    /// `max_results` and `next_token` (verified against pinned `aws-sdk-fsx`
    /// 1.115.0's `operation/describe_backups/_describe_backups_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself,
    /// matching `mq.rs`'s `list_configurations` pattern.
    pub async fn describe_backups(
        &self,
        backup_ids: Option<Vec<String>>,
        file_system_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<FsxBackupInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

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
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for backup in output.backups.unwrap_or_default() {
                let tags: Vec<(String, String)> = backup
                    .tags
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| (t.key.unwrap_or_default(), t.value.unwrap_or_default()))
                    .collect();
                items.push(FsxBackupInfo {
                    backup_id: backup.backup_id.unwrap_or_default(),
                    lifecycle: backup.lifecycle.map(|l| l.as_str().to_string()).unwrap_or_default(),
                    backup_type: backup.r#type.map(|t| t.as_str().to_string()).unwrap_or_default(),
                    creation_time: backup.creation_time,
                    file_system_id: backup.file_system.and_then(|fs| fs.file_system_id),
                    resource_arn: backup.resource_arn,
                    tags,
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Describes storage virtual machines, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `DescribeStorageVirtualMachines` has both `max_results` and
    /// `next_token` (verified against pinned `aws-sdk-fsx` 1.115.0's
    /// `operation/describe_storage_virtual_machines/
    /// _describe_storage_virtual_machines_input.rs`), so `limit` is capped
    /// to the remaining budget on the request itself, matching `mq.rs`'s
    /// `list_configurations` pattern.
    pub async fn describe_storage_virtual_machines(
        &self,
        file_system_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<FsxStorageVirtualMachineInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

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
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for svm in output.storage_virtual_machines.unwrap_or_default() {
                let tags: Vec<(String, String)> = svm
                    .tags
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| (t.key.unwrap_or_default(), t.value.unwrap_or_default()))
                    .collect();
                items.push(FsxStorageVirtualMachineInfo {
                    storage_virtual_machine_id: svm.storage_virtual_machine_id.unwrap_or_default(),
                    name: svm.name,
                    file_system_id: svm.file_system_id.unwrap_or_default(),
                    lifecycle: svm.lifecycle.map(|l| l.as_str().to_string()).unwrap_or_default(),
                    subtype: svm.subtype.map(|s| s.as_str().to_string()),
                    creation_time: svm.creation_time,
                    tags,
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::error::VaporError;

    const ENDPOINT: &str = "https://fsx.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_file_systems_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"FileSystems":[{"FileSystemId":"fs-1","FileSystemType":"LUSTRE","Lifecycle":"AVAILABLE","StorageCapacity":1200,"StorageType":"SSD","VpcId":"vpc-1","SubnetIds":["subnet-1"],"DNSName":"fs-1.fsx.us-east-1.amazonaws.com","KmsKeyId":"key-1","CreationTime":1700000000,"Tags":[{"Key":"env","Value":"prod"}]},{"FileSystemId":"fs-2","FileSystemType":"WINDOWS","Lifecycle":"CREATING"}]}"#,
            ),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (systems, token) = client
            .describe_file_systems(None, None, None)
            .await
            .unwrap();

        assert_eq!(systems.len(), 2);
        assert_eq!(systems[0].file_system_id, "fs-1");
        assert_eq!(systems[0].file_system_type, "LUSTRE");
        assert_eq!(systems[0].lifecycle, "AVAILABLE");
        assert_eq!(systems[0].storage_capacity_gi_b, Some(1200));
        assert_eq!(systems[0].storage_type, Some("SSD".to_string()));
        assert_eq!(systems[0].vpc_id, Some("vpc-1".to_string()));
        assert_eq!(systems[0].subnet_ids, vec!["subnet-1".to_string()]);
        assert_eq!(
            systems[0].dns_name,
            Some("fs-1.fsx.us-east-1.amazonaws.com".to_string())
        );
        assert_eq!(systems[0].kms_key_id, Some("key-1".to_string()));
        assert!(systems[0].creation_time.is_some());
        assert_eq!(
            systems[0].tags,
            vec![("env".to_string(), "prod".to_string())]
        );
        assert_eq!(systems[1].file_system_id, "fs-2");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_passes_file_system_ids_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"FileSystemIds":["fs-1"]}"#),
            json_response(200, r#"{"FileSystems":[{"FileSystemId":"fs-1"}]}"#),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (systems, _token) = client
            .describe_file_systems(Some(vec!["fs-1".to_string()]), None, None)
            .await
            .unwrap();

        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].file_system_id, "fs-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"FileSystems":[{"FileSystemId":"fs-3"}]}"#),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (systems, token) = client
            .describe_file_systems(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].file_system_id, "fs-3");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"FileSystems":[{"FileSystemId":"fs-a"},{"FileSystemId":"fs-b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (systems, token) = client
            .describe_file_systems(None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(systems.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"FileSystems":[{"FileSystemId":"fs-a"},{"FileSystemId":"fs-b"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"FileSystems":[{"FileSystemId":"fs-c"}]}"#),
            ),
        ]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (systems, token) = client
            .describe_file_systems(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(systems.len(), 3);
        assert_eq!(systems[2].file_system_id, "fs-c");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InternalServerError", "internal failure"),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_file_systems(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InternalServerError"));
                assert_eq!(message, "internal failure");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_backups_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Backups":[{"BackupId":"backup-1","Lifecycle":"AVAILABLE","Type":"USER_INITIATED","CreationTime":1700000000,"ResourceARN":"arn:aws:fsx:us-east-1:1:backup/backup-1","FileSystem":{"FileSystemId":"fs-1"},"Tags":[{"Key":"env","Value":"prod"}]},{"BackupId":"backup-2","Lifecycle":"CREATING","Type":"AUTOMATIC"}]}"#,
            ),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (backups, token) = client
            .describe_backups(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(backups.len(), 2);
        assert_eq!(backups[0].backup_id, "backup-1");
        assert_eq!(backups[0].lifecycle, "AVAILABLE");
        assert_eq!(backups[0].backup_type, "USER_INITIATED");
        assert!(backups[0].creation_time.is_some());
        assert_eq!(
            backups[0].resource_arn,
            Some("arn:aws:fsx:us-east-1:1:backup/backup-1".to_string())
        );
        assert_eq!(backups[0].file_system_id, Some("fs-1".to_string()));
        assert_eq!(
            backups[0].tags,
            vec![("env".to_string(), "prod".to_string())]
        );
        assert_eq!(backups[1].backup_id, "backup-2");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_backups_passes_backup_ids_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"BackupIds":["backup-1"]}"#),
            json_response(200, r#"{"Backups":[{"BackupId":"backup-1"}]}"#),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (backups, _token) = client
            .describe_backups(Some(vec!["backup-1".to_string()]), None, None, None)
            .await
            .unwrap();

        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].backup_id, "backup-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_backups_passes_file_system_id_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"Filters":[{"Name":"file-system-id","Values":["fs-1"]}]}"#,
            ),
            json_response(200, r#"{"Backups":[{"BackupId":"backup-3"}]}"#),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (backups, _token) = client
            .describe_backups(None, Some("fs-1".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].backup_id, "backup-3");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_backups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Backups":[{"BackupId":"backup-4"}]}"#),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (backups, token) = client
            .describe_backups(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].backup_id, "backup-4");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_backups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Backups":[{"BackupId":"backup-a"},{"BackupId":"backup-b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (backups, token) = client.describe_backups(None, None, Some(2), None).await.unwrap();

        assert_eq!(backups.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_backups_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("BadRequest", "invalid backup request"),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_backups(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("BadRequest"));
                assert_eq!(message, "invalid backup request");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_storage_virtual_machines_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"StorageVirtualMachines":[{"StorageVirtualMachineId":"svm-1","Name":"svm-one","FileSystemId":"fs-1","Lifecycle":"CREATED","Subtype":"DEFAULT","CreationTime":1700000000,"Tags":[{"Key":"env","Value":"prod"}]},{"StorageVirtualMachineId":"svm-2","FileSystemId":"fs-1","Lifecycle":"CREATING"}]}"#,
            ),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (svms, token) = client
            .describe_storage_virtual_machines(None, None, None)
            .await
            .unwrap();

        assert_eq!(svms.len(), 2);
        assert_eq!(svms[0].storage_virtual_machine_id, "svm-1");
        assert_eq!(svms[0].name, Some("svm-one".to_string()));
        assert_eq!(svms[0].file_system_id, "fs-1");
        assert_eq!(svms[0].lifecycle, "CREATED");
        assert_eq!(svms[0].subtype, Some("DEFAULT".to_string()));
        assert!(svms[0].creation_time.is_some());
        assert_eq!(
            svms[0].tags,
            vec![("env".to_string(), "prod".to_string())]
        );
        assert_eq!(svms[1].storage_virtual_machine_id, "svm-2");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_storage_virtual_machines_passes_file_system_id_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"Filters":[{"Name":"file-system-id","Values":["fs-1"]}]}"#,
            ),
            json_response(
                200,
                r#"{"StorageVirtualMachines":[{"StorageVirtualMachineId":"svm-3"}]}"#,
            ),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (svms, _token) = client
            .describe_storage_virtual_machines(Some("fs-1".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(svms.len(), 1);
        assert_eq!(svms[0].storage_virtual_machine_id, "svm-3");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_storage_virtual_machines_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(
                200,
                r#"{"StorageVirtualMachines":[{"StorageVirtualMachineId":"svm-4"}]}"#,
            ),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (svms, token) = client
            .describe_storage_virtual_machines(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(svms.len(), 1);
        assert_eq!(svms[0].storage_virtual_machine_id, "svm-4");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_storage_virtual_machines_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"StorageVirtualMachines":[{"StorageVirtualMachineId":"svm-a"},{"StorageVirtualMachineId":"svm-b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let (svms, token) = client
            .describe_storage_virtual_machines(None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(svms.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_storage_virtual_machines_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InternalServerError", "internal failure"),
        )]);
        let client = FsxClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_storage_virtual_machines(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InternalServerError"));
                assert_eq!(message, "internal failure");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
