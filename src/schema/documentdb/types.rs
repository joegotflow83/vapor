use async_graphql::SimpleObject;
use aws_sdk_docdb::types::{DbCluster, DbInstance};

#[derive(SimpleObject, Clone)]
pub struct DocDbCluster {
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
    pub deletion_protection: bool,
    pub db_subnet_group: Option<String>,
}

impl From<DbCluster> for DocDbCluster {
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
            deletion_protection: c.deletion_protection().unwrap_or(false),
            db_subnet_group: c.db_subnet_group().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DocDbInstance {
    pub instance_identifier: String,
    pub arn: Option<String>,
    pub instance_class: Option<String>,
    pub status: Option<String>,
    pub cluster_identifier: Option<String>,
    pub availability_zone: Option<String>,
    pub endpoint: Option<String>,
}

impl From<DbInstance> for DocDbInstance {
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
    fn test_docdb_cluster_full() {
        let cluster = DocDbCluster {
            cluster_identifier: "my-docdb-cluster".to_string(),
            arn: Some("arn:aws:rds:us-east-1:123456789012:cluster:my-docdb-cluster".to_string()),
            status: Some("available".to_string()),
            engine: Some("docdb".to_string()),
            engine_version: Some("5.0.0".to_string()),
            endpoint: Some("my-docdb-cluster.cluster-abc123.us-east-1.docdb.amazonaws.com".to_string()),
            reader_endpoint: Some("my-docdb-cluster.cluster-ro-abc123.us-east-1.docdb.amazonaws.com".to_string()),
            port: Some(27017),
            multi_az: true,
            storage_encrypted: true,
            deletion_protection: true,
            db_subnet_group: Some("my-subnet-group".to_string()),
        };

        assert_eq!(cluster.cluster_identifier, "my-docdb-cluster");
        assert_eq!(cluster.engine, Some("docdb".to_string()));
        assert_eq!(cluster.port, Some(27017));
        assert!(cluster.multi_az);
        assert!(cluster.storage_encrypted);
        assert!(cluster.deletion_protection);
        assert_eq!(cluster.db_subnet_group, Some("my-subnet-group".to_string()));
    }

    #[test]
    fn test_docdb_cluster_minimal() {
        let cluster = DocDbCluster {
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
            deletion_protection: false,
            db_subnet_group: None,
        };

        assert_eq!(cluster.cluster_identifier, "minimal-cluster");
        assert!(cluster.arn.is_none());
        assert!(!cluster.multi_az);
        assert!(!cluster.storage_encrypted);
        assert!(!cluster.deletion_protection);
        assert!(cluster.db_subnet_group.is_none());
    }

    #[test]
    fn test_docdb_instance_full() {
        let instance = DocDbInstance {
            instance_identifier: "my-docdb-instance".to_string(),
            arn: Some("arn:aws:rds:us-east-1:123456789012:db:my-docdb-instance".to_string()),
            instance_class: Some("db.r5.large".to_string()),
            status: Some("available".to_string()),
            cluster_identifier: Some("my-docdb-cluster".to_string()),
            availability_zone: Some("us-east-1a".to_string()),
            endpoint: Some("my-docdb-instance.abc123.us-east-1.docdb.amazonaws.com".to_string()),
        };

        assert_eq!(instance.instance_identifier, "my-docdb-instance");
        assert_eq!(instance.instance_class, Some("db.r5.large".to_string()));
        assert_eq!(instance.status, Some("available".to_string()));
        assert_eq!(instance.cluster_identifier, Some("my-docdb-cluster".to_string()));
        assert_eq!(instance.availability_zone, Some("us-east-1a".to_string()));
    }

    #[test]
    fn test_docdb_instance_minimal() {
        let instance = DocDbInstance {
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
