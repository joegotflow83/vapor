use async_graphql::SimpleObject;

use crate::aws::dms::{DmsEndpointInfo, DmsReplicationInstanceInfo, DmsReplicationTaskInfo};

#[derive(SimpleObject, Clone)]
pub struct DmsReplicationInstance {
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

impl From<DmsReplicationInstanceInfo> for DmsReplicationInstance {
    fn from(i: DmsReplicationInstanceInfo) -> Self {
        Self {
            replication_instance_identifier: i.replication_instance_identifier,
            replication_instance_arn: i.replication_instance_arn,
            replication_instance_class: i.replication_instance_class,
            replication_instance_status: i.replication_instance_status,
            allocated_storage: i.allocated_storage,
            publicly_accessible: i.publicly_accessible,
            engine_version: i.engine_version,
            vpc_security_groups: i.vpc_security_groups,
            replication_subnet_group_id: i.replication_subnet_group_id,
            availability_zone: i.availability_zone,
            multi_az: i.multi_az,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DmsEndpoint {
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

impl From<DmsEndpointInfo> for DmsEndpoint {
    fn from(e: DmsEndpointInfo) -> Self {
        Self {
            endpoint_identifier: e.endpoint_identifier,
            endpoint_arn: e.endpoint_arn,
            endpoint_type: e.endpoint_type,
            engine_name: e.engine_name,
            status: e.status,
            database_name: e.database_name,
            server_name: e.server_name,
            port: e.port,
            ssl_mode: e.ssl_mode,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DmsReplicationTask {
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

impl From<DmsReplicationTaskInfo> for DmsReplicationTask {
    fn from(t: DmsReplicationTaskInfo) -> Self {
        Self {
            replication_task_identifier: t.replication_task_identifier,
            replication_task_arn: t.replication_task_arn,
            status: t.status,
            migration_type: t.migration_type,
            source_endpoint_arn: t.source_endpoint_arn,
            target_endpoint_arn: t.target_endpoint_arn,
            replication_instance_arn: t.replication_instance_arn,
            replication_task_creation_date: t.replication_task_creation_date,
            replication_task_start_date: t.replication_task_start_date,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::dms::{DmsEndpointInfo, DmsReplicationInstanceInfo, DmsReplicationTaskInfo};

    #[test]
    fn test_replication_instance_from() {
        let info = DmsReplicationInstanceInfo {
            replication_instance_identifier: Some("my-ri".to_string()),
            replication_instance_arn: Some("arn:aws:dms:us-east-1:123456789012:ri:my-ri".to_string()),
            replication_instance_class: Some("dms.r5.large".to_string()),
            replication_instance_status: Some("available".to_string()),
            allocated_storage: Some(100),
            publicly_accessible: Some(false),
            engine_version: Some("3.5.1".to_string()),
            vpc_security_groups: vec!["sg-abc123".to_string()],
            replication_subnet_group_id: Some("my-subnet-group".to_string()),
            availability_zone: Some("us-east-1a".to_string()),
            multi_az: Some(false),
        };
        let result = DmsReplicationInstance::from(info);
        assert_eq!(result.replication_instance_identifier, Some("my-ri".to_string()));
        assert_eq!(result.replication_instance_class, Some("dms.r5.large".to_string()));
        assert_eq!(result.allocated_storage, Some(100));
        assert_eq!(result.publicly_accessible, Some(false));
        assert_eq!(result.vpc_security_groups, vec!["sg-abc123"]);
        assert_eq!(result.multi_az, Some(false));
    }

    #[test]
    fn test_replication_instance_minimal() {
        let info = DmsReplicationInstanceInfo {
            replication_instance_identifier: None,
            replication_instance_arn: None,
            replication_instance_class: None,
            replication_instance_status: None,
            allocated_storage: None,
            publicly_accessible: None,
            engine_version: None,
            vpc_security_groups: vec![],
            replication_subnet_group_id: None,
            availability_zone: None,
            multi_az: None,
        };
        let result = DmsReplicationInstance::from(info);
        assert!(result.replication_instance_identifier.is_none());
        assert!(result.vpc_security_groups.is_empty());
    }

    #[test]
    fn test_endpoint_from() {
        let info = DmsEndpointInfo {
            endpoint_identifier: Some("source-pg".to_string()),
            endpoint_arn: Some("arn:aws:dms:us-east-1:123456789012:endpoint:source-pg".to_string()),
            endpoint_type: Some("source".to_string()),
            engine_name: Some("postgres".to_string()),
            status: Some("active".to_string()),
            database_name: Some("mydb".to_string()),
            server_name: Some("db.example.com".to_string()),
            port: Some(5432),
            ssl_mode: Some("require".to_string()),
        };
        let result = DmsEndpoint::from(info);
        assert_eq!(result.endpoint_identifier, Some("source-pg".to_string()));
        assert_eq!(result.endpoint_type, Some("source".to_string()));
        assert_eq!(result.engine_name, Some("postgres".to_string()));
        assert_eq!(result.port, Some(5432));
        assert_eq!(result.ssl_mode, Some("require".to_string()));
    }

    #[test]
    fn test_endpoint_minimal() {
        let info = DmsEndpointInfo {
            endpoint_identifier: None,
            endpoint_arn: None,
            endpoint_type: None,
            engine_name: None,
            status: None,
            database_name: None,
            server_name: None,
            port: None,
            ssl_mode: None,
        };
        let result = DmsEndpoint::from(info);
        assert!(result.endpoint_identifier.is_none());
        assert!(result.port.is_none());
    }

    #[test]
    fn test_replication_task_from() {
        let info = DmsReplicationTaskInfo {
            replication_task_identifier: Some("my-task".to_string()),
            replication_task_arn: Some("arn:aws:dms:us-east-1:123456789012:task:my-task".to_string()),
            status: Some("running".to_string()),
            migration_type: Some("full-load-and-cdc".to_string()),
            source_endpoint_arn: Some("arn:aws:dms:us-east-1:123456789012:endpoint:source".to_string()),
            target_endpoint_arn: Some("arn:aws:dms:us-east-1:123456789012:endpoint:target".to_string()),
            replication_instance_arn: Some("arn:aws:dms:us-east-1:123456789012:ri:my-ri".to_string()),
            replication_task_creation_date: Some("2024-01-01T00:00:00Z".to_string()),
            replication_task_start_date: Some("2024-01-02T00:00:00Z".to_string()),
        };
        let result = DmsReplicationTask::from(info);
        assert_eq!(result.replication_task_identifier, Some("my-task".to_string()));
        assert_eq!(result.status, Some("running".to_string()));
        assert_eq!(result.migration_type, Some("full-load-and-cdc".to_string()));
        assert!(result.source_endpoint_arn.is_some());
        assert!(result.target_endpoint_arn.is_some());
    }

    #[test]
    fn test_replication_task_minimal() {
        let info = DmsReplicationTaskInfo {
            replication_task_identifier: None,
            replication_task_arn: None,
            status: None,
            migration_type: None,
            source_endpoint_arn: None,
            target_endpoint_arn: None,
            replication_instance_arn: None,
            replication_task_creation_date: None,
            replication_task_start_date: None,
        };
        let result = DmsReplicationTask::from(info);
        assert!(result.replication_task_identifier.is_none());
        assert!(result.migration_type.is_none());
    }
}
