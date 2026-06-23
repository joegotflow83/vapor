#[cfg(feature = "kafka")]
use aws_config::SdkConfig;
#[cfg(feature = "kafka")]
use aws_sdk_kafka::types::{Cluster, NodeInfo};

#[cfg(feature = "kafka")]
use crate::error::VaporError;

#[cfg(feature = "kafka")]
pub struct MskClient {
    inner: aws_sdk_kafka::Client,
}

impl MskClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_kafka::Client::new(config),
        }
    }

    pub async fn list_clusters_v2(&self) -> Result<Vec<Cluster>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_clusters_v2();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.cluster_info_list().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_nodes(&self, cluster_arn: &str) -> Result<Vec<NodeInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_nodes().cluster_arn(cluster_arn);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.node_info_list().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
