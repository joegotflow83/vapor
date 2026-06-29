use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct WorkspaceInfo {
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

pub struct WorkspaceCreationPropsInfo {
    pub enable_internet_access: Option<bool>,
    pub enable_maintenance_mode: Option<bool>,
    pub user_enabled_as_local_administrator: Option<bool>,
}

pub struct WorkspaceDirectoryInfo {
    pub directory_id: Option<String>,
    pub directory_name: Option<String>,
    pub directory_type: Option<String>,
    pub dns_ip_addresses: Vec<String>,
    pub alias: Option<String>,
    pub state: Option<String>,
    pub workspace_creation_properties: Option<WorkspaceCreationPropsInfo>,
}

pub struct WorkspaceBundleInfo {
    pub bundle_id: Option<String>,
    pub name: Option<String>,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub image_id: Option<String>,
    pub root_storage: Option<String>,
    pub user_storage: Option<String>,
    pub compute_type: Option<String>,
}

pub struct WorkspacesClient {
    inner: aws_sdk_workspaces::Client,
}

impl WorkspacesClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_workspaces::Client::new(config),
        }
    }

    pub async fn describe_workspaces(
        &self,
        directory_id: Option<String>,
        user_name: Option<String>,
        bundle_id: Option<String>,
    ) -> Result<Vec<WorkspaceInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_workspaces();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            if let Some(ref d) = directory_id {
                req = req.directory_id(d);
            }
            if let Some(ref u) = user_name {
                req = req.user_name(u);
            }
            if let Some(ref b) = bundle_id {
                req = req.bundle_id(b);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for ws in output.workspaces() {
                items.push(WorkspaceInfo {
                    workspace_id: ws.workspace_id().map(|s| s.to_string()),
                    directory_id: ws.directory_id().map(|s| s.to_string()),
                    user_name: ws.user_name().map(|s| s.to_string()),
                    ip_address: ws.ip_address().map(|s| s.to_string()),
                    state: ws.state().map(|s| s.as_str().to_string()),
                    bundle_id: ws.bundle_id().map(|s| s.to_string()),
                    subnet_id: ws.subnet_id().map(|s| s.to_string()),
                    error_message: ws.error_message().map(|s| s.to_string()),
                    computer_name: ws.computer_name().map(|s| s.to_string()),
                    volume_encryption_key: ws.volume_encryption_key().map(|s| s.to_string()),
                    user_volume_size_gib: ws
                        .workspace_properties()
                        .and_then(|p| p.user_volume_size_gib()),
                    root_volume_size_gib: ws
                        .workspace_properties()
                        .and_then(|p| p.root_volume_size_gib()),
                });
            }
            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_workspace_directories(&self) -> Result<Vec<WorkspaceDirectoryInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_workspace_directories();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for dir in output.directories() {
                let creation_props = dir.workspace_creation_properties().map(|p| {
                    WorkspaceCreationPropsInfo {
                        enable_internet_access: p.enable_internet_access(),
                        enable_maintenance_mode: p.enable_maintenance_mode(),
                        user_enabled_as_local_administrator: p.user_enabled_as_local_administrator(),
                    }
                });
                let dns_ip_addresses = dir
                    .dns_ip_addresses()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                items.push(WorkspaceDirectoryInfo {
                    directory_id: dir.directory_id().map(|s| s.to_string()),
                    directory_name: dir.directory_name().map(|s| s.to_string()),
                    directory_type: dir.directory_type().map(|t| t.as_str().to_string()),
                    dns_ip_addresses,
                    alias: dir.alias().map(|s| s.to_string()),
                    state: dir.state().map(|s| s.as_str().to_string()),
                    workspace_creation_properties: creation_props,
                });
            }
            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_workspace_bundles(
        &self,
        owner: Option<String>,
    ) -> Result<Vec<WorkspaceBundleInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_workspace_bundles();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            if let Some(ref o) = owner {
                req = req.owner(o);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for bundle in output.bundles() {
                items.push(WorkspaceBundleInfo {
                    bundle_id: bundle.bundle_id().map(|s| s.to_string()),
                    name: bundle.name().map(|s| s.to_string()),
                    owner: bundle.owner().map(|s| s.to_string()),
                    description: bundle.description().map(|s| s.to_string()),
                    image_id: bundle.image_id().map(|s| s.to_string()),
                    root_storage: bundle
                        .root_storage()
                        .map(|r| r.capacity())
                        .map(|s| s.to_string()),
                    user_storage: bundle
                        .user_storage()
                        .map(|u| u.capacity())
                        .map(|s| s.to_string()),
                    compute_type: bundle
                        .compute_type()
                        .and_then(|c| c.name())
                        .map(|n| n.as_str().to_string()),
                });
            }
            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
