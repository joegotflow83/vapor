use async_graphql::{Enum, SimpleObject};

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum WafScope {
    Regional,
    Cloudfront,
}

impl WafScope {
    pub fn to_sdk(&self) -> aws_sdk_wafv2::types::Scope {
        match self {
            WafScope::Regional => aws_sdk_wafv2::types::Scope::Regional,
            WafScope::Cloudfront => aws_sdk_wafv2::types::Scope::Cloudfront,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            WafScope::Regional => "REGIONAL",
            WafScope::Cloudfront => "CLOUDFRONT",
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct WebAcl {
    pub name: String,
    pub id: String,
    pub arn: Option<String>,
    pub scope: String,
    pub description: Option<String>,
    pub capacity: Option<i64>,
    pub default_action: Option<String>,
    pub rules_count: i32,
    pub managed_by_firewall_manager: bool,
}

impl WebAcl {
    pub fn from_sdk(
        acl: &aws_sdk_wafv2::types::WebAcl,
        scope: &WafScope,
    ) -> Self {
        let default_action = acl.default_action().map(|a| {
            if a.allow().is_some() {
                "ALLOW".to_string()
            } else {
                "BLOCK".to_string()
            }
        });

        Self {
            name: acl.name().to_string(),
            id: acl.id().to_string(),
            arn: Some(acl.arn().to_string()),
            scope: scope.as_str().to_string(),
            description: acl.description().map(|s| s.to_string()),
            capacity: Some(acl.capacity()),
            default_action,
            rules_count: acl.rules().len() as i32,
            managed_by_firewall_manager: acl.managed_by_firewall_manager(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct WafIpSet {
    pub name: String,
    pub id: String,
    pub arn: Option<String>,
    pub scope: String,
    pub description: Option<String>,
    pub ip_address_version: Option<String>,
    pub addresses: Vec<String>,
}

impl WafIpSet {
    pub fn from_sdk(
        ip_set: &aws_sdk_wafv2::types::IpSet,
        scope: &WafScope,
    ) -> Self {
        Self {
            name: ip_set.name().to_string(),
            id: ip_set.id().to_string(),
            arn: Some(ip_set.arn().to_string()),
            scope: scope.as_str().to_string(),
            description: ip_set.description().map(|s| s.to_string()),
            ip_address_version: Some(ip_set.ip_address_version().as_str().to_string()),
            addresses: ip_set.addresses().to_vec(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct WafRuleGroup {
    pub name: String,
    pub id: String,
    pub arn: Option<String>,
    pub scope: String,
    pub description: Option<String>,
    pub capacity: Option<i64>,
}

impl WafRuleGroup {
    pub fn from_summary(
        summary: &aws_sdk_wafv2::types::RuleGroupSummary,
        scope: &WafScope,
    ) -> Self {
        Self {
            name: summary.name().unwrap_or_default().to_string(),
            id: summary.id().unwrap_or_default().to_string(),
            arn: summary.arn().map(|s| s.to_string()),
            scope: scope.as_str().to_string(),
            description: summary.description().map(|s| s.to_string()),
            capacity: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waf_scope_to_sdk() {
        assert_eq!(WafScope::Regional.to_sdk(), aws_sdk_wafv2::types::Scope::Regional);
        assert_eq!(WafScope::Cloudfront.to_sdk(), aws_sdk_wafv2::types::Scope::Cloudfront);
    }

    #[test]
    fn test_waf_scope_as_str() {
        assert_eq!(WafScope::Regional.as_str(), "REGIONAL");
        assert_eq!(WafScope::Cloudfront.as_str(), "CLOUDFRONT");
    }

    #[test]
    fn test_web_acl_from_sdk() {
        let default_action = aws_sdk_wafv2::types::DefaultAction::builder()
            .allow(aws_sdk_wafv2::types::AllowAction::builder().build())
            .build();
        let rule = aws_sdk_wafv2::types::Rule::builder()
            .name("rule1")
            .priority(1)
            .statement(
                aws_sdk_wafv2::types::Statement::builder()
                    .build(),
            )
            .visibility_config(
                aws_sdk_wafv2::types::VisibilityConfig::builder()
                    .sampled_requests_enabled(true)
                    .cloud_watch_metrics_enabled(true)
                    .metric_name("rule1")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let acl = aws_sdk_wafv2::types::WebAcl::builder()
            .name("test-acl")
            .id("abc-123")
            .arn("arn:aws:wafv2:us-east-1:123456789012:regional/webacl/test-acl/abc-123")
            .default_action(default_action)
            .capacity(100)
            .rules(rule)
            .visibility_config(
                aws_sdk_wafv2::types::VisibilityConfig::builder()
                    .sampled_requests_enabled(true)
                    .cloud_watch_metrics_enabled(true)
                    .metric_name("test-acl")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let result = WebAcl::from_sdk(&acl, &WafScope::Regional);
        assert_eq!(result.name, "test-acl");
        assert_eq!(result.id, "abc-123");
        assert_eq!(result.scope, "REGIONAL");
        assert_eq!(result.capacity, Some(100));
        assert_eq!(result.default_action, Some("ALLOW".to_string()));
        assert_eq!(result.rules_count, 1);
    }

    #[test]
    fn test_waf_ip_set_from_sdk() {
        let ip_set = aws_sdk_wafv2::types::IpSet::builder()
            .name("my-ip-set")
            .id("ipset-123")
            .arn("arn:aws:wafv2:us-east-1:123456789012:regional/ipset/my-ip-set/ipset-123")
            .ip_address_version(aws_sdk_wafv2::types::IpAddressVersion::Ipv4)
            .addresses("10.0.0.0/8")
            .addresses("192.168.0.0/16")
            .build()
            .unwrap();
        let result = WafIpSet::from_sdk(&ip_set, &WafScope::Regional);
        assert_eq!(result.name, "my-ip-set");
        assert_eq!(result.id, "ipset-123");
        assert_eq!(result.scope, "REGIONAL");
        assert_eq!(result.ip_address_version, Some("IPV4".to_string()));
        assert_eq!(result.addresses, vec!["10.0.0.0/8", "192.168.0.0/16"]);
    }

    #[test]
    fn test_waf_rule_group_from_summary() {
        let summary = aws_sdk_wafv2::types::RuleGroupSummary::builder()
            .name("my-rule-group")
            .id("rg-123")
            .arn("arn:aws:wafv2:us-east-1:123456789012:regional/rulegroup/my-rule-group/rg-123")
            .description("Test rule group")
            .build();
        let result = WafRuleGroup::from_summary(&summary, &WafScope::Cloudfront);
        assert_eq!(result.name, "my-rule-group");
        assert_eq!(result.id, "rg-123");
        assert_eq!(result.scope, "CLOUDFRONT");
        assert_eq!(result.description, Some("Test rule group".to_string()));
        assert!(result.capacity.is_none());
    }
}
