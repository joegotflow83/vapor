use aws_config::SdkConfig;
use aws_sdk_codedeploy::types::{ApplicationInfo, DeploymentGroupInfo, DeploymentInfo};

use crate::error::VaporError;

pub struct CodeDeployClient {
    inner: aws_sdk_codedeploy::Client,
}

impl CodeDeployClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_codedeploy::Client::new(config),
        }
    }

    pub async fn list_applications(&self) -> Result<Vec<String>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_applications();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.applications().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(items)
    }

    pub async fn batch_get_applications(&self, names: Vec<String>) -> Result<Vec<ApplicationInfo>, VaporError> {
        let mut all = Vec::new();
        for chunk in names.chunks(100) {
            let output = self
                .inner
                .batch_get_applications()
                .set_application_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.applications_info().to_vec());
        }
        Ok(all)
    }

    pub async fn list_deployment_groups(&self, app_name: &str) -> Result<Vec<String>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_deployment_groups().application_name(app_name);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.deployment_groups().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(items)
    }

    pub async fn batch_get_deployment_groups(&self, app_name: &str, group_names: Vec<String>) -> Result<Vec<DeploymentGroupInfo>, VaporError> {
        let mut all = Vec::new();
        for chunk in group_names.chunks(100) {
            let output = self
                .inner
                .batch_get_deployment_groups()
                .application_name(app_name)
                .set_deployment_group_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.deployment_groups_info().to_vec());
        }
        Ok(all)
    }

    pub async fn list_deployments(&self, app_name: Option<&str>, group_name: Option<&str>) -> Result<Vec<String>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_deployments();
            if let Some(app) = app_name {
                req = req.application_name(app);
            }
            if let Some(group) = group_name {
                req = req.deployment_group_name(group);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.deployments().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(items)
    }

    pub async fn batch_get_deployments(&self, ids: Vec<String>) -> Result<Vec<DeploymentInfo>, VaporError> {
        let mut all = Vec::new();
        for chunk in ids.chunks(100) {
            let output = self
                .inner
                .batch_get_deployments()
                .set_deployment_ids(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.deployments_info().to_vec());
        }
        Ok(all)
    }
}
