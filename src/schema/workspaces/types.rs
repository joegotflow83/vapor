use async_graphql::SimpleObject;

use crate::aws::workspaces::{WorkspaceBundleInfo, WorkspaceCreationPropsInfo, WorkspaceDirectoryInfo, WorkspaceInfo};

#[derive(SimpleObject, Clone)]
pub struct Workspace {
    pub workspace_id: Option<String>,
    pub directory_id: Option<String>,
    pub user_name: Option<String>,
    pub ip_address: Option<String>,
    pub state: Option<String>,
    pub bundle_id: Option<String>,
    pub subnet_id: Option<String>,
    pub error_message: Option<String>,
    pub computer_name: Option<String>,
    pub volume_encryption_key: Option<String>,
    pub user_volume_size_gib: Option<i32>,
    pub root_volume_size_gib: Option<i32>,
}

impl From<WorkspaceInfo> for Workspace {
    fn from(i: WorkspaceInfo) -> Self {
        Self {
            workspace_id: i.workspace_id,
            directory_id: i.directory_id,
            user_name: i.user_name,
            ip_address: i.ip_address,
            state: i.state,
            bundle_id: i.bundle_id,
            subnet_id: i.subnet_id,
            error_message: i.error_message,
            computer_name: i.computer_name,
            volume_encryption_key: i.volume_encryption_key,
            user_volume_size_gib: i.user_volume_size_gib,
            root_volume_size_gib: i.root_volume_size_gib,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct WorkspaceCreationProps {
    pub enable_internet_access: Option<bool>,
    pub enable_maintenance_mode: Option<bool>,
    pub user_enabled_as_local_administrator: Option<bool>,
}

impl From<WorkspaceCreationPropsInfo> for WorkspaceCreationProps {
    fn from(i: WorkspaceCreationPropsInfo) -> Self {
        Self {
            enable_internet_access: i.enable_internet_access,
            enable_maintenance_mode: i.enable_maintenance_mode,
            user_enabled_as_local_administrator: i.user_enabled_as_local_administrator,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct WorkspaceDirectory {
    pub directory_id: Option<String>,
    pub directory_name: Option<String>,
    pub directory_type: Option<String>,
    pub dns_ip_addresses: Vec<String>,
    pub alias: Option<String>,
    pub state: Option<String>,
    pub workspace_creation_properties: Option<WorkspaceCreationProps>,
}

impl From<WorkspaceDirectoryInfo> for WorkspaceDirectory {
    fn from(i: WorkspaceDirectoryInfo) -> Self {
        Self {
            directory_id: i.directory_id,
            directory_name: i.directory_name,
            directory_type: i.directory_type,
            dns_ip_addresses: i.dns_ip_addresses,
            alias: i.alias,
            state: i.state,
            workspace_creation_properties: i.workspace_creation_properties.map(WorkspaceCreationProps::from),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct WorkspaceBundle {
    pub bundle_id: Option<String>,
    pub name: Option<String>,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub image_id: Option<String>,
    pub root_storage: Option<String>,
    pub user_storage: Option<String>,
    pub compute_type: Option<String>,
}

impl From<WorkspaceBundleInfo> for WorkspaceBundle {
    fn from(i: WorkspaceBundleInfo) -> Self {
        Self {
            bundle_id: i.bundle_id,
            name: i.name,
            owner: i.owner,
            description: i.description,
            image_id: i.image_id,
            root_storage: i.root_storage,
            user_storage: i.user_storage,
            compute_type: i.compute_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::workspaces::{WorkspaceBundleInfo, WorkspaceCreationPropsInfo, WorkspaceDirectoryInfo, WorkspaceInfo};

    #[test]
    fn test_workspace_from() {
        let info = WorkspaceInfo {
            workspace_id: Some("ws-abc123".to_string()),
            directory_id: Some("d-123456".to_string()),
            user_name: Some("jdoe".to_string()),
            ip_address: Some("10.0.0.5".to_string()),
            state: Some("AVAILABLE".to_string()),
            bundle_id: Some("wsb-abc123".to_string()),
            subnet_id: Some("subnet-abc123".to_string()),
            error_message: None,
            computer_name: Some("DESKTOP-ABC".to_string()),
            volume_encryption_key: None,
            user_volume_size_gib: Some(50),
            root_volume_size_gib: Some(80),
        };
        let result = Workspace::from(info);
        assert_eq!(result.workspace_id, Some("ws-abc123".to_string()));
        assert_eq!(result.user_name, Some("jdoe".to_string()));
        assert_eq!(result.state, Some("AVAILABLE".to_string()));
        assert_eq!(result.user_volume_size_gib, Some(50));
        assert_eq!(result.root_volume_size_gib, Some(80));
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_workspace_minimal() {
        let info = WorkspaceInfo {
            workspace_id: None,
            directory_id: None,
            user_name: None,
            ip_address: None,
            state: None,
            bundle_id: None,
            subnet_id: None,
            error_message: None,
            computer_name: None,
            volume_encryption_key: None,
            user_volume_size_gib: None,
            root_volume_size_gib: None,
        };
        let result = Workspace::from(info);
        assert!(result.workspace_id.is_none());
        assert!(result.state.is_none());
    }

    #[test]
    fn test_workspace_directory_from() {
        let info = WorkspaceDirectoryInfo {
            directory_id: Some("d-123456".to_string()),
            directory_name: Some("corp.example.com".to_string()),
            directory_type: Some("MICROSOFT_AD".to_string()),
            dns_ip_addresses: vec!["10.0.0.1".to_string(), "10.0.0.2".to_string()],
            alias: Some("corp-alias".to_string()),
            state: Some("REGISTERED".to_string()),
            workspace_creation_properties: Some(WorkspaceCreationPropsInfo {
                enable_internet_access: Some(true),
                enable_maintenance_mode: Some(false),
                user_enabled_as_local_administrator: Some(true),
            }),
        };
        let result = WorkspaceDirectory::from(info);
        assert_eq!(result.directory_id, Some("d-123456".to_string()));
        assert_eq!(result.directory_type, Some("MICROSOFT_AD".to_string()));
        assert_eq!(result.dns_ip_addresses.len(), 2);
        assert_eq!(result.state, Some("REGISTERED".to_string()));
        let props = result.workspace_creation_properties.unwrap();
        assert_eq!(props.enable_internet_access, Some(true));
        assert_eq!(props.enable_maintenance_mode, Some(false));
    }

    #[test]
    fn test_workspace_directory_no_creation_props() {
        let info = WorkspaceDirectoryInfo {
            directory_id: Some("d-000000".to_string()),
            directory_name: None,
            directory_type: None,
            dns_ip_addresses: vec![],
            alias: None,
            state: None,
            workspace_creation_properties: None,
        };
        let result = WorkspaceDirectory::from(info);
        assert!(result.workspace_creation_properties.is_none());
        assert!(result.dns_ip_addresses.is_empty());
    }

    #[test]
    fn test_workspace_bundle_from() {
        let info = WorkspaceBundleInfo {
            bundle_id: Some("wsb-abc123".to_string()),
            name: Some("Standard with Windows 10".to_string()),
            owner: Some("AMAZON".to_string()),
            description: Some("Standard bundle".to_string()),
            image_id: Some("wsi-abc123".to_string()),
            root_storage: Some("80".to_string()),
            user_storage: Some("50".to_string()),
            compute_type: Some("STANDARD".to_string()),
        };
        let result = WorkspaceBundle::from(info);
        assert_eq!(result.bundle_id, Some("wsb-abc123".to_string()));
        assert_eq!(result.owner, Some("AMAZON".to_string()));
        assert_eq!(result.compute_type, Some("STANDARD".to_string()));
        assert_eq!(result.root_storage, Some("80".to_string()));
        assert_eq!(result.user_storage, Some("50".to_string()));
    }

    #[test]
    fn test_workspace_bundle_minimal() {
        let info = WorkspaceBundleInfo {
            bundle_id: None,
            name: None,
            owner: None,
            description: None,
            image_id: None,
            root_storage: None,
            user_storage: None,
            compute_type: None,
        };
        let result = WorkspaceBundle::from(info);
        assert!(result.bundle_id.is_none());
        assert!(result.compute_type.is_none());
    }
}
