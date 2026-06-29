use async_graphql::SimpleObject;

use crate::aws::storage_gateway::{
    StorageGatewayFileShareInfo, StorageGatewayInfo, StorageGatewayVolumeInfo,
};

#[derive(SimpleObject, Clone)]
pub struct StorageGatewayGateway {
    pub gateway_id: Option<String>,
    pub gateway_arn: Option<String>,
    pub gateway_type: Option<String>,
    pub gateway_name: Option<String>,
    pub gateway_operational_state: Option<String>,
    pub gateway_region: Option<String>,
    pub ec2_instance_id: Option<String>,
}

impl From<StorageGatewayInfo> for StorageGatewayGateway {
    fn from(i: StorageGatewayInfo) -> Self {
        Self {
            gateway_id: i.gateway_id,
            gateway_arn: i.gateway_arn,
            gateway_type: i.gateway_type,
            gateway_name: i.gateway_name,
            gateway_operational_state: i.gateway_operational_state,
            gateway_region: i.gateway_region,
            ec2_instance_id: i.ec2_instance_id,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct StorageGatewayVolume {
    pub volume_arn: Option<String>,
    pub volume_id: Option<String>,
    pub gateway_arn: Option<String>,
    pub volume_type: Option<String>,
    pub volume_size_in_bytes: Option<i64>,
    pub volume_status: Option<String>,
}

impl From<StorageGatewayVolumeInfo> for StorageGatewayVolume {
    fn from(i: StorageGatewayVolumeInfo) -> Self {
        Self {
            volume_arn: i.volume_arn,
            volume_id: i.volume_id,
            gateway_arn: i.gateway_arn,
            volume_type: i.volume_type,
            volume_size_in_bytes: i.volume_size_in_bytes,
            volume_status: i.volume_status,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct StorageGatewayFileShare {
    pub file_share_arn: Option<String>,
    pub file_share_id: Option<String>,
    pub file_share_type: Option<String>,
    pub gateway_arn: Option<String>,
    pub path: Option<String>,
    pub file_share_status: Option<String>,
    pub location_arn: Option<String>,
}

impl From<StorageGatewayFileShareInfo> for StorageGatewayFileShare {
    fn from(i: StorageGatewayFileShareInfo) -> Self {
        Self {
            file_share_arn: i.file_share_arn,
            file_share_id: i.file_share_id,
            file_share_type: i.file_share_type,
            gateway_arn: i.gateway_arn,
            path: i.path,
            file_share_status: i.file_share_status,
            location_arn: i.location_arn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::storage_gateway::{
        StorageGatewayFileShareInfo, StorageGatewayInfo, StorageGatewayVolumeInfo,
    };

    #[test]
    fn test_gateway_from() {
        let info = StorageGatewayInfo {
            gateway_id: Some("sgw-12345678".to_string()),
            gateway_arn: Some("arn:aws:storagegateway:us-east-1:123456789012:gateway/sgw-12345678".to_string()),
            gateway_type: Some("FILE_S3".to_string()),
            gateway_name: Some("my-gateway".to_string()),
            gateway_operational_state: Some("RUNNING".to_string()),
            gateway_region: Some("us-east-1".to_string()),
            ec2_instance_id: Some("i-1234567890abcdef0".to_string()),
        };
        let result = StorageGatewayGateway::from(info);
        assert_eq!(result.gateway_id, Some("sgw-12345678".to_string()));
        assert_eq!(result.gateway_type, Some("FILE_S3".to_string()));
        assert_eq!(result.gateway_name, Some("my-gateway".to_string()));
        assert_eq!(result.gateway_operational_state, Some("RUNNING".to_string()));
        assert_eq!(result.gateway_region, Some("us-east-1".to_string()));
    }

    #[test]
    fn test_gateway_minimal() {
        let info = StorageGatewayInfo {
            gateway_id: None,
            gateway_arn: None,
            gateway_type: None,
            gateway_name: None,
            gateway_operational_state: None,
            gateway_region: None,
            ec2_instance_id: None,
        };
        let result = StorageGatewayGateway::from(info);
        assert!(result.gateway_id.is_none());
        assert!(result.gateway_type.is_none());
        assert!(result.ec2_instance_id.is_none());
    }

    #[test]
    fn test_volume_from() {
        let info = StorageGatewayVolumeInfo {
            volume_arn: Some("arn:aws:storagegateway:us-east-1:123456789012:gateway/sgw-12345678/volume/vol-12345678".to_string()),
            volume_id: Some("vol-12345678".to_string()),
            gateway_arn: Some("arn:aws:storagegateway:us-east-1:123456789012:gateway/sgw-12345678".to_string()),
            volume_type: Some("STORED".to_string()),
            volume_size_in_bytes: Some(107374182400),
            volume_status: Some("AVAILABLE".to_string()),
        };
        let result = StorageGatewayVolume::from(info);
        assert_eq!(result.volume_id, Some("vol-12345678".to_string()));
        assert_eq!(result.volume_type, Some("STORED".to_string()));
        assert_eq!(result.volume_size_in_bytes, Some(107374182400));
        assert_eq!(result.volume_status, Some("AVAILABLE".to_string()));
    }

    #[test]
    fn test_volume_minimal() {
        let info = StorageGatewayVolumeInfo {
            volume_arn: None,
            volume_id: None,
            gateway_arn: None,
            volume_type: None,
            volume_size_in_bytes: None,
            volume_status: None,
        };
        let result = StorageGatewayVolume::from(info);
        assert!(result.volume_id.is_none());
        assert!(result.volume_size_in_bytes.is_none());
    }

    #[test]
    fn test_file_share_nfs_from() {
        let info = StorageGatewayFileShareInfo {
            file_share_arn: Some("arn:aws:storagegateway:us-east-1:123456789012:share/share-12345678".to_string()),
            file_share_id: Some("share-12345678".to_string()),
            file_share_type: Some("NFS".to_string()),
            gateway_arn: Some("arn:aws:storagegateway:us-east-1:123456789012:gateway/sgw-12345678".to_string()),
            path: Some("/export".to_string()),
            file_share_status: Some("AVAILABLE".to_string()),
            location_arn: Some("arn:aws:s3:::my-bucket".to_string()),
        };
        let result = StorageGatewayFileShare::from(info);
        assert_eq!(result.file_share_id, Some("share-12345678".to_string()));
        assert_eq!(result.file_share_type, Some("NFS".to_string()));
        assert_eq!(result.path, Some("/export".to_string()));
        assert_eq!(result.file_share_status, Some("AVAILABLE".to_string()));
        assert_eq!(result.location_arn, Some("arn:aws:s3:::my-bucket".to_string()));
    }

    #[test]
    fn test_file_share_smb_from() {
        let info = StorageGatewayFileShareInfo {
            file_share_arn: Some("arn:aws:storagegateway:us-east-1:123456789012:share/share-87654321".to_string()),
            file_share_id: Some("share-87654321".to_string()),
            file_share_type: Some("SMB".to_string()),
            gateway_arn: Some("arn:aws:storagegateway:us-east-1:123456789012:gateway/sgw-12345678".to_string()),
            path: Some("/smb-export".to_string()),
            file_share_status: Some("AVAILABLE".to_string()),
            location_arn: Some("arn:aws:s3:::my-smb-bucket".to_string()),
        };
        let result = StorageGatewayFileShare::from(info);
        assert_eq!(result.file_share_type, Some("SMB".to_string()));
        assert_eq!(result.path, Some("/smb-export".to_string()));
    }

    #[test]
    fn test_file_share_minimal() {
        let info = StorageGatewayFileShareInfo {
            file_share_arn: None,
            file_share_id: None,
            file_share_type: None,
            gateway_arn: None,
            path: None,
            file_share_status: None,
            location_arn: None,
        };
        let result = StorageGatewayFileShare::from(info);
        assert!(result.file_share_id.is_none());
        assert!(result.file_share_type.is_none());
        assert!(result.location_arn.is_none());
    }
}
