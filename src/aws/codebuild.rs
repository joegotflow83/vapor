use aws_config::SdkConfig;
use aws_sdk_codebuild::types::{Build, Project};

use crate::error::VaporError;

pub struct CodeBuildClient {
    inner: aws_sdk_codebuild::Client,
}

impl CodeBuildClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_codebuild::Client::new(config),
        }
    }

    pub async fn list_projects(&self) -> Result<Vec<String>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_projects();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.projects().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(items)
    }

    pub async fn batch_get_projects(&self, names: Vec<String>) -> Result<Vec<Project>, VaporError> {
        let mut all = Vec::new();
        for chunk in names.chunks(100) {
            let output = self
                .inner
                .batch_get_projects()
                .set_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.projects().to_vec());
        }
        Ok(all)
    }

    pub async fn list_builds_for_project(&self, project_name: &str) -> Result<Vec<String>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_builds_for_project().project_name(project_name);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.ids().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(items)
    }

    pub async fn batch_get_builds(&self, ids: Vec<String>) -> Result<Vec<Build>, VaporError> {
        let mut all = Vec::new();
        for chunk in ids.chunks(100) {
            let output = self
                .inner
                .batch_get_builds()
                .set_ids(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.builds().to_vec());
        }
        Ok(all)
    }
}
