use aws_config::SdkConfig;
use aws_sdk_databasemigration::types::Filter;

use crate::error::VaporError;

pub struct DmsReplicationInstanceInfo {
    pub replication_instance_identifier: Option<String>,
    pub replication_instance_arn: Option<String>,
    pub replication_instance_class: Option<String>,
    pub replication_instance_status: Option<String>,
    pub allocated_storage: Option<i32>,
    pub publicly_accessible: Option<bool>,
    pub engine_version: Option<String>,
    pub vpc_security_groups: Vec<String>,
    pub replication_subnet_group_id: Option<String>,
    pub availability_zone: Option<String>,
    pub multi_az: Option<bool>,
}

pub struct DmsEndpointInfo {
    pub endpoint_identifier: Option<String>,
    pub endpoint_arn: Option<String>,
    pub endpoint_type: Option<String>,
    pub engine_name: Option<String>,
    pub status: Option<String>,
    pub database_name: Option<String>,
    pub server_name: Option<String>,
    pub port: Option<i32>,
    pub ssl_mode: Option<String>,
}

pub struct DmsReplicationTaskInfo {
    pub replication_task_identifier: Option<String>,
    pub replication_task_arn: Option<String>,
    pub status: Option<String>,
    pub migration_type: Option<String>,
    pub source_endpoint_arn: Option<String>,
    pub target_endpoint_arn: Option<String>,
    pub replication_instance_arn: Option<String>,
    pub replication_task_creation_date: Option<String>,
    pub replication_task_start_date: Option<String>,
}

pub struct DmsClient {
    inner: aws_sdk_databasemigration::Client,
}

impl DmsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_databasemigration::Client::new(config),
        }
    }

    pub async fn describe_replication_instances(&self) -> Result<Vec<DmsReplicationInstanceInfo>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_replication_instances();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for ri in output.replication_instances() {
                let vpc_security_groups = ri
                    .vpc_security_groups()
                    .iter()
                    .filter_map(|sg| sg.vpc_security_group_id().map(|s| s.to_string()))
                    .collect();
                items.push(DmsReplicationInstanceInfo {
                    replication_instance_identifier: ri.replication_instance_identifier().map(|s| s.to_string()),
                    replication_instance_arn: ri.replication_instance_arn().map(|s| s.to_string()),
                    replication_instance_class: ri.replication_instance_class().map(|s| s.to_string()),
                    replication_instance_status: ri.replication_instance_status().map(|s| s.to_string()),
                    allocated_storage: Some(ri.allocated_storage()),
                    publicly_accessible: Some(ri.publicly_accessible()),
                    engine_version: ri.engine_version().map(|s| s.to_string()),
                    vpc_security_groups,
                    replication_subnet_group_id: ri
                        .replication_subnet_group()
                        .and_then(|g| g.replication_subnet_group_identifier())
                        .map(|s| s.to_string()),
                    availability_zone: ri.availability_zone().map(|s| s.to_string()),
                    multi_az: Some(ri.multi_az()),
                });
            }
            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_endpoints(&self, endpoint_type: Option<String>) -> Result<Vec<DmsEndpointInfo>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_endpoints();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(ref et) = endpoint_type {
                let filter = Filter::builder()
                    .name("endpoint-type")
                    .values(et.to_lowercase())
                    .build()
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                req = req.filters(filter);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for ep in output.endpoints() {
                items.push(DmsEndpointInfo {
                    endpoint_identifier: ep.endpoint_identifier().map(|s| s.to_string()),
                    endpoint_arn: ep.endpoint_arn().map(|s| s.to_string()),
                    endpoint_type: ep.endpoint_type().map(|t| t.as_str().to_string()),
                    engine_name: ep.engine_name().map(|s| s.to_string()),
                    status: ep.status().map(|s| s.to_string()),
                    database_name: ep.database_name().map(|s| s.to_string()),
                    server_name: ep.server_name().map(|s| s.to_string()),
                    port: ep.port(),
                    ssl_mode: ep.ssl_mode().map(|m| m.as_str().to_string()),
                });
            }
            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_replication_tasks(&self) -> Result<Vec<DmsReplicationTaskInfo>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.describe_replication_tasks();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for task in output.replication_tasks() {
                items.push(DmsReplicationTaskInfo {
                    replication_task_identifier: task.replication_task_identifier().map(|s| s.to_string()),
                    replication_task_arn: task.replication_task_arn().map(|s| s.to_string()),
                    status: task.status().map(|s| s.to_string()),
                    migration_type: task.migration_type().map(|t| t.as_str().to_string()),
                    source_endpoint_arn: task.source_endpoint_arn().map(|s| s.to_string()),
                    target_endpoint_arn: task.target_endpoint_arn().map(|s| s.to_string()),
                    replication_instance_arn: task.replication_instance_arn().map(|s| s.to_string()),
                    replication_task_creation_date: task.replication_task_creation_date().map(|d| d.to_string()),
                    replication_task_start_date: task.replication_task_start_date().map(|d| d.to_string()),
                });
            }
            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
