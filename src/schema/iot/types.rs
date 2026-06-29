use async_graphql::SimpleObject;

use crate::aws::iot::{
    IotCertificateInfo, IotPolicyInfo, IotTagPair, IotThingGroupInfo, IotThingInfo,
    IotTopicRuleInfo,
};

#[derive(SimpleObject, Clone)]
#[graphql(name = "IotTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

impl From<IotTagPair> for Tag {
    fn from(p: IotTagPair) -> Self {
        Self {
            key: p.key,
            value: p.value,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct IotThing {
    pub thing_name: Option<String>,
    pub thing_arn: Option<String>,
    pub thing_type_name: Option<String>,
    pub attributes: Vec<Tag>,
    pub version: Option<i64>,
}

impl From<IotThingInfo> for IotThing {
    fn from(i: IotThingInfo) -> Self {
        Self {
            thing_name: i.thing_name,
            thing_arn: i.thing_arn,
            thing_type_name: i.thing_type_name,
            attributes: i.attributes.into_iter().map(Tag::from).collect(),
            version: i.version,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct IotThingGroup {
    pub group_name: Option<String>,
    pub group_arn: Option<String>,
    pub group_id: Option<String>,
    pub status: Option<String>,
}

impl From<IotThingGroupInfo> for IotThingGroup {
    fn from(i: IotThingGroupInfo) -> Self {
        Self {
            group_name: i.group_name,
            group_arn: i.group_arn,
            group_id: i.group_id,
            status: i.status,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct IotPolicy {
    pub policy_name: Option<String>,
    pub policy_arn: Option<String>,
}

impl From<IotPolicyInfo> for IotPolicy {
    fn from(i: IotPolicyInfo) -> Self {
        Self {
            policy_name: i.policy_name,
            policy_arn: i.policy_arn,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct IotCertificate {
    pub certificate_id: Option<String>,
    pub certificate_arn: Option<String>,
    pub status: Option<String>,
    pub creation_date: Option<String>,
}

impl From<IotCertificateInfo> for IotCertificate {
    fn from(i: IotCertificateInfo) -> Self {
        Self {
            certificate_id: i.certificate_id,
            certificate_arn: i.certificate_arn,
            status: i.status,
            creation_date: i.creation_date,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct IotTopicRule {
    pub rule_name: Option<String>,
    pub topic_pattern: Option<String>,
    pub created_at: Option<String>,
    pub rule_disabled: Option<bool>,
    pub rule_arn: Option<String>,
}

impl From<IotTopicRuleInfo> for IotTopicRule {
    fn from(i: IotTopicRuleInfo) -> Self {
        Self {
            rule_name: i.rule_name,
            topic_pattern: i.topic_pattern,
            created_at: i.created_at,
            rule_disabled: i.rule_disabled,
            rule_arn: i.rule_arn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::iot::{
        IotCertificateInfo, IotPolicyInfo, IotTagPair, IotThingGroupInfo, IotThingInfo,
        IotTopicRuleInfo,
    };

    #[test]
    fn test_thing_from_full() {
        let info = IotThingInfo {
            thing_name: Some("sensor-01".to_string()),
            thing_arn: Some("arn:aws:iot:us-east-1:123456789012:thing/sensor-01".to_string()),
            thing_type_name: Some("TemperatureSensor".to_string()),
            attributes: vec![IotTagPair {
                key: "location".to_string(),
                value: "building-a".to_string(),
            }],
            version: Some(1),
        };
        let result = IotThing::from(info);
        assert_eq!(result.thing_name, Some("sensor-01".to_string()));
        assert_eq!(result.thing_type_name, Some("TemperatureSensor".to_string()));
        assert_eq!(result.attributes.len(), 1);
        assert_eq!(result.attributes[0].key, "location");
        assert_eq!(result.attributes[0].value, "building-a");
        assert_eq!(result.version, Some(1));
    }

    #[test]
    fn test_thing_from_minimal() {
        let info = IotThingInfo {
            thing_name: Some("device-x".to_string()),
            thing_arn: None,
            thing_type_name: None,
            attributes: vec![],
            version: None,
        };
        let result = IotThing::from(info);
        assert_eq!(result.thing_name, Some("device-x".to_string()));
        assert!(result.thing_arn.is_none());
        assert!(result.attributes.is_empty());
        assert!(result.version.is_none());
    }

    #[test]
    fn test_thing_group_from() {
        let info = IotThingGroupInfo {
            group_name: Some("factory-floor".to_string()),
            group_arn: Some("arn:aws:iot:us-east-1:123456789012:thinggroup/factory-floor".to_string()),
            group_id: Some("group-abc123".to_string()),
            status: Some("ACTIVE".to_string()),
        };
        let result = IotThingGroup::from(info);
        assert_eq!(result.group_name, Some("factory-floor".to_string()));
        assert_eq!(result.group_id, Some("group-abc123".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
    }

    #[test]
    fn test_thing_group_from_no_extras() {
        let info = IotThingGroupInfo {
            group_name: Some("basic-group".to_string()),
            group_arn: Some("arn:aws:iot:us-east-1:123456789012:thinggroup/basic-group".to_string()),
            group_id: None,
            status: None,
        };
        let result = IotThingGroup::from(info);
        assert_eq!(result.group_name, Some("basic-group".to_string()));
        assert!(result.group_id.is_none());
        assert!(result.status.is_none());
    }

    #[test]
    fn test_policy_from() {
        let info = IotPolicyInfo {
            policy_name: Some("AllowPublish".to_string()),
            policy_arn: Some("arn:aws:iot:us-east-1:123456789012:policy/AllowPublish".to_string()),
        };
        let result = IotPolicy::from(info);
        assert_eq!(result.policy_name, Some("AllowPublish".to_string()));
        assert!(result.policy_arn.is_some());
    }

    #[test]
    fn test_certificate_from() {
        let info = IotCertificateInfo {
            certificate_id: Some("abc123def456".to_string()),
            certificate_arn: Some("arn:aws:iot:us-east-1:123456789012:cert/abc123".to_string()),
            status: Some("ACTIVE".to_string()),
            creation_date: Some("2024-01-15T10:00:00Z".to_string()),
        };
        let result = IotCertificate::from(info);
        assert_eq!(result.certificate_id, Some("abc123def456".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert!(result.creation_date.is_some());
    }

    #[test]
    fn test_certificate_from_minimal() {
        let info = IotCertificateInfo {
            certificate_id: None,
            certificate_arn: None,
            status: Some("INACTIVE".to_string()),
            creation_date: None,
        };
        let result = IotCertificate::from(info);
        assert!(result.certificate_id.is_none());
        assert_eq!(result.status, Some("INACTIVE".to_string()));
    }

    #[test]
    fn test_topic_rule_from() {
        let info = IotTopicRuleInfo {
            rule_name: Some("temperature-alert".to_string()),
            topic_pattern: Some("sensors/+/temperature".to_string()),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            rule_disabled: Some(false),
            rule_arn: Some("arn:aws:iot:us-east-1:123456789012:rule/temperature-alert".to_string()),
        };
        let result = IotTopicRule::from(info);
        assert_eq!(result.rule_name, Some("temperature-alert".to_string()));
        assert_eq!(result.topic_pattern, Some("sensors/+/temperature".to_string()));
        assert_eq!(result.rule_disabled, Some(false));
        assert!(result.rule_arn.is_some());
    }

    #[test]
    fn test_topic_rule_disabled() {
        let info = IotTopicRuleInfo {
            rule_name: Some("disabled-rule".to_string()),
            topic_pattern: None,
            created_at: None,
            rule_disabled: Some(true),
            rule_arn: None,
        };
        let result = IotTopicRule::from(info);
        assert_eq!(result.rule_disabled, Some(true));
        assert!(result.topic_pattern.is_none());
    }
}
