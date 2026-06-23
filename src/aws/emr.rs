use aws_config::SdkConfig;
use aws_sdk_emr::types::{Cluster, ClusterState, ClusterSummary, StepSummary};

use crate::error::VaporError;

pub struct EmrClient {
    inner: aws_sdk_emr::Client,
}

impl EmrClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_emr::Client::new(config),
        }
    }

    pub async fn list_clusters(
        &self,
        states: Option<Vec<ClusterState>>,
    ) -> Result<Vec<ClusterSummary>, VaporError> {
        let mut clusters = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_clusters();
            if let Some(ref s) = states {
                req = req.set_cluster_states(Some(s.clone()));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            clusters.extend(output.clusters().to_vec());

            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(clusters)
    }

    pub async fn describe_cluster(&self, cluster_id: &str) -> Result<Cluster, VaporError> {
        let output = self
            .inner
            .describe_cluster()
            .cluster_id(cluster_id)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        output
            .cluster
            .ok_or_else(|| VaporError::AwsSdk(format!("No cluster returned for id {cluster_id}")))
    }

    pub async fn list_steps(&self, cluster_id: &str) -> Result<Vec<StepSummary>, VaporError> {
        let mut steps = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_steps().cluster_id(cluster_id);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            steps.extend(output.steps().to_vec());

            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(steps)
    }
}
