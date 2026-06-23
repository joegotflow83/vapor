use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
#[graphql(name = "EfsTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct EfsFileSystem {
    pub file_system_id: String,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub life_cycle_state: Option<String>,
    pub performance_mode: Option<String>,
    pub throughput_mode: Option<String>,
    pub provisioned_throughput: Option<f64>,
    pub size_in_bytes: Option<i64>,
    pub encrypted: bool,
    pub kms_key_id: Option<String>,
    pub number_of_mount_targets: i32,
    pub creation_time: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<&aws_sdk_efs::types::FileSystemDescription> for EfsFileSystem {
    fn from(fs: &aws_sdk_efs::types::FileSystemDescription) -> Self {
        let name = fs.name().map(|s| s.to_string());
        let tags: Vec<Tag> = fs
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().to_string(),
                value: t.value().to_string(),
            })
            .collect();

        Self {
            file_system_id: fs.file_system_id().to_string(),
            arn: fs.file_system_arn().map(|s| s.to_string()),
            name,
            life_cycle_state: Some(fs.life_cycle_state().as_str().to_string()),
            performance_mode: Some(fs.performance_mode().as_str().to_string()),
            throughput_mode: fs.throughput_mode().map(|m| m.as_str().to_string()),
            provisioned_throughput: fs.provisioned_throughput_in_mibps(),
            size_in_bytes: fs.size_in_bytes().map(|s| s.value()),
            encrypted: fs.encrypted().unwrap_or(false),
            kms_key_id: fs.kms_key_id().map(|s| s.to_string()),
            number_of_mount_targets: fs.number_of_mount_targets() as i32,
            creation_time: Some(fs.creation_time().to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct EfsMountTarget {
    pub mount_target_id: String,
    pub file_system_id: String,
    pub subnet_id: Option<String>,
    pub ip_address: Option<String>,
    pub life_cycle_state: Option<String>,
    pub availability_zone: Option<String>,
}

impl From<&aws_sdk_efs::types::MountTargetDescription> for EfsMountTarget {
    fn from(mt: &aws_sdk_efs::types::MountTargetDescription) -> Self {
        Self {
            mount_target_id: mt.mount_target_id().to_string(),
            file_system_id: mt.file_system_id().to_string(),
            subnet_id: Some(mt.subnet_id().to_string()),
            ip_address: mt.ip_address().map(|s| s.to_string()),
            life_cycle_state: Some(mt.life_cycle_state().as_str().to_string()),
            availability_zone: mt.availability_zone_name().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct EfsAccessPoint {
    pub access_point_id: String,
    pub arn: Option<String>,
    pub file_system_id: String,
    pub name: Option<String>,
    pub life_cycle_state: Option<String>,
    pub root_directory_path: Option<String>,
    pub posix_uid: Option<i64>,
    pub posix_gid: Option<i64>,
}

impl From<&aws_sdk_efs::types::AccessPointDescription> for EfsAccessPoint {
    fn from(ap: &aws_sdk_efs::types::AccessPointDescription) -> Self {
        Self {
            access_point_id: ap.access_point_id().unwrap_or_default().to_string(),
            arn: ap.access_point_arn().map(|s| s.to_string()),
            file_system_id: ap.file_system_id().unwrap_or_default().to_string(),
            name: ap.name().map(|s| s.to_string()),
            life_cycle_state: ap.life_cycle_state().map(|s| s.as_str().to_string()),
            root_directory_path: ap.root_directory().and_then(|r| r.path().map(|s| s.to_string())),
            posix_uid: ap.posix_user().map(|u| u.uid()),
            posix_gid: ap.posix_user().map(|u| u.gid()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efs_file_system_from_sdk() {
        let tag = aws_sdk_efs::types::Tag::builder()
            .key("Name")
            .value("my-fs")
            .build()
            .unwrap();
        let size = aws_sdk_efs::types::FileSystemSize::builder()
            .value(1024)
            .build();
        let fs = aws_sdk_efs::types::FileSystemDescription::builder()
            .owner_id("123456789012")
            .file_system_id("fs-12345")
            .file_system_arn("arn:aws:elasticfilesystem:us-east-1:123456789012:file-system/fs-12345")
            .name("my-fs")
            .life_cycle_state(aws_sdk_efs::types::LifeCycleState::Available)
            .performance_mode(aws_sdk_efs::types::PerformanceMode::GeneralPurpose)
            .throughput_mode(aws_sdk_efs::types::ThroughputMode::Bursting)
            .size_in_bytes(size)
            .encrypted(true)
            .kms_key_id("arn:aws:kms:us-east-1:123456789012:key/abc")
            .number_of_mount_targets(2)
            .creation_time(aws_sdk_efs::primitives::DateTime::from_secs(1_000_000))
            .creation_token("token")
            .tags(tag)
            .build()
            .unwrap();

        let result = EfsFileSystem::from(&fs);
        assert_eq!(result.file_system_id, "fs-12345");
        assert_eq!(result.arn, Some("arn:aws:elasticfilesystem:us-east-1:123456789012:file-system/fs-12345".to_string()));
        assert_eq!(result.name, Some("my-fs".to_string()));
        assert_eq!(result.life_cycle_state, Some("available".to_string()));
        assert_eq!(result.performance_mode, Some("generalPurpose".to_string()));
        assert!(result.encrypted);
        assert_eq!(result.number_of_mount_targets, 2);
        assert_eq!(result.size_in_bytes, Some(1024));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Name");
        assert_eq!(result.tags[0].value, "my-fs");
    }

    #[test]
    fn test_efs_file_system_minimal() {
        let fs = aws_sdk_efs::types::FileSystemDescription::builder()
            .owner_id("123456789012")
            .file_system_id("fs-minimal")
            .life_cycle_state(aws_sdk_efs::types::LifeCycleState::Available)
            .performance_mode(aws_sdk_efs::types::PerformanceMode::GeneralPurpose)
            .number_of_mount_targets(0)
            .creation_time(aws_sdk_efs::primitives::DateTime::from_secs(0))
            .creation_token("tok")
            .tags(aws_sdk_efs::types::Tag::builder().key("k").value("v").build().unwrap())
            .build()
            .unwrap();

        let result = EfsFileSystem::from(&fs);
        assert_eq!(result.file_system_id, "fs-minimal");
        assert!(result.name.is_none());
        assert!(!result.encrypted);
        assert!(result.kms_key_id.is_none());
        assert!(result.throughput_mode.is_none());
        assert!(result.provisioned_throughput.is_none());
    }

    #[test]
    fn test_efs_mount_target_from_sdk() {
        let mt = aws_sdk_efs::types::MountTargetDescription::builder()
            .mount_target_id("fsmt-12345")
            .file_system_id("fs-12345")
            .subnet_id("subnet-abc")
            .ip_address("10.0.0.1")
            .life_cycle_state(aws_sdk_efs::types::LifeCycleState::Available)
            .availability_zone_name("us-east-1a")
            .build()
            .unwrap();

        let result = EfsMountTarget::from(&mt);
        assert_eq!(result.mount_target_id, "fsmt-12345");
        assert_eq!(result.file_system_id, "fs-12345");
        assert_eq!(result.subnet_id, Some("subnet-abc".to_string()));
        assert_eq!(result.ip_address, Some("10.0.0.1".to_string()));
        assert_eq!(result.life_cycle_state, Some("available".to_string()));
        assert_eq!(result.availability_zone, Some("us-east-1a".to_string()));
    }

    #[test]
    fn test_efs_access_point_from_sdk() {
        let posix = aws_sdk_efs::types::PosixUser::builder()
            .uid(1000)
            .gid(1000)
            .build()
            .unwrap();
        let root_dir = aws_sdk_efs::types::RootDirectory::builder()
            .path("/data")
            .build();
        let ap = aws_sdk_efs::types::AccessPointDescription::builder()
            .access_point_id("fsap-12345")
            .access_point_arn("arn:aws:elasticfilesystem:us-east-1:123456789012:access-point/fsap-12345")
            .file_system_id("fs-12345")
            .name("my-ap")
            .life_cycle_state(aws_sdk_efs::types::LifeCycleState::Available)
            .root_directory(root_dir)
            .posix_user(posix)
            .build();

        let result = EfsAccessPoint::from(&ap);
        assert_eq!(result.access_point_id, "fsap-12345");
        assert_eq!(result.arn, Some("arn:aws:elasticfilesystem:us-east-1:123456789012:access-point/fsap-12345".to_string()));
        assert_eq!(result.file_system_id, "fs-12345");
        assert_eq!(result.name, Some("my-ap".to_string()));
        assert_eq!(result.root_directory_path, Some("/data".to_string()));
        assert_eq!(result.posix_uid, Some(1000));
        assert_eq!(result.posix_gid, Some(1000));
    }

    #[test]
    fn test_efs_access_point_minimal() {
        let ap = aws_sdk_efs::types::AccessPointDescription::builder().build();
        let result = EfsAccessPoint::from(&ap);
        assert_eq!(result.access_point_id, "");
        assert!(result.arn.is_none());
        assert!(result.name.is_none());
        assert!(result.root_directory_path.is_none());
        assert!(result.posix_uid.is_none());
        assert!(result.posix_gid.is_none());
    }
}
