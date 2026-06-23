use aws_config::SdkConfig;
use aws_sdk_memorydb::types::{Cluster, SubnetGroup, Tag};

use crate::error::VaporError;

pub struct MemoryDbClient {
    inner: aws_sdk_memorydb::Client,
}

impl MemoryDbClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_memorydb::Client::new(config),
        }
    }

    pub async fn describe_clusters(&self) -> Result<Vec<Cluster>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_clusters();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.clusters().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_subnet_groups(&self) -> Result<Vec<SubnetGroup>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_subnet_groups();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.subnet_groups().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_tags(&self, resource_arn: &str) -> Result<Vec<Tag>, VaporError> {
        let output = self
            .inner
            .list_tags()
            .resource_arn(resource_arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.tag_list().to_vec())
    }
}
