use async_graphql::SimpleObject;
use aws_sdk_config as config_sdk;

#[derive(SimpleObject, Clone)]
pub struct ConfigRule {
    pub name: String,
    pub arn: Option<String>,
    pub rule_id: Option<String>,
    pub description: Option<String>,
    pub state: Option<String>,
    pub source_identifier: Option<String>,
    pub source_owner: Option<String>,
}

impl From<config_sdk::types::ConfigRule> for ConfigRule {
    fn from(r: config_sdk::types::ConfigRule) -> Self {
        let (source_identifier, source_owner) = r
            .source()
            .map(|s| {
                (
                    s.source_identifier().map(|v| v.to_string()),
                    Some(s.owner().as_str().to_string()),
                )
            })
            .unwrap_or((None, None));

        Self {
            name: r.config_rule_name().unwrap_or_default().to_string(),
            arn: r.config_rule_arn().map(|s| s.to_string()),
            rule_id: r.config_rule_id().map(|s| s.to_string()),
            description: r.description().map(|s| s.to_string()),
            state: r.config_rule_state().map(|s| s.as_str().to_string()),
            source_identifier,
            source_owner,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ComplianceSummary {
    pub rule_name: String,
    pub compliance_type: String,
    pub compliant_count: Option<i32>,
    pub non_compliant_count: Option<i32>,
}

impl From<config_sdk::types::ComplianceByConfigRule> for ComplianceSummary {
    fn from(c: config_sdk::types::ComplianceByConfigRule) -> Self {
        let (compliance_type, compliant_count, non_compliant_count) = c
            .compliance()
            .map(|comp| {
                let ct = comp
                    .compliance_type()
                    .map(|t| t.as_str().to_string())
                    .unwrap_or_default();
                let count = comp
                    .compliance_contributor_count()
                    .map(|cc| cc.capped_count());
                let (compliant, non_compliant) = match ct.as_str() {
                    "COMPLIANT" => (count, None),
                    "NON_COMPLIANT" => (None, count),
                    _ => (None, None),
                };
                (ct, compliant, non_compliant)
            })
            .unwrap_or_default();

        Self {
            rule_name: c.config_rule_name().unwrap_or_default().to_string(),
            compliance_type,
            compliant_count,
            non_compliant_count,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ComplianceByResource {
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub compliance_type: Option<String>,
}

impl From<config_sdk::types::ComplianceByResource> for ComplianceByResource {
    fn from(c: config_sdk::types::ComplianceByResource) -> Self {
        let compliance_type = c
            .compliance()
            .and_then(|comp| comp.compliance_type())
            .map(|t| t.as_str().to_string());

        Self {
            resource_type: c.resource_type().map(|s| s.to_string()),
            resource_id: c.resource_id().map(|s| s.to_string()),
            compliance_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_rule_from_sdk() {
        let source = config_sdk::types::Source::builder()
            .owner(config_sdk::types::Owner::Aws)
            .source_identifier("S3_BUCKET_VERSIONING_ENABLED")
            .build()
            .unwrap();
        let rule = config_sdk::types::ConfigRule::builder()
            .config_rule_name("s3-versioning")
            .config_rule_arn("arn:aws:config:us-east-1:123456789012:config-rule/config-rule-abc")
            .config_rule_id("config-rule-abc")
            .description("Checks S3 bucket versioning")
            .config_rule_state(config_sdk::types::ConfigRuleState::Active)
            .source(source)
            .build();

        let result = ConfigRule::from(rule);
        assert_eq!(result.name, "s3-versioning");
        assert_eq!(
            result.arn,
            Some("arn:aws:config:us-east-1:123456789012:config-rule/config-rule-abc".to_string())
        );
        assert_eq!(result.rule_id, Some("config-rule-abc".to_string()));
        assert_eq!(result.description, Some("Checks S3 bucket versioning".to_string()));
        assert_eq!(result.state, Some("ACTIVE".to_string()));
        assert_eq!(
            result.source_identifier,
            Some("S3_BUCKET_VERSIONING_ENABLED".to_string())
        );
        assert_eq!(result.source_owner, Some("AWS".to_string()));
    }

    #[test]
    fn test_config_rule_minimal() {
        let rule = config_sdk::types::ConfigRule::builder()
            .config_rule_name("minimal-rule")
            .build();

        let result = ConfigRule::from(rule);
        assert_eq!(result.name, "minimal-rule");
        assert!(result.arn.is_none());
        assert!(result.rule_id.is_none());
        assert!(result.description.is_none());
        assert!(result.state.is_none());
        assert!(result.source_identifier.is_none());
        assert!(result.source_owner.is_none());
    }

    #[test]
    fn test_compliance_summary_from_sdk() {
        let compliance = config_sdk::types::Compliance::builder()
            .compliance_type(config_sdk::types::ComplianceType::NonCompliant)
            .build();
        let cbr = config_sdk::types::ComplianceByConfigRule::builder()
            .config_rule_name("my-rule")
            .compliance(compliance)
            .build();

        let result = ComplianceSummary::from(cbr);
        assert_eq!(result.rule_name, "my-rule");
        assert_eq!(result.compliance_type, "NON_COMPLIANT");
    }

    #[test]
    fn test_compliance_by_resource_from_sdk() {
        let compliance = config_sdk::types::Compliance::builder()
            .compliance_type(config_sdk::types::ComplianceType::Compliant)
            .build();
        let cbr = config_sdk::types::ComplianceByResource::builder()
            .resource_type("AWS::S3::Bucket")
            .resource_id("my-bucket")
            .compliance(compliance)
            .build();

        let result = ComplianceByResource::from(cbr);
        assert_eq!(result.resource_type, Some("AWS::S3::Bucket".to_string()));
        assert_eq!(result.resource_id, Some("my-bucket".to_string()));
        assert_eq!(result.compliance_type, Some("COMPLIANT".to_string()));
    }

    #[test]
    fn test_compliance_by_resource_minimal() {
        let cbr = config_sdk::types::ComplianceByResource::builder().build();

        let result = ComplianceByResource::from(cbr);
        assert!(result.resource_type.is_none());
        assert!(result.resource_id.is_none());
        assert!(result.compliance_type.is_none());
    }
}
