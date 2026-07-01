use async_graphql::SimpleObject;

use crate::schema::common::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct Firewall {
    pub firewall_name: String,
    pub firewall_arn: Option<String>,
    pub firewall_policy_arn: Option<String>,
    pub vpc_id: Option<String>,
    pub subnet_mappings: Vec<String>,
    pub delete_protection: bool,
    pub policy_change_protection: bool,
    pub description: Option<String>,
    pub firewall_status: Option<String>,
    pub tags: Vec<Tag>,
}

#[derive(SimpleObject, Clone)]
pub struct FirewallPolicy {
    pub name: String,
    pub arn: Option<String>,
    pub description: Option<String>,
    pub firewall_policy_status: Option<String>,
    pub stateless_default_actions: Vec<String>,
    pub stateless_fragment_default_actions: Vec<String>,
    pub stateless_rule_group_arns: Vec<String>,
    pub stateful_rule_group_arns: Vec<String>,
    pub stateful_default_actions: Vec<String>,
    pub tags: Vec<Tag>,
}

#[derive(SimpleObject, Clone)]
pub struct RuleGroup {
    pub name: String,
    pub arn: Option<String>,
    pub rule_group_type: Option<String>,
    pub description: Option<String>,
    pub capacity: Option<i32>,
    pub rule_group_status: Option<String>,
    pub consumed_capacity: Option<i32>,
    pub number_of_associations: Option<i32>,
    pub tags: Vec<Tag>,
}

fn sdk_tag_to_tag(t: &aws_sdk_networkfirewall::types::Tag) -> Tag {
    Tag {
        key: t.key().to_string(),
        value: t.value().to_string(),
    }
}

impl
    From<
        aws_sdk_networkfirewall::operation::describe_firewall::DescribeFirewallOutput,
    > for Firewall
{
    fn from(
        output: aws_sdk_networkfirewall::operation::describe_firewall::DescribeFirewallOutput,
    ) -> Self {
        let firewall_status = output
            .firewall_status()
            .map(|s| s.status())
            .map(|s| s.as_str().to_string());

        if let Some(fw) = output.firewall() {
            Self {
                firewall_name: fw.firewall_name().unwrap_or_default().to_string(),
                firewall_arn: fw.firewall_arn().map(|s| s.to_string()),
                firewall_policy_arn: Some(fw.firewall_policy_arn().to_string()),
                vpc_id: Some(fw.vpc_id().to_string()),
                subnet_mappings: fw
                    .subnet_mappings()
                    .iter()
                    .map(|s| s.subnet_id().to_string())
                    .collect(),
                delete_protection: fw.delete_protection(),
                policy_change_protection: fw.firewall_policy_change_protection(),
                description: fw.description().map(|s| s.to_string()),
                firewall_status,
                tags: fw.tags().iter().map(sdk_tag_to_tag).collect(),
            }
        } else {
            Self {
                firewall_name: String::new(),
                firewall_arn: None,
                firewall_policy_arn: None,
                vpc_id: None,
                subnet_mappings: vec![],
                delete_protection: false,
                policy_change_protection: false,
                description: None,
                firewall_status,
                tags: vec![],
            }
        }
    }
}

