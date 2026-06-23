use async_graphql::SimpleObject;
use aws_sdk_memorydb::types::{Cluster as SdkCluster, SubnetGroup as SdkSubnetGroup, Tag as SdkTag};

#[derive(SimpleObject, Clone)]
#[graphql(name = "MemoryDbTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct MemoryDbCluster {
    pub name: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub node_type: Option<String>,
    pub engine_version: Option<String>,
    pub num_shards: Option<i32>,
    pub num_replicas_per_shard: Option<i32>,
    pub acl_name: Option<String>,
    pub subnet_group_name: Option<String>,
    pub tls_enabled: bool,
    pub kms_key_id: Option<String>,
    pub sns_topic_arn: Option<String>,
    pub tags: Vec<Tag>,
}

impl MemoryDbCluster {
    pub fn from_sdk(c: SdkCluster, sdk_tags: &[SdkTag]) -> Self {
        let num_shards = c.number_of_shards();
        let num_replicas_per_shard = c.shards().first().and_then(|s| {
            s.number_of_nodes().map(|n| n - 1)
        });
        let tags = sdk_tags
            .iter()
            .filter_map(|t| {
                Some(Tag {
                    key: t.key()?.to_string(),
                    value: t.value()?.to_string(),
                })
            })
            .collect();

        Self {
            name: c.name().unwrap_or_default().to_string(),
            arn: c.arn().map(|v| v.to_string()),
            status: c.status().map(|v| v.to_string()),
            node_type: c.node_type().map(|v| v.to_string()),
            engine_version: c.engine_version().map(|v| v.to_string()),
            num_shards,
            num_replicas_per_shard,
            acl_name: c.acl_name().map(|v| v.to_string()),
            subnet_group_name: c.subnet_group_name().map(|v| v.to_string()),
            tls_enabled: c.tls_enabled().unwrap_or(false),
            kms_key_id: c.kms_key_id().map(|v| v.to_string()),
            sns_topic_arn: c.sns_topic_arn().map(|v| v.to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct MemoryDbSubnetGroup {
    pub name: String,
    pub arn: Option<String>,
    pub description: Option<String>,
    pub vpc_id: Option<String>,
    pub subnet_ids: Vec<String>,
    pub tags: Vec<Tag>,
}

impl MemoryDbSubnetGroup {
    pub fn from_sdk(sg: SdkSubnetGroup, sdk_tags: &[SdkTag]) -> Self {
        let subnet_ids = sg.subnets()
            .iter()
            .filter_map(|s| s.identifier().map(|v| v.to_string()))
            .collect();
        let tags = sdk_tags
            .iter()
            .filter_map(|t| {
                Some(Tag {
                    key: t.key()?.to_string(),
                    value: t.value()?.to_string(),
                })
            })
            .collect();

        Self {
            name: sg.name().unwrap_or_default().to_string(),
            arn: sg.arn().map(|v| v.to_string()),
            description: sg.description().map(|v| v.to_string()),
            vpc_id: sg.vpc_id().map(|v| v.to_string()),
            subnet_ids,
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_memorydb::types::Tag as SdkTag;

    fn make_sdk_tags(pairs: &[(&str, &str)]) -> Vec<SdkTag> {
        pairs
            .iter()
            .map(|(k, v)| SdkTag::builder().key(*k).value(*v).build())
            .collect()
    }

    #[test]
    fn test_memorydb_cluster_from_sdk_minimal() {
        let cluster = SdkCluster::builder()
            .name("my-cluster")
            .arn("arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster")
            .status("available")
            .node_type("db.r6g.large")
            .tls_enabled(true)
            .build();
        let mdb = MemoryDbCluster::from_sdk(cluster, &[]);
        assert_eq!(mdb.name, "my-cluster");
        assert_eq!(mdb.arn, Some("arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster".to_string()));
        assert_eq!(mdb.status, Some("available".to_string()));
        assert_eq!(mdb.node_type, Some("db.r6g.large".to_string()));
        assert!(mdb.tls_enabled);
        assert!(mdb.tags.is_empty());
    }

    #[test]
    fn test_memorydb_cluster_from_sdk_with_tags() {
        let cluster = SdkCluster::builder()
            .name("tagged-cluster")
            .arn("arn:aws:memorydb:us-east-1:123456789012:cluster/tagged-cluster")
            .build();
        let sdk_tags = make_sdk_tags(&[("env", "prod"), ("team", "platform")]);
        let mdb = MemoryDbCluster::from_sdk(cluster, &sdk_tags);
        assert_eq!(mdb.tags.len(), 2);
        assert_eq!(mdb.tags[0].key, "env");
        assert_eq!(mdb.tags[0].value, "prod");
        assert_eq!(mdb.tags[1].key, "team");
        assert_eq!(mdb.tags[1].value, "platform");
    }

    #[test]
    fn test_memorydb_cluster_defaults() {
        let cluster = SdkCluster::builder().build();
        let mdb = MemoryDbCluster::from_sdk(cluster, &[]);
        assert_eq!(mdb.name, "");
        assert!(!mdb.tls_enabled);
        assert!(mdb.arn.is_none());
        assert!(mdb.num_shards.is_none());
    }

    #[test]
    fn test_memorydb_subnet_group_from_sdk() {
        let sg = SdkSubnetGroup::builder()
            .name("my-subnet-group")
            .arn("arn:aws:memorydb:us-east-1:123456789012:subnetgroup/my-subnet-group")
            .description("Test subnet group")
            .vpc_id("vpc-12345")
            .build();
        let mdb_sg = MemoryDbSubnetGroup::from_sdk(sg, &[]);
        assert_eq!(mdb_sg.name, "my-subnet-group");
        assert_eq!(mdb_sg.vpc_id, Some("vpc-12345".to_string()));
        assert_eq!(mdb_sg.description, Some("Test subnet group".to_string()));
        assert!(mdb_sg.subnet_ids.is_empty());
        assert!(mdb_sg.tags.is_empty());
    }

    #[test]
    fn test_memorydb_subnet_group_from_sdk_with_tags() {
        let sg = SdkSubnetGroup::builder()
            .name("tagged-sg")
            .arn("arn:aws:memorydb:us-east-1:123456789012:subnetgroup/tagged-sg")
            .build();
        let sdk_tags = make_sdk_tags(&[("env", "staging")]);
        let mdb_sg = MemoryDbSubnetGroup::from_sdk(sg, &sdk_tags);
        assert_eq!(mdb_sg.tags.len(), 1);
        assert_eq!(mdb_sg.tags[0].key, "env");
        assert_eq!(mdb_sg.tags[0].value, "staging");
    }

    #[test]
    fn test_memorydb_subnet_group_defaults() {
        let sg = SdkSubnetGroup::builder().build();
        let mdb_sg = MemoryDbSubnetGroup::from_sdk(sg, &[]);
        assert_eq!(mdb_sg.name, "");
        assert!(mdb_sg.vpc_id.is_none());
        assert!(mdb_sg.subnet_ids.is_empty());
        assert!(mdb_sg.tags.is_empty());
    }
}
