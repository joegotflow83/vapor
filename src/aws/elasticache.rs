use aws_config::SdkConfig;
use aws_sdk_elasticache::types::{CacheCluster, CacheSubnetGroup, ReplicationGroup, Tag};
use futures::future::join_all;

use crate::error::VaporError;

pub struct ElastiCacheClient {
    inner: aws_sdk_elasticache::Client,
}

impl ElastiCacheClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_elasticache::Client::new(config),
        }
    }

    pub async fn describe_cache_clusters(
        &self,
        cluster_id: Option<&str>,
    ) -> Result<Vec<(CacheCluster, Vec<Tag>)>, VaporError> {
        let mut clusters: Vec<CacheCluster> = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .describe_cache_clusters()
                .show_cache_node_info(true);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(id) = cluster_id {
                req = req.cache_cluster_id(id);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for c in output.cache_clusters() {
                clusters.push(c.clone());
            }
            match output.marker() {
                Some(m) => marker = Some(m.to_string()),
                None => break,
            }
        }

        let tag_futs = clusters.iter().map(|c| {
            let arn = c.arn().unwrap_or("").to_string();
            async move {
                if arn.is_empty() {
                    return vec![];
                }
                self.list_tags(&arn).await.unwrap_or_default()
            }
        });
        let all_tags = join_all(tag_futs).await;

        Ok(clusters.into_iter().zip(all_tags).collect())
    }

    pub async fn describe_replication_groups(
        &self,
        replication_group_id: Option<&str>,
    ) -> Result<Vec<ReplicationGroup>, VaporError> {
        let mut groups: Vec<ReplicationGroup> = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_replication_groups();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(id) = replication_group_id {
                req = req.replication_group_id(id);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for g in output.replication_groups() {
                groups.push(g.clone());
            }
            match output.marker() {
                Some(m) => marker = Some(m.to_string()),
                None => break,
            }
        }

        Ok(groups)
    }

    pub async fn describe_cache_subnet_groups(
        &self,
    ) -> Result<Vec<CacheSubnetGroup>, VaporError> {
        let mut subnet_groups: Vec<CacheSubnetGroup> = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_cache_subnet_groups();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for sg in output.cache_subnet_groups() {
                subnet_groups.push(sg.clone());
            }
            match output.marker() {
                Some(m) => marker = Some(m.to_string()),
                None => break,
            }
        }

        Ok(subnet_groups)
    }

    async fn list_tags(&self, arn: &str) -> Result<Vec<Tag>, VaporError> {
        let output = self
            .inner
            .list_tags_for_resource()
            .resource_name(arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.tag_list().to_vec())
    }
}