impl
    From<
        aws_sdk_networkfirewall::operation::describe_firewall_policy::DescribeFirewallPolicyOutput,
    > for FirewallPolicy
{
    fn from(
        output: aws_sdk_networkfirewall::operation::describe_firewall_policy::DescribeFirewallPolicyOutput,
    ) -> Self {
        let resp = output.firewall_policy_response();
        let policy = output.firewall_policy();

        let stateless_default_actions = policy
            .map(|p| p.stateless_default_actions().to_vec())
            .unwrap_or_default();
        let stateless_fragment_default_actions = policy
            .map(|p| p.stateless_fragment_default_actions().to_vec())
            .unwrap_or_default();
        let stateless_rule_group_arns = policy
            .map(|p| {
                p.stateless_rule_group_references()
                    .iter()
                    .map(|r| r.resource_arn().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let stateful_rule_group_arns = policy
            .map(|p| {
                p.stateful_rule_group_references()
                    .iter()
                    .map(|r| r.resource_arn().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let stateful_default_actions = policy
            .map(|p| p.stateful_default_actions().to_vec())
            .unwrap_or_default();

        Self {
            name: resp
                .map(|r| r.firewall_policy_name().to_string())
                .unwrap_or_default(),
            arn: resp.map(|r| r.firewall_policy_arn().to_string()),
            description: resp.and_then(|r| r.description()).map(|s| s.to_string()),
            firewall_policy_status: resp
                .and_then(|r| r.firewall_policy_status())
                .map(|s| s.as_str().to_string()),
            stateless_default_actions,
            stateless_fragment_default_actions,
            stateless_rule_group_arns,
            stateful_rule_group_arns,
            stateful_default_actions,
            tags: resp
                .map(|r| r.tags().iter().map(sdk_tag_to_tag).collect())
                .unwrap_or_default(),
        }
    }
}

impl
    From<
        aws_sdk_networkfirewall::operation::describe_rule_group::DescribeRuleGroupOutput,
    > for RuleGroup
{
    fn from(
        output: aws_sdk_networkfirewall::operation::describe_rule_group::DescribeRuleGroupOutput,
    ) -> Self {
        let resp = output.rule_group_response();
        Self {
            name: resp
                .map(|r| r.rule_group_name().to_string())
                .unwrap_or_default(),
            arn: resp.map(|r| r.rule_group_arn().to_string()),
            rule_group_type: resp
                .and_then(|r| r.r#type())
                .map(|t| t.as_str().to_string()),
            description: resp.and_then(|r| r.description()).map(|s| s.to_string()),
            capacity: resp.and_then(|r| r.capacity()),
            rule_group_status: resp
                .and_then(|r| r.rule_group_status())
                .map(|s| s.as_str().to_string()),
            consumed_capacity: resp.and_then(|r| r.consumed_capacity()),
            number_of_associations: resp.and_then(|r| r.number_of_associations()),
            tags: resp
                .map(|r| r.tags().iter().map(sdk_tag_to_tag).collect())
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firewall_from_output_minimal() {
        let output =
            aws_sdk_networkfirewall::operation::describe_firewall::DescribeFirewallOutput::builder()
                .build();
        let fw = Firewall::from(output);
        assert_eq!(fw.firewall_name, "");
        assert!(fw.firewall_arn.is_none());
        assert!(fw.firewall_policy_arn.is_none());
        assert!(fw.vpc_id.is_none());
        assert!(fw.subnet_mappings.is_empty());
        assert!(!fw.delete_protection);
        assert!(!fw.policy_change_protection);
        assert!(fw.description.is_none());
        assert!(fw.firewall_status.is_none());
        assert!(fw.tags.is_empty());
    }

    #[test]
    fn test_firewall_from_output_full() {
        let tag = aws_sdk_networkfirewall::types::Tag::builder()
            .key("env")
            .value("prod")
            .build()
            .expect("tag build");
        let subnet = aws_sdk_networkfirewall::types::SubnetMapping::builder()
            .subnet_id("subnet-123")
            .build()
            .expect("subnet build");
        let sdk_fw = aws_sdk_networkfirewall::types::Firewall::builder()
            .firewall_name("test-fw")
            .firewall_id("fw-1")
            .firewall_policy_arn("arn:aws:network-firewall::123:firewall-policy/pol")
            .vpc_id("vpc-123")
            .subnet_mappings(subnet)
            .delete_protection(true)
            .firewall_policy_change_protection(false)
            .description("Test firewall")
            .tags(tag)
            .build()
            .expect("fw build");
        let output =
            aws_sdk_networkfirewall::operation::describe_firewall::DescribeFirewallOutput::builder()
                .firewall(sdk_fw)
                .build();
        let fw = Firewall::from(output);
        assert_eq!(fw.firewall_name, "test-fw");
        assert_eq!(
            fw.firewall_policy_arn,
            Some("arn:aws:network-firewall::123:firewall-policy/pol".to_string())
        );
        assert_eq!(fw.vpc_id, Some("vpc-123".to_string()));
        assert_eq!(fw.subnet_mappings, vec!["subnet-123".to_string()]);
        assert!(fw.delete_protection);
        assert!(!fw.policy_change_protection);
        assert_eq!(fw.description, Some("Test firewall".to_string()));
        assert!(fw.firewall_status.is_none());
        assert_eq!(fw.tags.len(), 1);
        assert_eq!(fw.tags[0].key, "env");
        assert_eq!(fw.tags[0].value, "prod");
    }

    #[test]
    fn test_firewall_policy_from_output_minimal() {
        let resp = aws_sdk_networkfirewall::types::FirewallPolicyResponse::builder()
            .firewall_policy_name("pol")
            .firewall_policy_id("pol-1")
            .firewall_policy_arn("arn:aws:network-firewall::123:firewall-policy/pol")
            .build()
            .expect("resp build");
        let output = aws_sdk_networkfirewall::operation::describe_firewall_policy::DescribeFirewallPolicyOutput::builder()
            .update_token("token")
            .firewall_policy_response(resp)
            .build()
            .expect("output build");
        let policy = FirewallPolicy::from(output);
        assert_eq!(policy.name, "pol");
        assert_eq!(
            policy.arn,
            Some("arn:aws:network-firewall::123:firewall-policy/pol".to_string())
        );
        assert!(policy.description.is_none());
        assert!(policy.firewall_policy_status.is_none());
        assert!(policy.stateless_default_actions.is_empty());
        assert!(policy.stateless_rule_group_arns.is_empty());
        assert!(policy.stateful_rule_group_arns.is_empty());
        assert!(policy.tags.is_empty());
    }

    #[test]
    fn test_firewall_policy_from_output_full() {
        let tag = aws_sdk_networkfirewall::types::Tag::builder()
            .key("team")
            .value("security")
            .build()
            .expect("tag build");
        let stateless_ref = aws_sdk_networkfirewall::types::StatelessRuleGroupReference::builder()
            .resource_arn("arn:aws:network-firewall::123:stateless-rulegroup/sl")
            .priority(1)
            .build()
            .expect("stateless ref build");
        let stateful_ref = aws_sdk_networkfirewall::types::StatefulRuleGroupReference::builder()
            .resource_arn("arn:aws:network-firewall::123:stateful-rulegroup/sf")
            .build()
            .expect("stateful ref build");
        let sdk_policy = aws_sdk_networkfirewall::types::FirewallPolicy::builder()
            .stateless_default_actions("aws:drop")
            .stateless_fragment_default_actions("aws:drop")
            .stateless_rule_group_references(stateless_ref)
            .stateful_rule_group_references(stateful_ref)
            .stateful_default_actions("aws:drop_strict")
            .build()
            .expect("policy build");
        let resp = aws_sdk_networkfirewall::types::FirewallPolicyResponse::builder()
            .firewall_policy_name("full-pol")
            .firewall_policy_id("pol-2")
            .firewall_policy_arn("arn:aws:network-firewall::123:firewall-policy/full-pol")
            .description("Full policy")
            .tags(tag)
            .build()
            .expect("resp build");
        let output = aws_sdk_networkfirewall::operation::describe_firewall_policy::DescribeFirewallPolicyOutput::builder()
            .update_token("token")
            .firewall_policy_response(resp)
            .firewall_policy(sdk_policy)
            .build()
            .expect("output build");
        let policy = FirewallPolicy::from(output);
        assert_eq!(policy.name, "full-pol");
        assert_eq!(
            policy.description,
            Some("Full policy".to_string())
        );
        assert_eq!(policy.stateless_default_actions, vec!["aws:drop".to_string()]);
        assert_eq!(
            policy.stateless_rule_group_arns,
            vec!["arn:aws:network-firewall::123:stateless-rulegroup/sl".to_string()]
        );
        assert_eq!(
            policy.stateful_rule_group_arns,
            vec!["arn:aws:network-firewall::123:stateful-rulegroup/sf".to_string()]
        );
        assert_eq!(policy.stateful_default_actions, vec!["aws:drop_strict".to_string()]);
        assert_eq!(policy.tags.len(), 1);
        assert_eq!(policy.tags[0].key, "team");
    }

    #[test]
    fn test_rule_group_from_output_minimal() {
        let resp = aws_sdk_networkfirewall::types::RuleGroupResponse::builder()
            .rule_group_name("rg")
            .rule_group_id("rg-1")
            .rule_group_arn("arn:aws:network-firewall::123:stateless-rulegroup/rg")
            .build()
            .expect("resp build");
        let output = aws_sdk_networkfirewall::operation::describe_rule_group::DescribeRuleGroupOutput::builder()
            .update_token("token")
            .rule_group_response(resp)
            .build()
            .expect("output build");
        let rg = RuleGroup::from(output);
        assert_eq!(rg.name, "rg");
        assert_eq!(
            rg.arn,
            Some("arn:aws:network-firewall::123:stateless-rulegroup/rg".to_string())
        );
        assert!(rg.rule_group_type.is_none());
        assert!(rg.description.is_none());
        assert!(rg.capacity.is_none());
        assert!(rg.rule_group_status.is_none());
        assert!(rg.consumed_capacity.is_none());
        assert!(rg.number_of_associations.is_none());
        assert!(rg.tags.is_empty());
    }

    #[test]
    fn test_rule_group_from_output_full() {
        let tag = aws_sdk_networkfirewall::types::Tag::builder()
            .key("type")
            .value("stateless")
            .build()
            .expect("tag build");
        let resp = aws_sdk_networkfirewall::types::RuleGroupResponse::builder()
            .rule_group_name("my-rg")
            .rule_group_id("rg-2")
            .rule_group_arn("arn:aws:network-firewall::123:stateless-rulegroup/my-rg")
            .r#type(aws_sdk_networkfirewall::types::RuleGroupType::Stateless)
            .description("My rule group")
            .capacity(100)
            .consumed_capacity(10)
            .number_of_associations(2)
            .tags(tag)
            .build()
            .expect("resp build");
        let output = aws_sdk_networkfirewall::operation::describe_rule_group::DescribeRuleGroupOutput::builder()
            .update_token("token")
            .rule_group_response(resp)
            .build()
            .expect("output build");
        let rg = RuleGroup::from(output);
        assert_eq!(rg.name, "my-rg");
        assert_eq!(rg.rule_group_type, Some("STATELESS".to_string()));
        assert_eq!(rg.description, Some("My rule group".to_string()));
        assert_eq!(rg.capacity, Some(100));
        assert_eq!(rg.consumed_capacity, Some(10));
        assert_eq!(rg.number_of_associations, Some(2));
        assert_eq!(rg.tags.len(), 1);
        assert_eq!(rg.tags[0].key, "type");
        assert_eq!(rg.tags[0].value, "stateless");
    }
}
