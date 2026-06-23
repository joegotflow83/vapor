use async_graphql::SimpleObject;
use aws_sdk_neptune::types::{DbCluster, DbInstance};

#[derive(SimpleObject, Clone)]
pub struct NeptuneCluster {
    pub cluster_identifier: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    pub endpoint: Option<String>,
    pub reader_endpoint: Option<String>,
    pub port: Option<i32>,
    pub multi_az: bool,
    pub storage_encrypted: bool,
    pub kms_key_id: Option<String>,
    pub deletion_protection: bool,
}

impl From<DbCluster> for NeptuneCluster {
    fn from(c: DbCluster) -> Self {
        Self {
            cluster_identifier: c.db_cluster_identifier().unwrap_or_default().to_string(),
            arn: c.db_cluster_arn().map(|s| s.to_string()),
            status: c.status().map(|s| s.to_string()),
            engine: c.engine().map(|s| s.to_string()),
            engine_version: c.engine_version().map(|s| s.to_string()),
            endpoint: c.endpoint().map(|s| s.to_string()),
            reader_endpoint: c.reader_endpoint().map(|s| s.to_string()),
            port: c.port(),
            multi_az: c.multi_az().unwrap_or(false),
            storage_encrypted: c.storage_encrypted().unwrap_or(false),
            kms_key_id: c.kms_key_id().map(|s| s.to_string()),
            deletion_protection: c.deletion_protection().unwrap_or(false),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct NeptuneInstance {
    pub instance_identifier: String,
    pub arn: Option<String>,
    pub instance_class: Option<String>,
    pub status: Option<String>,
    pub cluster_identifier: Option<String>,
    pub availability_zone: Option<String>,
    pub endpoint: Option<String>,
}

impl From<DbInstance> for NeptuneInstance {
    fn from(i: DbInstance) -> Self {
        Self {
            instance_identifier: i.db_instance_identifier().unwrap_or_default().to_string(),
            arn: i.db_instance_arn().map(|s| s.to_string()),
            instance_class: i.db_instance_class().map(|s| s.to_string()),
            status: i.db_instance_status().map(|s| s.to_string()),
            cluster_identifier: i.db_cluster_identifier().map(|s| s.to_string()),
            availability_zone: i.availability_zone().map(|s| s.to_string()),
            endpoint: i.endpoint().and_then(|e| e.address()).map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neptune_cluster_full() {
        let cluster = NeptuneCluster {
            cluster_identifier: "my-neptune-cluster".to_string(),
            arn: Some("arn:aws:rds:us-east-1:123456789012:cluster:my-neptune-cluster".to_string()),
            status: Some("available".to_string()),
            engine: Some("neptune".to_string()),
            engine_version: Some("1.2.1.0".to_string()),
            endpoint: Some("my-neptune-cluster.cluster-abc123.us-east-1.neptune.amazonaws.com".to_string()),
            reader_endpoint: Some("my-neptune-cluster.cluster-ro-abc123.us-east-1.neptune.amazonaws.com".to_string()),
            port: Some(8182),
            multi_az: true,
            storage_encrypted: true,
            kms_key_id: Some("arn:aws:kms:us-east-1:123456789012:key/abc123".to_string()),
            deletion_protection: true,
        };

        assert_eq!(cluster.cluster_identifier, "my-neptune-cluster");
        assert_eq!(cluster.engine, Some("neptune".to_string()));
        assert_eq!(cluster.port, Some(8182));
        assert!(cluster.multi_az);
        assert!(cluster.storage_encrypted);
        assert!(cluster.deletion_protection);
    }

    #[test]
    fn test_neptune_cluster_minimal() {
        let cluster = NeptuneCluster {
            cluster_identifier: "minimal-cluster".to_string(),
            arn: None,
            status: None,
            engine: None,
            engine_version: None,
            endpoint: None,
            reader_endpoint: None,
            port: None,
            multi_az: false,
            storage_encrypted: false,
            kms_key_id: None,
            deletion_protection: false,
        };

        assert_eq!(cluster.cluster_identifier, "minimal-cluster");
        assert!(cluster.arn.is_none());
        assert!(!cluster.multi_az);
        assert!(!cluster.storage_encrypted);
        assert!(!cluster.deletion_protection);
    }

    #[test]
    fn test_neptune_instance_full() {
        let instance = NeptuneInstance {
            instance_identifier: "my-neptune-instance".to_string(),
            arn: Some("arn:aws:rds:us-east-1:123456789012:db:my-neptune-instance".to_string()),
            instance_class: Some("db.r5.large".to_string()),
            status: Some("available".to_string()),
            cluster_identifier: Some("my-neptune-cluster".to_string()),
            availability_zone: Some("us-east-1a".to_string()),
            endpoint: Some("my-neptune-instance.abc123.us-east-1.neptune.amazonaws.com".to_string()),
        };

        assert_eq!(instance.instance_identifier, "my-neptune-instance");
        assert_eq!(instance.instance_class, Some("db.r5.large".to_string()));
        assert_eq!(instance.status, Some("available".to_string()));
        assert_eq!(instance.cluster_identifier, Some("my-neptune-cluster".to_string()));
        assert_eq!(instance.availability_zone, Some("us-east-1a".to_string()));
    }

    #[test]
    fn test_neptune_instance_minimal() {
        let instance = NeptuneInstance {
            instance_identifier: "".to_string(),
            arn: None,
            instance_class: None,
            status: None,
            cluster_identifier: None,
            availability_zone: None,
            endpoint: None,
        };

        assert!(instance.arn.is_none());
        assert!(instance.instance_class.is_none());
        assert!(instance.cluster_identifier.is_none());
        assert!(instance.endpoint.is_none());
    }
}
