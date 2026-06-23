use async_graphql::SimpleObject;
use aws_sdk_elasticache::types::{CacheCluster, CacheSubnetGroup, ReplicationGroup, Tag as SdkTag};

use crate::schema::ec2::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct ElastiCacheSecurityGroup {
    pub security_group_id: Option<String>,
    pub status: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct ElastiCacheCluster {
    pub cache_cluster_id: Option<String>,
    pub cache_node_type: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    pub cache_cluster_status: Option<String>,
    pub num_cache_nodes: Option<i32>,
    pub preferred_availability_zone: Option<String>,
    pub replication_group_id: Option<String>,
    pub cache_subnet_group_name: Option<String>,
    pub auto_minor_version_upgrade: Option<bool>,
    pub at_rest_encryption_enabled: Option<bool>,
    pub transit_encryption_enabled: Option<bool>,
    pub security_groups: Vec<ElastiCacheSecurityGroup>,
    pub tags: Vec<Tag>,
}

impl ElastiCacheCluster {
    pub fn from_sdk(cluster: &CacheCluster, sdk_tags: &[SdkTag]) -> Self {
        let security_groups = cluster
            .security_groups()
            .iter()
            .map(|sg| ElastiCacheSecurityGroup {
                security_group_id: sg.security_group_id().map(|s| s.to_string()),
                status: sg.status().map(|s| s.to_string()),
            })
            .collect();

        let tags = sdk_tags
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or("").to_string(),
                value: t.value().unwrap_or("").to_string(),
            })
            .collect();

        Self {
            cache_cluster_id: cluster.cache_cluster_id().map(|s| s.to_string()),
            cache_node_type: cluster.cache_node_type().map(|s| s.to_string()),
            engine: cluster.engine().map(|s| s.to_string()),
            engine_version: cluster.engine_version().map(|s| s.to_string()),
            cache_cluster_status: cluster.cache_cluster_status().map(|s| s.to_string()),
            num_cache_nodes: cluster.num_cache_nodes(),
            preferred_availability_zone: cluster
                .preferred_availability_zone()
                .map(|s| s.to_string()),
            replication_group_id: cluster.replication_group_id().map(|s| s.to_string()),
            cache_subnet_group_name: cluster.cache_subnet_group_name().map(|s| s.to_string()),
            auto_minor_version_upgrade: cluster.auto_minor_version_upgrade(),
            at_rest_encryption_enabled: cluster.at_rest_encryption_enabled(),
            transit_encryption_enabled: cluster.transit_encryption_enabled(),
            security_groups,
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ElastiCacheReplicationGroup {
    pub replication_group_id: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub member_clusters: Vec<String>,
    pub automatic_failover: Option<String>,
    pub multi_az: Option<String>,
    pub snapshotting_cluster_id: Option<String>,
    pub cluster_mode: Option<String>,
    pub at_rest_encryption_enabled: Option<bool>,
    pub transit_encryption_enabled: Option<bool>,
}

impl From<&ReplicationGroup> for ElastiCacheReplicationGroup {
    fn from(rg: &ReplicationGroup) -> Self {
        Self {
            replication_group_id: rg.replication_group_id().map(|s| s.to_string()),
            description: rg.description().map(|s| s.to_string()),
            status: rg.status().map(|s| s.to_string()),
            member_clusters: rg.member_clusters().iter().map(|s| s.to_string()).collect(),
            automatic_failover: rg
                .automatic_failover()
                .map(|v| v.as_str().to_string()),
            multi_az: rg.multi_az().map(|v| v.as_str().to_string()),
            snapshotting_cluster_id: rg.snapshotting_cluster_id().map(|s| s.to_string()),
            cluster_mode: rg.cluster_mode().map(|v| v.as_str().to_string()),
            at_rest_encryption_enabled: rg.at_rest_encryption_enabled(),
            transit_encryption_enabled: rg.transit_encryption_enabled(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ElastiCacheSubnetGroup {
    pub cache_subnet_group_name: Option<String>,
    pub cache_subnet_group_description: Option<String>,
    pub vpc_id: Option<String>,
    pub subnet_ids: Vec<String>,
}

impl From<&CacheSubnetGroup> for ElastiCacheSubnetGroup {
    fn from(sg: &CacheSubnetGroup) -> Self {
        let subnet_ids = sg
            .subnets()
            .iter()
            .filter_map(|s| s.subnet_identifier().map(|id| id.to_string()))
            .collect();

        Self {
            cache_subnet_group_name: sg.cache_subnet_group_name().map(|s| s.to_string()),
            cache_subnet_group_description: sg
                .cache_subnet_group_description()
                .map(|s| s.to_string()),
            vpc_id: sg.vpc_id().map(|s| s.to_string()),
            subnet_ids,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elasticache_security_group_fields() {
        let sg = ElastiCacheSecurityGroup {
            security_group_id: Some("sg-abc123".to_string()),
            status: Some("active".to_string()),
        };
        assert_eq!(sg.security_group_id, Some("sg-abc123".to_string()));
        assert_eq!(sg.status, Some("active".to_string()));
    }

    #[test]
    fn test_elasticache_cluster_from_sdk_no_tags() {
        // Build a minimal CacheCluster via builder
        let cluster = aws_sdk_elasticache::types::CacheCluster::builder()
            .cache_cluster_id("my-cluster")
            .engine("redis")
            .cache_cluster_status("available")
            .build();

        let result = ElastiCacheCluster::from_sdk(&cluster, &[]);
        assert_eq!(result.cache_cluster_id, Some("my-cluster".to_string()));
        assert_eq!(result.engine, Some("redis".to_string()));
        assert_eq!(result.cache_cluster_status, Some("available".to_string()));
        assert!(result.tags.is_empty());
        assert!(result.security_groups.is_empty());
    }

    #[test]
    fn test_elasticache_cluster_with_tags() {
        let cluster = aws_sdk_elasticache::types::CacheCluster::builder()
            .cache_cluster_id("tagged-cluster")
            .build();

        let sdk_tags = vec![
            SdkTag::builder().key("Env").value("prod").build(),
            SdkTag::builder().key("Team").value("infra").build(),
        ];

        let result = ElastiCacheCluster::from_sdk(&cluster, &sdk_tags);
        assert_eq!(result.tags.len(), 2);
        assert_eq!(result.tags[0].key, "Env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_elasticache_subnet_group_from_sdk() {
        let sg = aws_sdk_elasticache::types::CacheSubnetGroup::builder()
            .cache_subnet_group_name("my-subnet-group")
            .cache_subnet_group_description("Test group")
            .vpc_id("vpc-12345")
            .build();

        let result = ElastiCacheSubnetGroup::from(&sg);
        assert_eq!(
            result.cache_subnet_group_name,
            Some("my-subnet-group".to_string())
        );
        assert_eq!(result.vpc_id, Some("vpc-12345".to_string()));
        assert!(result.subnet_ids.is_empty());
    }
}
