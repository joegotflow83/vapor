use async_graphql::SimpleObject;
use aws_sdk_kafka::types::{Cluster as SdkCluster, NodeInfo as SdkNodeInfo};

#[derive(SimpleObject, Clone)]
#[graphql(name = "MskTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct MskCluster {
    pub arn: String,
    pub name: String,
    pub state: Option<String>,
    pub cluster_type: Option<String>,
    pub kafka_version: Option<String>,
    pub broker_count: Option<i32>,
    pub broker_instance_type: Option<String>,
    pub storage_gb: Option<i32>,
    pub enhanced_monitoring: Option<String>,
    pub created_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<SdkCluster> for MskCluster {
    fn from(c: SdkCluster) -> Self {
        let (kafka_version, broker_count, broker_instance_type, storage_gb, enhanced_monitoring) =
            match c.provisioned() {
                Some(p) => (
                    p.current_broker_software_info()
                        .and_then(|i| i.kafka_version().map(|v| v.to_string())),
                    p.number_of_broker_nodes(),
                    p.broker_node_group_info()
                        .and_then(|b| b.instance_type().map(|v| v.to_string())),
                    p.broker_node_group_info()
                        .and_then(|b| b.storage_info())
                        .and_then(|s| s.ebs_storage_info())
                        .and_then(|e| e.volume_size()),
                    p.enhanced_monitoring().map(|e| e.as_str().to_string()),
                ),
                None => (None, None, None, None, None),
            };

        let tags: Vec<Tag> = match c.tags() {
            Some(map) => map
                .iter()
                .map(|(k, v)| Tag {
                    key: k.clone(),
                    value: v.clone(),
                })
                .collect(),
            None => Vec::new(),
        };

        Self {
            arn: c.cluster_arn().unwrap_or_default().to_string(),
            name: c.cluster_name().unwrap_or_default().to_string(),
            state: c.state().map(|s| s.as_str().to_string()),
            cluster_type: c.cluster_type().map(|t| t.as_str().to_string()),
            kafka_version,
            broker_count,
            broker_instance_type,
            storage_gb,
            enhanced_monitoring,
            created_at: c.creation_time().map(|t| t.to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BrokerNode {
    pub broker_id: Option<f64>,
    pub instance_type: Option<String>,
    pub az: Option<String>,
    pub client_vpc_ip: Option<String>,
    pub attached_eni_id: Option<String>,
}

impl From<SdkNodeInfo> for BrokerNode {
    fn from(n: SdkNodeInfo) -> Self {
        Self {
            broker_id: n.broker_node_info().and_then(|b| b.broker_id()),
            instance_type: n.instance_type().map(|v| v.to_string()),
            az: None,
            client_vpc_ip: n.broker_node_info().and_then(|b| b.client_vpc_ip_address().map(|v| v.to_string())),
            attached_eni_id: n.broker_node_info().and_then(|b| b.attached_eni_id().map(|v| v.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msk_cluster_from_sdk_minimal() {
        let cluster = SdkCluster::builder()
            .cluster_arn("arn:aws:kafka:us-east-1:123456789012:cluster/my-cluster/abc-123")
            .cluster_name("my-cluster")
            .state(aws_sdk_kafka::types::ClusterState::Active)
            .cluster_type(aws_sdk_kafka::types::ClusterType::Provisioned)
            .build();
        let msk = MskCluster::from(cluster);
        assert_eq!(msk.arn, "arn:aws:kafka:us-east-1:123456789012:cluster/my-cluster/abc-123");
        assert_eq!(msk.name, "my-cluster");
        assert_eq!(msk.state, Some("ACTIVE".to_string()));
        assert_eq!(msk.cluster_type, Some("PROVISIONED".to_string()));
        assert!(msk.tags.is_empty());
    }

    #[test]
    fn test_msk_cluster_with_tags() {
        let cluster = SdkCluster::builder()
            .cluster_arn("arn:aws:kafka:us-east-1:123456789012:cluster/tagged/def-456")
            .cluster_name("tagged")
            .tags("env", "prod")
            .tags("team", "platform")
            .build();
        let msk = MskCluster::from(cluster);
        assert_eq!(msk.name, "tagged");
        assert_eq!(msk.tags.len(), 2);
    }

    #[test]
    fn test_broker_node_from_sdk_no_info() {
        let node = SdkNodeInfo::builder().build();
        let broker = BrokerNode::from(node);
        assert!(broker.broker_id.is_none());
        assert!(broker.client_vpc_ip.is_none());
        assert!(broker.attached_eni_id.is_none());
    }

    #[test]
    fn test_tag_fields() {
        let tag = Tag {
            key: "env".to_string(),
            value: "staging".to_string(),
        };
        assert_eq!(tag.key, "env");
        assert_eq!(tag.value, "staging");
    }
}
