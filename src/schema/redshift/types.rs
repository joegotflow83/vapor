use async_graphql::SimpleObject;
use aws_sdk_redshift::types::{Cluster as SdkCluster, Snapshot as SdkSnapshot};

#[derive(SimpleObject, Clone)]
#[graphql(name = "RedshiftTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct RedshiftCluster {
    pub identifier: String,
    pub node_type: Option<String>,
    pub cluster_status: Option<String>,
    pub db_name: Option<String>,
    pub master_username: Option<String>,
    pub endpoint_address: Option<String>,
    pub endpoint_port: Option<i32>,
    pub number_of_nodes: Option<i32>,
    pub vpc_id: Option<String>,
    pub encrypted: bool,
    pub publicly_accessible: bool,
    pub created_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<SdkCluster> for RedshiftCluster {
    fn from(c: SdkCluster) -> Self {
        let tags = c
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        Self {
            identifier: c.cluster_identifier().unwrap_or_default().to_string(),
            node_type: c.node_type().map(|v| v.to_string()),
            cluster_status: c.cluster_status().map(|v| v.to_string()),
            db_name: c.db_name().map(|v| v.to_string()),
            master_username: c.master_username().map(|v| v.to_string()),
            endpoint_address: c.endpoint().and_then(|e| e.address().map(|v| v.to_string())),
            endpoint_port: c.endpoint().and_then(|e| e.port()),
            number_of_nodes: c.number_of_nodes(),
            vpc_id: c.vpc_id().map(|v| v.to_string()),
            encrypted: c.encrypted().unwrap_or(false),
            publicly_accessible: c.publicly_accessible().unwrap_or(false),
            created_at: c.cluster_create_time().map(|t| t.to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RedshiftSnapshot {
    pub id: String,
    pub cluster_identifier: Option<String>,
    pub snapshot_type: Option<String>,
    pub status: Option<String>,
    pub node_type: Option<String>,
    pub number_of_nodes: Option<i32>,
    pub cluster_version: Option<String>,
    pub availability_zone: Option<String>,
    pub total_backup_size_in_mega_bytes: f64,
    pub encrypted: bool,
    pub master_username: Option<String>,
    pub created_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<SdkSnapshot> for RedshiftSnapshot {
    fn from(s: SdkSnapshot) -> Self {
        let tags = s
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        Self {
            id: s.snapshot_identifier().unwrap_or_default().to_string(),
            cluster_identifier: s.cluster_identifier().map(|v| v.to_string()),
            snapshot_type: s.snapshot_type().map(|v| v.to_string()),
            status: s.status().map(|v| v.to_string()),
            node_type: s.node_type().map(|v| v.to_string()),
            number_of_nodes: s.number_of_nodes(),
            cluster_version: s.cluster_version().map(|v| v.to_string()),
            availability_zone: s.availability_zone().map(|v| v.to_string()),
            total_backup_size_in_mega_bytes: s.total_backup_size_in_mega_bytes().unwrap_or_default(),
            encrypted: s.encrypted().unwrap_or(false),
            master_username: s.master_username().map(|v| v.to_string()),
            created_at: s.snapshot_create_time().map(|t| t.to_string()),
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redshift_cluster_from_sdk_minimal() {
        let cluster = SdkCluster::builder()
            .cluster_identifier("my-cluster")
            .cluster_status("available")
            .node_type("dc2.large")
            .number_of_nodes(2)
            .encrypted(true)
            .publicly_accessible(false)
            .build();
        let rc = RedshiftCluster::from(cluster);
        assert_eq!(rc.identifier, "my-cluster");
        assert_eq!(rc.cluster_status, Some("available".to_string()));
        assert_eq!(rc.node_type, Some("dc2.large".to_string()));
        assert_eq!(rc.number_of_nodes, Some(2));
        assert!(rc.encrypted);
        assert!(!rc.publicly_accessible);
        assert!(rc.tags.is_empty());
    }

    #[test]
    fn test_redshift_cluster_with_tags() {
        let cluster = SdkCluster::builder()
            .cluster_identifier("tagged-cluster")
            .tags(
                aws_sdk_redshift::types::Tag::builder()
                    .key("env")
                    .value("prod")
                    .build(),
            )
            .build();
        let rc = RedshiftCluster::from(cluster);
        assert_eq!(rc.identifier, "tagged-cluster");
        assert_eq!(rc.tags.len(), 1);
        assert_eq!(rc.tags[0].key, "env");
        assert_eq!(rc.tags[0].value, "prod");
    }

    #[test]
    fn test_tag_fields() {
        let tag = Tag {
            key: "team".to_string(),
            value: "data".to_string(),
        };
        assert_eq!(tag.key, "team");
        assert_eq!(tag.value, "data");
    }

    #[test]
    fn test_redshift_snapshot_from_sdk_minimal() {
        let snap = SdkSnapshot::builder()
            .snapshot_identifier("snap-001")
            .cluster_identifier("my-cluster")
            .snapshot_type("manual")
            .status("available")
            .encrypted(true)
            .build();
        let rs = RedshiftSnapshot::from(snap);
        assert_eq!(rs.id, "snap-001");
        assert_eq!(rs.cluster_identifier, Some("my-cluster".to_string()));
        assert_eq!(rs.snapshot_type, Some("manual".to_string()));
        assert_eq!(rs.status, Some("available".to_string()));
        assert!(rs.encrypted);
        assert!(rs.tags.is_empty());
        assert_eq!(rs.total_backup_size_in_mega_bytes, 0.0);
    }

    #[test]
    fn test_redshift_snapshot_from_sdk_with_tags() {
        let snap = SdkSnapshot::builder()
            .snapshot_identifier("snap-002")
            .cluster_identifier("prod-cluster")
            .snapshot_type("automated")
            .node_type("ra3.xlplus")
            .number_of_nodes(2)
            .cluster_version("1.0")
            .availability_zone("us-east-1a")
            .total_backup_size_in_mega_bytes(512.0)
            .encrypted(false)
            .master_username("admin")
            .tags(
                aws_sdk_redshift::types::Tag::builder()
                    .key("env")
                    .value("prod")
                    .build(),
            )
            .build();
        let rs = RedshiftSnapshot::from(snap);
        assert_eq!(rs.id, "snap-002");
        assert_eq!(rs.cluster_identifier, Some("prod-cluster".to_string()));
        assert_eq!(rs.snapshot_type, Some("automated".to_string()));
        assert_eq!(rs.node_type, Some("ra3.xlplus".to_string()));
        assert_eq!(rs.number_of_nodes, Some(2));
        assert_eq!(rs.cluster_version, Some("1.0".to_string()));
        assert_eq!(rs.availability_zone, Some("us-east-1a".to_string()));
        assert_eq!(rs.total_backup_size_in_mega_bytes, 512.0);
        assert!(!rs.encrypted);
        assert_eq!(rs.master_username, Some("admin".to_string()));
        assert_eq!(rs.tags.len(), 1);
        assert_eq!(rs.tags[0].key, "env");
        assert_eq!(rs.tags[0].value, "prod");
    }

    #[test]
    fn test_redshift_snapshot_encrypted_defaults_false() {
        let snap = SdkSnapshot::builder()
            .snapshot_identifier("snap-003")
            .build();
        let rs = RedshiftSnapshot::from(snap);
        assert!(!rs.encrypted);
        assert_eq!(rs.id, "snap-003");
        assert!(rs.cluster_identifier.is_none());
        assert!(rs.created_at.is_none());
    }
}
