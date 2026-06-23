use aws_config::SdkConfig;
use aws_sdk_redshift::types::{Cluster, Snapshot};

use crate::error::VaporError;

pub struct RedshiftClient {
    inner: aws_sdk_redshift::Client,
}

impl RedshiftClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_redshift::Client::new(config),
        }
    }

    pub async fn describe_clusters(&self) -> Result<Vec<Cluster>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_clusters();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.clusters().to_vec());

            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_cluster_snapshots(
        &self,
        cluster_identifier: Option<String>,
        snapshot_type: Option<String>,
    ) -> Result<Vec<Snapshot>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_cluster_snapshots();
            if let Some(ref id) = cluster_identifier {
                req = req.cluster_identifier(id);
            }
            if let Some(ref st) = snapshot_type {
                req = req.snapshot_type(st);
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.snapshots().to_vec());

            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
