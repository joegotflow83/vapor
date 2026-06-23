use aws_config::SdkConfig;
use aws_sdk_efs::types::{AccessPointDescription, FileSystemDescription, MountTargetDescription};

use crate::error::VaporError;

pub struct EfsClient {
    inner: aws_sdk_efs::Client,
}

impl EfsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_efs::Client::new(config),
        }
    }

    pub async fn describe_file_systems(&self) -> Result<Vec<FileSystemDescription>, VaporError> {
        let mut results = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_file_systems();
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.file_systems().to_vec());
            match output.next_marker() {
                Some(m) => next_marker = Some(m.to_string()),
                None => break,
            }
        }

        Ok(results)
    }

    pub async fn describe_mount_targets(
        &self,
        file_system_id: &str,
    ) -> Result<Vec<MountTargetDescription>, VaporError> {
        let mut results = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_mount_targets().file_system_id(file_system_id);
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.mount_targets().to_vec());
            match output.next_marker() {
                Some(m) => next_marker = Some(m.to_string()),
                None => break,
            }
        }

        Ok(results)
    }

    pub async fn describe_access_points(
        &self,
        file_system_id: Option<&str>,
    ) -> Result<Vec<AccessPointDescription>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_access_points();
            if let Some(id) = file_system_id {
                req = req.file_system_id(id);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.access_points().to_vec());
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(results)
    }
}
