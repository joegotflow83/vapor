use async_graphql::SimpleObject;

use crate::aws::mq::{MqBrokerInfo, MqBrokerInstanceInfo, MqConfigurationInfo};
use crate::schema::common::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct MqBrokerInstance {
    pub console_url: Option<String>,
    pub endpoints: Vec<String>,
    pub ip_address: Option<String>,
}

impl From<MqBrokerInstanceInfo> for MqBrokerInstance {
    fn from(i: MqBrokerInstanceInfo) -> Self {
        Self {
            console_url: i.console_url,
            endpoints: i.endpoints,
            ip_address: i.ip_address,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct MqBroker {
    pub broker_id: Option<String>,
    pub broker_arn: Option<String>,
    pub broker_name: Option<String>,
    pub broker_state: Option<String>,
    pub engine_type: Option<String>,
    pub engine_version: Option<String>,
    pub deployment_mode: Option<String>,
    pub host_instance_type: Option<String>,
    pub publicly_accessible: Option<bool>,
    pub broker_instances: Vec<MqBrokerInstance>,
    pub subnet_ids: Vec<String>,
    pub security_groups: Vec<String>,
    pub tags: Vec<Tag>,
}

impl From<MqBrokerInfo> for MqBroker {
    fn from(b: MqBrokerInfo) -> Self {
        Self {
            broker_id: b.broker_id,
            broker_arn: b.broker_arn,
            broker_name: b.broker_name,
            broker_state: b.broker_state,
            engine_type: b.engine_type,
            engine_version: b.engine_version,
            deployment_mode: b.deployment_mode,
            host_instance_type: b.host_instance_type,
            publicly_accessible: b.publicly_accessible,
            broker_instances: b.broker_instances.into_iter().map(MqBrokerInstance::from).collect(),
            subnet_ids: b.subnet_ids,
            security_groups: b.security_groups,
            tags: b.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct MqConfiguration {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub engine_type: Option<String>,
    pub engine_version: Option<String>,
    pub description: Option<String>,
    pub latest_revision: Option<i32>,
    pub created: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<MqConfigurationInfo> for MqConfiguration {
    fn from(c: MqConfigurationInfo) -> Self {
        Self {
            id: c.id,
            arn: c.arn,
            name: c.name,
            engine_type: c.engine_type,
            engine_version: c.engine_version,
            description: c.description,
            latest_revision: c.latest_revision,
            created: c.created,
            tags: c.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::mq::{MqBrokerInfo, MqBrokerInstanceInfo, MqConfigurationInfo};

    #[test]
    fn test_mq_broker_from_minimal() {
        let info = MqBrokerInfo {
            broker_id: Some("b-abc123".to_string()),
            broker_arn: Some("arn:aws:mq:us-east-1:123456789012:broker:my-broker:b-abc123".to_string()),
            broker_name: Some("my-broker".to_string()),
            broker_state: Some("RUNNING".to_string()),
            engine_type: Some("ACTIVEMQ".to_string()),
            engine_version: Some("5.15.14".to_string()),
            deployment_mode: Some("SINGLE_INSTANCE".to_string()),
            host_instance_type: Some("mq.m5.large".to_string()),
            publicly_accessible: Some(false),
            broker_instances: vec![],
            subnet_ids: vec!["subnet-aaa".to_string()],
            security_groups: vec!["sg-xyz".to_string()],
            tags: vec![],
        };
        let result = MqBroker::from(info);
        assert_eq!(result.broker_name, Some("my-broker".to_string()));
        assert_eq!(result.broker_state, Some("RUNNING".to_string()));
        assert_eq!(result.engine_type, Some("ACTIVEMQ".to_string()));
        assert_eq!(result.publicly_accessible, Some(false));
        assert!(result.broker_instances.is_empty());
        assert_eq!(result.subnet_ids, vec!["subnet-aaa"]);
        assert_eq!(result.security_groups, vec!["sg-xyz"]);
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_mq_broker_with_instances() {
        let info = MqBrokerInfo {
            broker_id: Some("b-def456".to_string()),
            broker_arn: None,
            broker_name: Some("ha-broker".to_string()),
            broker_state: Some("RUNNING".to_string()),
            engine_type: Some("RABBITMQ".to_string()),
            engine_version: Some("3.11.20".to_string()),
            deployment_mode: Some("CLUSTER_MULTI_AZ".to_string()),
            host_instance_type: None,
            publicly_accessible: Some(true),
            broker_instances: vec![
                MqBrokerInstanceInfo {
                    console_url: Some("https://console.mq.us-east-1.amazonaws.com/".to_string()),
                    endpoints: vec!["amqps://b-def456.mq.us-east-1.amazonaws.com:5671".to_string()],
                    ip_address: Some("10.0.1.100".to_string()),
                },
            ],
            subnet_ids: vec!["subnet-aaa".to_string(), "subnet-bbb".to_string()],
            security_groups: vec![],
            tags: vec![("Env".to_string(), "prod".to_string())],
        };
        let result = MqBroker::from(info);
        assert_eq!(result.engine_type, Some("RABBITMQ".to_string()));
        assert_eq!(result.deployment_mode, Some("CLUSTER_MULTI_AZ".to_string()));
        assert_eq!(result.broker_instances.len(), 1);
        let inst = &result.broker_instances[0];
        assert_eq!(inst.endpoints.len(), 1);
        assert_eq!(inst.ip_address, Some("10.0.1.100".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_mq_broker_instance_from() {
        let info = MqBrokerInstanceInfo {
            console_url: Some("https://console.mq.amazonaws.com/".to_string()),
            endpoints: vec!["amqp://broker.example.com:5671".to_string()],
            ip_address: None,
        };
        let result = MqBrokerInstance::from(info);
        assert_eq!(result.console_url, Some("https://console.mq.amazonaws.com/".to_string()));
        assert_eq!(result.endpoints.len(), 1);
        assert!(result.ip_address.is_none());
    }

    #[test]
    fn test_mq_configuration_from() {
        let info = MqConfigurationInfo {
            id: Some("c-abc123".to_string()),
            arn: Some("arn:aws:mq:us-east-1:123456789012:configuration:my-config:c-abc123".to_string()),
            name: Some("my-config".to_string()),
            engine_type: Some("ACTIVEMQ".to_string()),
            engine_version: Some("5.15.14".to_string()),
            description: Some("Production ActiveMQ config".to_string()),
            latest_revision: Some(3),
            created: Some("2024-01-01T00:00:00Z".to_string()),
            tags: vec![("Project".to_string(), "vapor".to_string())],
        };
        let result = MqConfiguration::from(info);
        assert_eq!(result.id, Some("c-abc123".to_string()));
        assert_eq!(result.name, Some("my-config".to_string()));
        assert_eq!(result.latest_revision, Some(3));
        assert_eq!(result.description, Some("Production ActiveMQ config".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Project");
    }

    #[test]
    fn test_mq_configuration_minimal() {
        let info = MqConfigurationInfo {
            id: None,
            arn: None,
            name: None,
            engine_type: None,
            engine_version: None,
            description: None,
            latest_revision: None,
            created: None,
            tags: vec![],
        };
        let result = MqConfiguration::from(info);
        assert!(result.id.is_none());
        assert!(result.name.is_none());
        assert!(result.latest_revision.is_none());
        assert!(result.tags.is_empty());
    }
}
