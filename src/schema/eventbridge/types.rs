use async_graphql::SimpleObject;
use aws_sdk_eventbridge::types::{EventBus, Rule, Target};

#[derive(SimpleObject)]
pub struct EbEventBus {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub description: Option<String>,
    pub policy: Option<String>,
    pub created_by: Option<String>,
}

impl From<&EventBus> for EbEventBus {
    fn from(b: &EventBus) -> Self {
        Self {
            name: b.name().map(|s| s.to_string()),
            arn: b.arn().map(|s| s.to_string()),
            description: b.description().map(|s| s.to_string()),
            policy: b.policy().map(|s| s.to_string()),
            // `EventBus` exposes no `created_by` field in the SDK; left unset.
            created_by: None,
        }
    }
}

#[derive(SimpleObject)]
pub struct EbRule {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub event_bus_name: Option<String>,
    pub state: Option<String>,
    pub description: Option<String>,
    pub schedule_expression: Option<String>,
    pub event_pattern: Option<String>,
    pub role_arn: Option<String>,
}

impl From<&Rule> for EbRule {
    fn from(r: &Rule) -> Self {
        Self {
            name: r.name().map(|s| s.to_string()),
            arn: r.arn().map(|s| s.to_string()),
            event_bus_name: r.event_bus_name().map(|s| s.to_string()),
            state: r.state().map(|s| s.as_str().to_string()),
            description: r.description().map(|s| s.to_string()),
            schedule_expression: r.schedule_expression().map(|s| s.to_string()),
            event_pattern: r.event_pattern().map(|s| s.to_string()),
            role_arn: r.role_arn().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject)]
pub struct EbTarget {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub role_arn: Option<String>,
    pub input: Option<String>,
    pub input_path: Option<String>,
}

impl From<&Target> for EbTarget {
    fn from(t: &Target) -> Self {
        Self {
            id: Some(t.id().to_string()),
            arn: Some(t.arn().to_string()),
            role_arn: t.role_arn().map(|s| s.to_string()),
            input: t.input().map(|s| s.to_string()),
            input_path: t.input_path().map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_eventbridge::types::RuleState;

    #[test]
    fn test_eb_event_bus_all_fields() {
        let sdk = EventBus::builder()
            .name("my-bus")
            .arn("arn:aws:events:us-east-1:123456789012:event-bus/my-bus")
            .description("My event bus")
            .policy("{\"Statement\":[]}")
            .build();

        let bus = EbEventBus::from(&sdk);
        assert_eq!(bus.name, Some("my-bus".to_string()));
        assert_eq!(bus.arn, Some("arn:aws:events:us-east-1:123456789012:event-bus/my-bus".to_string()));
        assert_eq!(bus.description, Some("My event bus".to_string()));
        assert_eq!(bus.policy, Some("{\"Statement\":[]}".to_string()));
        // `EventBus` no longer exposes `created_by` in the SDK; always None.
        assert_eq!(bus.created_by, None);
    }

    #[test]
    fn test_eb_event_bus_minimal() {
        let sdk = EventBus::builder().name("default").build();

        let bus = EbEventBus::from(&sdk);
        assert_eq!(bus.name, Some("default".to_string()));
        assert!(bus.arn.is_none());
        assert!(bus.description.is_none());
        assert!(bus.policy.is_none());
        assert!(bus.created_by.is_none());
    }

    #[test]
    fn test_eb_rule_all_fields() {
        let sdk = Rule::builder()
            .name("my-rule")
            .arn("arn:aws:events:us-east-1:123456789012:rule/my-rule")
            .event_bus_name("my-bus")
            .state(RuleState::Enabled)
            .description("My rule description")
            .schedule_expression("rate(5 minutes)")
            .event_pattern("{\"source\":[\"aws.ec2\"]}")
            .role_arn("arn:aws:iam::123456789012:role/MyRole")
            .build();

        let rule = EbRule::from(&sdk);
        assert_eq!(rule.name, Some("my-rule".to_string()));
        assert_eq!(rule.arn, Some("arn:aws:events:us-east-1:123456789012:rule/my-rule".to_string()));
        assert_eq!(rule.event_bus_name, Some("my-bus".to_string()));
        assert_eq!(rule.state, Some("ENABLED".to_string()));
        assert_eq!(rule.description, Some("My rule description".to_string()));
        assert_eq!(rule.schedule_expression, Some("rate(5 minutes)".to_string()));
        assert_eq!(rule.event_pattern, Some("{\"source\":[\"aws.ec2\"]}".to_string()));
        assert_eq!(rule.role_arn, Some("arn:aws:iam::123456789012:role/MyRole".to_string()));
    }

    #[test]
    fn test_eb_rule_disabled_state() {
        let sdk = Rule::builder()
            .name("disabled-rule")
            .state(RuleState::Disabled)
            .build();

        let rule = EbRule::from(&sdk);
        assert_eq!(rule.name, Some("disabled-rule".to_string()));
        assert_eq!(rule.state, Some("DISABLED".to_string()));
        assert!(rule.schedule_expression.is_none());
        assert!(rule.event_pattern.is_none());
    }

    #[test]
    fn test_eb_rule_minimal() {
        let sdk = Rule::builder().build();

        let rule = EbRule::from(&sdk);
        assert!(rule.name.is_none());
        assert!(rule.arn.is_none());
        assert!(rule.state.is_none());
        assert!(rule.role_arn.is_none());
    }

    #[test]
    fn test_eb_target_all_fields() {
        let sdk = Target::builder()
            .id("my-target")
            .arn("arn:aws:lambda:us-east-1:123456789012:function:my-function")
            .role_arn("arn:aws:iam::123456789012:role/MyRole")
            .input("{\"key\": \"value\"}")
            .input_path("$.body")
            .build()
            .unwrap();

        let target = EbTarget::from(&sdk);
        assert_eq!(target.id, Some("my-target".to_string()));
        assert_eq!(target.arn, Some("arn:aws:lambda:us-east-1:123456789012:function:my-function".to_string()));
        assert_eq!(target.role_arn, Some("arn:aws:iam::123456789012:role/MyRole".to_string()));
        assert_eq!(target.input, Some("{\"key\": \"value\"}".to_string()));
        assert_eq!(target.input_path, Some("$.body".to_string()));
    }

    #[test]
    fn test_eb_target_minimal() {
        let sdk = Target::builder()
            .id("simple-target")
            .arn("arn:aws:sqs:us-east-1:123456789012:my-queue")
            .build()
            .unwrap();

        let target = EbTarget::from(&sdk);
        assert_eq!(target.id, Some("simple-target".to_string()));
        assert_eq!(target.arn, Some("arn:aws:sqs:us-east-1:123456789012:my-queue".to_string()));
        assert!(target.role_arn.is_none());
        assert!(target.input.is_none());
        assert!(target.input_path.is_none());
    }
}
