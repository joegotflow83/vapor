use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct RdsClient {
    inner: aws_sdk_rds::Client,
}

impl RdsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_rds::Client::new(config),
        }
    }

    pub async fn describe_db_instances(
        &self,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_rds::types::DbInstance>, VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbInstance> = Vec::new();

        if let Some(ids) = ids {
            for id in ids {
                let output = self
                    .inner
                    .describe_db_instances()
                    .db_instance_identifier(&id)
                    .send()
                    .await
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                all_items.extend(output.db_instances().iter().cloned());
            }
        } else {
            let mut next_marker: Option<String> = None;
            loop {
                let mut request = self.inner.describe_db_instances();
                if let Some(ref marker) = next_marker {
                    request = request.marker(marker);
                }
                let output = request
                    .send()
                    .await
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                all_items.extend(output.db_instances().iter().cloned());
                match output.marker() {
                    Some(m) => next_marker = Some(m.to_string()),
                    None => break,
                }
            }
        }

        Ok(all_items)
    }

    pub async fn describe_db_clusters(
        &self,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_rds::types::DbCluster>, VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbCluster> = Vec::new();

        if let Some(ids) = ids {
            for id in ids {
                let output = self
                    .inner
                    .describe_db_clusters()
                    .db_cluster_identifier(&id)
                    .send()
                    .await
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                all_items.extend(output.db_clusters().iter().cloned());
            }
        } else {
            let mut next_marker: Option<String> = None;
            loop {
                let mut request = self.inner.describe_db_clusters();
                if let Some(ref marker) = next_marker {
                    request = request.marker(marker);
                }
                let output = request
                    .send()
                    .await
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                all_items.extend(output.db_clusters().iter().cloned());
                match output.marker() {
                    Some(m) => next_marker = Some(m.to_string()),
                    None => break,
                }
            }
        }

        Ok(all_items)
    }

    pub async fn describe_db_parameter_groups(
        &self,
        name: Option<String>,
    ) -> Result<Vec<aws_sdk_rds::types::DbParameterGroup>, VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbParameterGroup> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = self.inner.describe_db_parameter_groups();
            if let Some(ref n) = name {
                request = request.db_parameter_group_name(n);
            }
            if let Some(ref marker) = next_marker {
                request = request.marker(marker);
            }
            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_items.extend(output.db_parameter_groups().iter().cloned());
            match output.marker() {
                Some(m) => next_marker = Some(m.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }

    pub async fn describe_db_subnet_groups(
        &self,
        name: Option<String>,
    ) -> Result<Vec<aws_sdk_rds::types::DbSubnetGroup>, VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbSubnetGroup> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = self.inner.describe_db_subnet_groups();
            if let Some(ref n) = name {
                request = request.db_subnet_group_name(n);
            }
            if let Some(ref marker) = next_marker {
                request = request.marker(marker);
            }
            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_items.extend(output.db_subnet_groups().iter().cloned());
            match output.marker() {
                Some(m) => next_marker = Some(m.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }

    pub async fn describe_db_snapshots(
        &self,
        db_instance_id: Option<String>,
        snapshot_type: Option<String>,
    ) -> Result<Vec<aws_sdk_rds::types::DbSnapshot>, VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbSnapshot> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = self.inner.describe_db_snapshots();

            if let Some(ref id) = db_instance_id {
                request = request.db_instance_identifier(id);
            }
            if let Some(ref st) = snapshot_type {
                request = request.snapshot_type(st);
            }
            if let Some(ref marker) = next_marker {
                request = request.marker(marker);
            }

            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_items.extend(output.db_snapshots().iter().cloned());

            match output.marker() {
                Some(m) => next_marker = Some(m.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }
}
