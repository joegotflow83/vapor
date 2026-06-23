use aws_config::SdkConfig;
use aws_sdk_neptune::types::{DbCluster, DbInstance, Filter};

use crate::error::VaporError;

pub struct NeptuneClient {
    inner: aws_sdk_neptune::Client,
}

impl NeptuneClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_neptune::Client::new(config),
        }
    }

    pub async fn describe_db_clusters(&self) -> Result<Vec<DbCluster>, VaporError> {
        let mut clusters = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_db_clusters();
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            clusters.extend(output.db_clusters().to_vec());

            match output.marker() {
                Some(m) if !m.is_empty() => next_marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(clusters)
    }

    pub async fn describe_db_instances(
        &self,
        cluster_id: Option<String>,
    ) -> Result<Vec<DbInstance>, VaporError> {
        let mut instances = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_db_instances();

            if let Some(ref cid) = cluster_id {
                let filter = Filter::builder()
                    .name("db-cluster-id")
                    .values(cid)
                    .build();
                req = req.filters(filter);
            }

            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            instances.extend(output.db_instances().to_vec());

            match output.marker() {
                Some(m) if !m.is_empty() => next_marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(instances)
    }
}
