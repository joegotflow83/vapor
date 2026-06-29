use async_graphql::SimpleObject;

use crate::aws::xray::{XRayEncryptionConfigInfo, XRayGroupInfo, XRayGroupInsightsConfig, XRaySamplingRuleInfo};

#[derive(SimpleObject, Clone)]
pub struct XRayInsightsConfig {
    pub insights_enabled: Option<bool>,
    pub notifications_enabled: Option<bool>,
}

impl From<XRayGroupInsightsConfig> for XRayInsightsConfig {
    fn from(ic: XRayGroupInsightsConfig) -> Self {
        Self {
            insights_enabled: ic.insights_enabled,
            notifications_enabled: ic.notifications_enabled,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct XRayGroup {
    pub group_name: Option<String>,
    pub group_arn: Option<String>,
    pub filter_expression: Option<String>,
    pub insights_configuration: Option<XRayInsightsConfig>,
}

impl From<XRayGroupInfo> for XRayGroup {
    fn from(g: XRayGroupInfo) -> Self {
        Self {
            group_name: g.group_name,
            group_arn: g.group_arn,
            filter_expression: g.filter_expression,
            insights_configuration: g.insights_configuration.map(XRayInsightsConfig::from),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct XRaySamplingRule {
    pub rule_name: Option<String>,
    pub rule_arn: Option<String>,
    pub priority: Option<i32>,
    pub fixed_rate: Option<f64>,
    pub reservoir_size: Option<i32>,
    pub service_name: Option<String>,
    pub service_type: Option<String>,
    pub host: Option<String>,
    pub http_method: Option<String>,
    pub url_path: Option<String>,
    pub resource_arn: Option<String>,
    pub version: Option<i32>,
}

impl From<XRaySamplingRuleInfo> for XRaySamplingRule {
    fn from(r: XRaySamplingRuleInfo) -> Self {
        Self {
            rule_name: r.rule_name,
            rule_arn: r.rule_arn,
            priority: r.priority,
            fixed_rate: r.fixed_rate,
            reservoir_size: r.reservoir_size,
            service_name: r.service_name,
            service_type: r.service_type,
            host: r.host,
            http_method: r.http_method,
            url_path: r.url_path,
            resource_arn: r.resource_arn,
            version: r.version,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct XRayEncryptionConfig {
    pub key_id: Option<String>,
    pub status: Option<String>,
    pub type_: Option<String>,
}

impl From<XRayEncryptionConfigInfo> for XRayEncryptionConfig {
    fn from(ec: XRayEncryptionConfigInfo) -> Self {
        Self {
            key_id: ec.key_id,
            status: ec.status,
            type_: ec.type_,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::xray::{XRayEncryptionConfigInfo, XRayGroupInfo, XRayGroupInsightsConfig, XRaySamplingRuleInfo};

    #[test]
    fn test_insights_config_from() {
        let ic = XRayGroupInsightsConfig {
            insights_enabled: Some(true),
            notifications_enabled: Some(false),
        };
        let result = XRayInsightsConfig::from(ic);
        assert_eq!(result.insights_enabled, Some(true));
        assert_eq!(result.notifications_enabled, Some(false));
    }

    #[test]
    fn test_group_from_full() {
        let info = XRayGroupInfo {
            group_name: Some("my-group".to_string()),
            group_arn: Some("arn:aws:xray:us-east-1:123456789012:group/my-group/abc123".to_string()),
            filter_expression: Some("service(\"my-service\")".to_string()),
            insights_configuration: Some(XRayGroupInsightsConfig {
                insights_enabled: Some(true),
                notifications_enabled: Some(true),
            }),
        };
        let result = XRayGroup::from(info);
        assert_eq!(result.group_name, Some("my-group".to_string()));
        assert_eq!(result.filter_expression, Some("service(\"my-service\")".to_string()));
        assert!(result.insights_configuration.is_some());
        let ic = result.insights_configuration.unwrap();
        assert_eq!(ic.insights_enabled, Some(true));
    }

    #[test]
    fn test_group_from_minimal() {
        let info = XRayGroupInfo {
            group_name: None,
            group_arn: None,
            filter_expression: None,
            insights_configuration: None,
        };
        let result = XRayGroup::from(info);
        assert!(result.group_name.is_none());
        assert!(result.insights_configuration.is_none());
    }

    #[test]
    fn test_sampling_rule_from_full() {
        let info = XRaySamplingRuleInfo {
            rule_name: Some("my-rule".to_string()),
            rule_arn: Some("arn:aws:xray:us-east-1:123456789012:sampling-rule/my-rule".to_string()),
            priority: Some(1000),
            fixed_rate: Some(0.05),
            reservoir_size: Some(5),
            service_name: Some("my-service".to_string()),
            service_type: Some("AWS::ECS::Container".to_string()),
            host: Some("*".to_string()),
            http_method: Some("*".to_string()),
            url_path: Some("*".to_string()),
            resource_arn: Some("*".to_string()),
            version: Some(1),
        };
        let result = XRaySamplingRule::from(info);
        assert_eq!(result.rule_name, Some("my-rule".to_string()));
        assert_eq!(result.priority, Some(1000));
        assert_eq!(result.fixed_rate, Some(0.05));
        assert_eq!(result.reservoir_size, Some(5));
        assert_eq!(result.version, Some(1));
    }

    #[test]
    fn test_sampling_rule_from_minimal() {
        let info = XRaySamplingRuleInfo {
            rule_name: None,
            rule_arn: None,
            priority: None,
            fixed_rate: None,
            reservoir_size: None,
            service_name: None,
            service_type: None,
            host: None,
            http_method: None,
            url_path: None,
            resource_arn: None,
            version: None,
        };
        let result = XRaySamplingRule::from(info);
        assert!(result.rule_name.is_none());
        assert!(result.priority.is_none());
        assert!(result.fixed_rate.is_none());
    }

    #[test]
    fn test_encryption_config_from_kms() {
        let info = XRayEncryptionConfigInfo {
            key_id: Some("arn:aws:kms:us-east-1:123456789012:key/abc-def".to_string()),
            status: Some("ACTIVE".to_string()),
            type_: Some("KMS".to_string()),
        };
        let result = XRayEncryptionConfig::from(info);
        assert!(result.key_id.is_some());
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.type_, Some("KMS".to_string()));
    }

    #[test]
    fn test_encryption_config_from_none_type() {
        let info = XRayEncryptionConfigInfo {
            key_id: None,
            status: Some("ACTIVE".to_string()),
            type_: Some("NONE".to_string()),
        };
        let result = XRayEncryptionConfig::from(info);
        assert!(result.key_id.is_none());
        assert_eq!(result.type_, Some("NONE".to_string()));
    }
}
