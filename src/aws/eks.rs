#[cfg(feature = "eks")]
use aws_config::SdkConfig;

#[cfg(feature = "eks")]
use crate::error::VaporError;

#[cfg(feature = "eks")]
pub struct EksClient {
    inner: aws_sdk_eks::Client,
}

impl EksClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_eks::Client::new(config),
        }
    }

    /// List all EKS cluster names with next_token pagination.
    pub async fn list_clusters(&self) -> Result<Vec<String>, VaporError> {
        let mut names = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_clusters();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            names.extend(output.clusters().iter().map(|s| s.to_string()));
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(names)
    }

    /// Describe a single EKS cluster by name. Returns None if not found.
    pub async fn describe_cluster(
        &self,
        name: &str,
    ) -> Result<Option<aws_sdk_eks::types::Cluster>, VaporError> {
        let result = self.inner.describe_cluster().name(name).send().await;
        match result {
            Ok(output) => Ok(output.cluster().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_resource_not_found_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(e.to_string()))
                }
            }
        }
    }

    /// List nodegroup names for a cluster with next_token pagination.
    pub async fn list_nodegroups(&self, cluster: &str) -> Result<Vec<String>, VaporError> {
        let mut names = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_nodegroups().cluster_name(cluster);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            names.extend(output.nodegroups().iter().map(|s| s.to_string()));
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(names)
    }

    /// Describe a single nodegroup. Returns None if not found.
    pub async fn describe_nodegroup(
        &self,
        cluster: &str,
        nodegroup: &str,
    ) -> Result<Option<aws_sdk_eks::types::Nodegroup>, VaporError> {
        let result = self
            .inner
            .describe_nodegroup()
            .cluster_name(cluster)
            .nodegroup_name(nodegroup)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.nodegroup().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_resource_not_found_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(e.to_string()))
                }
            }
        }
    }

    /// List Fargate profile names for a cluster with next_token pagination.
    pub async fn list_fargate_profiles(&self, cluster: &str) -> Result<Vec<String>, VaporError> {
        let mut names = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_fargate_profiles().cluster_name(cluster);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            names.extend(output.fargate_profile_names().iter().map(|s| s.to_string()));
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(names)
    }

    /// Describe a single Fargate profile. Returns None if not found.
    pub async fn describe_fargate_profile(
        &self,
        cluster: &str,
        profile: &str,
    ) -> Result<Option<aws_sdk_eks::types::FargateProfile>, VaporError> {
        let result = self
            .inner
            .describe_fargate_profile()
            .cluster_name(cluster)
            .fargate_profile_name(profile)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.fargate_profile().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_resource_not_found_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(e.to_string()))
                }
            }
        }
    }

    /// List addon names for a cluster with next_token pagination.
    pub async fn list_addons(&self, cluster: &str) -> Result<Vec<String>, VaporError> {
        let mut names = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_addons().cluster_name(cluster);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            names.extend(output.addons().iter().map(|s| s.to_string()));
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(names)
    }

    /// Describe a single addon. Returns None if not found.
    pub async fn describe_addon(
        &self,
        cluster: &str,
        addon: &str,
    ) -> Result<Option<aws_sdk_eks::types::Addon>, VaporError> {
        let result = self
            .inner
            .describe_addon()
            .cluster_name(cluster)
            .addon_name(addon)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.addon().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_resource_not_found_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(e.to_string()))
                }
            }
        }
    }
}
