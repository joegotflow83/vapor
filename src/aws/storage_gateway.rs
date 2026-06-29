use std::collections::HashMap;

use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct StorageGatewayInfo {
    pub gateway_id: Option<String>,
    pub gateway_arn: Option<String>,
    pub gateway_type: Option<String>,
    pub gateway_name: Option<String>,
    pub gateway_operational_state: Option<String>,
    pub gateway_region: Option<String>,
    pub ec2_instance_id: Option<String>,
}

pub struct StorageGatewayVolumeInfo {
    pub volume_arn: Option<String>,
    pub volume_id: Option<String>,
    pub gateway_arn: Option<String>,
    pub volume_type: Option<String>,
    pub volume_size_in_bytes: Option<i64>,
    pub volume_status: Option<String>,
}

pub struct StorageGatewayFileShareInfo {
    pub file_share_arn: Option<String>,
    pub file_share_id: Option<String>,
    pub file_share_type: Option<String>,
    pub gateway_arn: Option<String>,
    pub path: Option<String>,
    pub file_share_status: Option<String>,
    pub location_arn: Option<String>,
}

pub struct StorageGatewayClient {
    inner: aws_sdk_storagegateway::Client,
}

impl StorageGatewayClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_storagegateway::Client::new(config),
        }
    }

    pub async fn list_gateways(&self) -> Result<Vec<StorageGatewayInfo>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_gateways();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for gw in output.gateways() {
                items.push(StorageGatewayInfo {
                    gateway_id: gw.gateway_id().map(|s| s.to_string()),
                    gateway_arn: gw.gateway_arn().map(|s| s.to_string()),
                    gateway_type: gw.gateway_type().map(|s| s.to_string()),
                    gateway_name: gw.gateway_name().map(|s| s.to_string()),
                    gateway_operational_state: gw.gateway_operational_state().map(|s| s.to_string()),
                    gateway_region: gw.ec2_instance_region().map(|s| s.to_string()),
                    ec2_instance_id: gw.ec2_instance_id().map(|s| s.to_string()),
                });
            }
            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_volumes(&self, gateway_arn: String) -> Result<Vec<StorageGatewayVolumeInfo>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_volumes().gateway_arn(&gateway_arn);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for vol in output.volume_infos() {
                items.push(StorageGatewayVolumeInfo {
                    volume_arn: vol.volume_arn().map(|s| s.to_string()),
                    volume_id: vol.volume_id().map(|s| s.to_string()),
                    gateway_arn: vol.gateway_arn().map(|s| s.to_string()),
                    volume_type: vol.volume_type().map(|s| s.to_string()),
                    volume_size_in_bytes: Some(vol.volume_size_in_bytes()),
                    volume_status: vol.volume_attachment_status().map(|s| s.to_string()),
                });
            }
            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_file_shares(&self, gateway_arn: String) -> Result<Vec<StorageGatewayFileShareInfo>, VaporError> {
        let mut summaries = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_file_shares().gateway_arn(&gateway_arn);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for share in output.file_share_info_list() {
                summaries.push((
                    share.file_share_arn().map(|s| s.to_string()),
                    share.file_share_id().map(|s| s.to_string()),
                    share.file_share_type().map(|t| t.as_str().to_string()),
                    share.gateway_arn().map(|s| s.to_string()),
                    share.file_share_status().map(|s| s.to_string()),
                ));
            }
            match output.next_marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        if summaries.is_empty() {
            return Ok(Vec::new());
        }

        // Collect ARNs by type for batch describe calls
        let mut nfs_arns: Vec<String> = Vec::new();
        let mut smb_arns: Vec<String> = Vec::new();
        for (arn, _, share_type, _, _) in &summaries {
            if let Some(arn) = arn {
                match share_type.as_deref() {
                    Some("NFS") => nfs_arns.push(arn.clone()),
                    Some("SMB") => smb_arns.push(arn.clone()),
                    _ => {}
                }
            }
        }

        // Batch describe NFS shares for path and location_arn
        let mut nfs_details: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
        if !nfs_arns.is_empty() {
            let mut req = self.inner.describe_nfs_file_shares();
            for arn in &nfs_arns {
                req = req.file_share_arn_list(arn);
            }
            if let Ok(output) = req.send().await {
                for info in output.nfs_file_share_info_list() {
                    if let Some(arn) = info.file_share_arn() {
                        nfs_details.insert(
                            arn.to_string(),
                            (
                                info.path().map(|s| s.to_string()),
                                info.location_arn().map(|s| s.to_string()),
                            ),
                        );
                    }
                }
            }
        }

        // Batch describe SMB shares for path and location_arn
        let mut smb_details: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
        if !smb_arns.is_empty() {
            let mut req = self.inner.describe_smb_file_shares();
            for arn in &smb_arns {
                req = req.file_share_arn_list(arn);
            }
            if let Ok(output) = req.send().await {
                for info in output.smb_file_share_info_list() {
                    if let Some(arn) = info.file_share_arn() {
                        smb_details.insert(
                            arn.to_string(),
                            (
                                info.path().map(|s| s.to_string()),
                                info.location_arn().map(|s| s.to_string()),
                            ),
                        );
                    }
                }
            }
        }

        let mut items = Vec::new();
        for (arn, id, share_type, gw_arn, status) in summaries {
            let (path, location_arn) = arn.as_deref()
                .and_then(|a| nfs_details.get(a).or_else(|| smb_details.get(a)))
                .cloned()
                .unwrap_or((None, None));

            items.push(StorageGatewayFileShareInfo {
                file_share_arn: arn,
                file_share_id: id,
                file_share_type: share_type,
                gateway_arn: gw_arn,
                path,
                file_share_status: status,
                location_arn,
            });
        }

        Ok(items)
    }
}
