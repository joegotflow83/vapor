use async_graphql::SimpleObject;

use crate::aws::connect::{ConnectContactFlowInfo, ConnectInstanceInfo, ConnectQueueInfo, ConnectUserInfo};

#[derive(SimpleObject, Clone)]
pub struct ConnectInstance {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub identity_management_type: Option<String>,
    pub instance_alias: Option<String>,
    pub created_time: Option<String>,
    pub service_role: Option<String>,
    pub instance_status: Option<String>,
    pub inbound_calls_enabled: Option<bool>,
    pub outbound_calls_enabled: Option<bool>,
}

impl From<ConnectInstanceInfo> for ConnectInstance {
    fn from(i: ConnectInstanceInfo) -> Self {
        Self {
            id: i.id,
            arn: i.arn,
            identity_management_type: i.identity_management_type,
            instance_alias: i.instance_alias,
            created_time: i.created_time,
            service_role: i.service_role,
            instance_status: i.instance_status,
            inbound_calls_enabled: i.inbound_calls_enabled,
            outbound_calls_enabled: i.outbound_calls_enabled,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ConnectQueue {
    pub queue_id: Option<String>,
    pub queue_arn: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub queue_type: Option<String>,
    pub status: Option<String>,
}

impl From<ConnectQueueInfo> for ConnectQueue {
    fn from(i: ConnectQueueInfo) -> Self {
        Self {
            queue_id: i.queue_id,
            queue_arn: i.queue_arn,
            name: i.name,
            description: i.description,
            queue_type: i.queue_type,
            status: i.status,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ConnectContactFlow {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub contact_flow_type: Option<String>,
    pub description: Option<String>,
}

impl From<ConnectContactFlowInfo> for ConnectContactFlow {
    fn from(i: ConnectContactFlowInfo) -> Self {
        Self {
            id: i.id,
            arn: i.arn,
            name: i.name,
            contact_flow_type: i.contact_flow_type,
            description: i.description,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ConnectUser {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub username: Option<String>,
    pub routing_profile_id: Option<String>,
    pub hierarchy_group_id: Option<String>,
    pub security_profile_ids: Vec<String>,
}

impl From<ConnectUserInfo> for ConnectUser {
    fn from(i: ConnectUserInfo) -> Self {
        Self {
            id: i.id,
            arn: i.arn,
            username: i.username,
            routing_profile_id: i.routing_profile_id,
            hierarchy_group_id: i.hierarchy_group_id,
            security_profile_ids: i.security_profile_ids,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::connect::{ConnectContactFlowInfo, ConnectInstanceInfo, ConnectQueueInfo, ConnectUserInfo};

    #[test]
    fn test_connect_instance_from_full() {
        let info = ConnectInstanceInfo {
            id: Some("inst-123".to_string()),
            arn: Some("arn:aws:connect:us-east-1:123456789:instance/inst-123".to_string()),
            identity_management_type: Some("CONNECT_MANAGED".to_string()),
            instance_alias: Some("my-contact-center".to_string()),
            created_time: Some("2024-01-01T00:00:00Z".to_string()),
            service_role: Some("arn:aws:iam::123456789:role/aws-service-role/connect.amazonaws.com/AWSServiceRoleForAmazonConnect".to_string()),
            instance_status: Some("ACTIVE".to_string()),
            inbound_calls_enabled: Some(true),
            outbound_calls_enabled: Some(false),
        };
        let result = ConnectInstance::from(info);
        assert_eq!(result.id, Some("inst-123".to_string()));
        assert_eq!(result.instance_alias, Some("my-contact-center".to_string()));
        assert_eq!(result.instance_status, Some("ACTIVE".to_string()));
        assert_eq!(result.inbound_calls_enabled, Some(true));
        assert_eq!(result.outbound_calls_enabled, Some(false));
    }

    #[test]
    fn test_connect_instance_from_minimal() {
        let info = ConnectInstanceInfo {
            id: None,
            arn: None,
            identity_management_type: None,
            instance_alias: None,
            created_time: None,
            service_role: None,
            instance_status: None,
            inbound_calls_enabled: None,
            outbound_calls_enabled: None,
        };
        let result = ConnectInstance::from(info);
        assert!(result.id.is_none());
        assert!(result.arn.is_none());
        assert!(result.instance_status.is_none());
    }

    #[test]
    fn test_connect_queue_from_full() {
        let info = ConnectQueueInfo {
            queue_id: Some("queue-456".to_string()),
            queue_arn: Some("arn:aws:connect:us-east-1:123456789:instance/inst-123/queue/queue-456".to_string()),
            name: Some("BasicQueue".to_string()),
            description: Some("Main support queue".to_string()),
            queue_type: Some("STANDARD".to_string()),
            status: Some("ENABLED".to_string()),
        };
        let result = ConnectQueue::from(info);
        assert_eq!(result.queue_id, Some("queue-456".to_string()));
        assert_eq!(result.name, Some("BasicQueue".to_string()));
        assert_eq!(result.queue_type, Some("STANDARD".to_string()));
        assert_eq!(result.status, Some("ENABLED".to_string()));
    }

    #[test]
    fn test_connect_queue_from_minimal() {
        let info = ConnectQueueInfo {
            queue_id: None,
            queue_arn: None,
            name: None,
            description: None,
            queue_type: None,
            status: None,
        };
        let result = ConnectQueue::from(info);
        assert!(result.queue_id.is_none());
        assert!(result.name.is_none());
        assert!(result.status.is_none());
    }

    #[test]
    fn test_connect_contact_flow_from_full() {
        let info = ConnectContactFlowInfo {
            id: Some("flow-789".to_string()),
            arn: Some("arn:aws:connect:us-east-1:123456789:instance/inst-123/contact-flow/flow-789".to_string()),
            name: Some("Default inbound flow".to_string()),
            contact_flow_type: Some("CONTACT_FLOW".to_string()),
            description: None,
        };
        let result = ConnectContactFlow::from(info);
        assert_eq!(result.id, Some("flow-789".to_string()));
        assert_eq!(result.name, Some("Default inbound flow".to_string()));
        assert_eq!(result.contact_flow_type, Some("CONTACT_FLOW".to_string()));
        assert!(result.description.is_none());
    }

    #[test]
    fn test_connect_contact_flow_from_minimal() {
        let info = ConnectContactFlowInfo {
            id: None,
            arn: None,
            name: None,
            contact_flow_type: None,
            description: None,
        };
        let result = ConnectContactFlow::from(info);
        assert!(result.id.is_none());
        assert!(result.contact_flow_type.is_none());
    }

    #[test]
    fn test_connect_user_from_full() {
        let info = ConnectUserInfo {
            id: Some("user-abc".to_string()),
            arn: Some("arn:aws:connect:us-east-1:123456789:instance/inst-123/agent/user-abc".to_string()),
            username: Some("agent.smith".to_string()),
            routing_profile_id: Some("rp-123".to_string()),
            hierarchy_group_id: Some("hg-456".to_string()),
            security_profile_ids: vec!["sp-111".to_string(), "sp-222".to_string()],
        };
        let result = ConnectUser::from(info);
        assert_eq!(result.id, Some("user-abc".to_string()));
        assert_eq!(result.username, Some("agent.smith".to_string()));
        assert_eq!(result.routing_profile_id, Some("rp-123".to_string()));
        assert_eq!(result.security_profile_ids.len(), 2);
        assert_eq!(result.security_profile_ids[0], "sp-111");
    }

    #[test]
    fn test_connect_user_from_minimal() {
        let info = ConnectUserInfo {
            id: None,
            arn: None,
            username: None,
            routing_profile_id: None,
            hierarchy_group_id: None,
            security_profile_ids: vec![],
        };
        let result = ConnectUser::from(info);
        assert!(result.id.is_none());
        assert!(result.username.is_none());
        assert!(result.security_profile_ids.is_empty());
    }
}
