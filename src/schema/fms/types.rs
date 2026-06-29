use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct FmsPolicy {
    pub policy_id: Option<String>,
    pub policy_arn: Option<String>,
    pub policy_name: Option<String>,
    pub security_service_type: Option<String>,
    pub remediation_enabled: Option<bool>,
    pub resource_type: Option<String>,
    pub delete_unused_fm_managed_resources: Option<bool>,
}

impl From<aws_sdk_fms::types::PolicySummary> for FmsPolicy {
    fn from(p: aws_sdk_fms::types::PolicySummary) -> Self {
        Self {
            policy_id: p.policy_id().map(|v| v.to_string()),
            policy_arn: p.policy_arn().map(|v| v.to_string()),
            policy_name: p.policy_name().map(|v| v.to_string()),
            security_service_type: p.security_service_type().map(|v| v.as_str().to_string()),
            remediation_enabled: Some(p.remediation_enabled()),
            resource_type: p.resource_type().map(|v| v.to_string()),
            delete_unused_fm_managed_resources: Some(p.delete_unused_fm_managed_resources()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct FmsEvaluationResult {
    pub compliance_status: Option<String>,
    pub violator_count: Option<i64>,
    pub evaluation_limit_exceeded: Option<bool>,
}

impl From<&aws_sdk_fms::types::EvaluationResult> for FmsEvaluationResult {
    fn from(e: &aws_sdk_fms::types::EvaluationResult) -> Self {
        Self {
            compliance_status: e.compliance_status().map(|v| v.as_str().to_string()),
            violator_count: Some(e.violator_count()),
            evaluation_limit_exceeded: Some(e.evaluation_limit_exceeded()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct FmsPolicyComplianceStatus {
    pub policy_owner: Option<String>,
    pub policy_id: Option<String>,
    pub policy_name: Option<String>,
    pub member_account: Option<String>,
    pub evaluation_results: Vec<FmsEvaluationResult>,
}

impl From<aws_sdk_fms::types::PolicyComplianceStatus> for FmsPolicyComplianceStatus {
    fn from(s: aws_sdk_fms::types::PolicyComplianceStatus) -> Self {
        Self {
            policy_owner: s.policy_owner().map(|v| v.to_string()),
            policy_id: s.policy_id().map(|v| v.to_string()),
            policy_name: s.policy_name().map(|v| v.to_string()),
            member_account: s.member_account().map(|v| v.to_string()),
            evaluation_results: s
                .evaluation_results()
                .iter()
                .map(FmsEvaluationResult::from)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fms_policy_from_minimal() {
        let p = aws_sdk_fms::types::PolicySummary::builder().build();
        let result = FmsPolicy::from(p);
        assert!(result.policy_id.is_none());
        assert!(result.policy_arn.is_none());
        assert!(result.policy_name.is_none());
        assert!(result.security_service_type.is_none());
        assert_eq!(result.remediation_enabled, Some(false));
        assert!(result.resource_type.is_none());
        assert_eq!(result.delete_unused_fm_managed_resources, Some(false));
    }

    #[test]
    fn test_fms_policy_from_full() {
        let p = aws_sdk_fms::types::PolicySummary::builder()
            .policy_id("policy-123")
            .policy_arn("arn:aws:fms:us-east-1:123456789012:policy/policy-123")
            .policy_name("MyPolicy")
            .security_service_type(aws_sdk_fms::types::SecurityServiceType::Wafv2)
            .remediation_enabled(true)
            .resource_type("AWS::EC2::Instance")
            .delete_unused_fm_managed_resources(true)
            .build();
        let result = FmsPolicy::from(p);
        assert_eq!(result.policy_id, Some("policy-123".to_string()));
        assert_eq!(
            result.policy_arn,
            Some("arn:aws:fms:us-east-1:123456789012:policy/policy-123".to_string())
        );
        assert_eq!(result.policy_name, Some("MyPolicy".to_string()));
        assert_eq!(result.security_service_type, Some("WAFV2".to_string()));
        assert_eq!(result.remediation_enabled, Some(true));
        assert_eq!(
            result.resource_type,
            Some("AWS::EC2::Instance".to_string())
        );
        assert_eq!(result.delete_unused_fm_managed_resources, Some(true));
    }

    #[test]
    fn test_fms_evaluation_result_from_minimal() {
        let e = aws_sdk_fms::types::EvaluationResult::builder().build();
        let result = FmsEvaluationResult::from(&e);
        assert!(result.compliance_status.is_none());
        assert_eq!(result.violator_count, Some(0));
        assert_eq!(result.evaluation_limit_exceeded, Some(false));
    }

    #[test]
    fn test_fms_evaluation_result_compliant() {
        let e = aws_sdk_fms::types::EvaluationResult::builder()
            .compliance_status(aws_sdk_fms::types::PolicyComplianceStatusType::Compliant)
            .violator_count(0)
            .evaluation_limit_exceeded(false)
            .build();
        let result = FmsEvaluationResult::from(&e);
        assert_eq!(result.compliance_status, Some("COMPLIANT".to_string()));
        assert_eq!(result.violator_count, Some(0));
        assert_eq!(result.evaluation_limit_exceeded, Some(false));
    }

    #[test]
    fn test_fms_evaluation_result_non_compliant() {
        let e = aws_sdk_fms::types::EvaluationResult::builder()
            .compliance_status(aws_sdk_fms::types::PolicyComplianceStatusType::NonCompliant)
            .violator_count(5)
            .evaluation_limit_exceeded(false)
            .build();
        let result = FmsEvaluationResult::from(&e);
        assert_eq!(result.compliance_status, Some("NON_COMPLIANT".to_string()));
        assert_eq!(result.violator_count, Some(5));
    }

    #[test]
    fn test_fms_policy_compliance_status_from_minimal() {
        let s = aws_sdk_fms::types::PolicyComplianceStatus::builder().build();
        let result = FmsPolicyComplianceStatus::from(s);
        assert!(result.policy_owner.is_none());
        assert!(result.policy_id.is_none());
        assert!(result.policy_name.is_none());
        assert!(result.member_account.is_none());
        assert!(result.evaluation_results.is_empty());
    }

    #[test]
    fn test_fms_policy_compliance_status_from_full() {
        let eval = aws_sdk_fms::types::EvaluationResult::builder()
            .compliance_status(aws_sdk_fms::types::PolicyComplianceStatusType::NonCompliant)
            .violator_count(2)
            .build();
        let s = aws_sdk_fms::types::PolicyComplianceStatus::builder()
            .policy_owner("123456789012")
            .policy_id("policy-123")
            .policy_name("TestPolicy")
            .member_account("987654321098")
            .evaluation_results(eval)
            .build();
        let result = FmsPolicyComplianceStatus::from(s);
        assert_eq!(result.policy_owner, Some("123456789012".to_string()));
        assert_eq!(result.policy_id, Some("policy-123".to_string()));
        assert_eq!(result.policy_name, Some("TestPolicy".to_string()));
        assert_eq!(result.member_account, Some("987654321098".to_string()));
        assert_eq!(result.evaluation_results.len(), 1);
        assert_eq!(
            result.evaluation_results[0].compliance_status,
            Some("NON_COMPLIANT".to_string())
        );
        assert_eq!(result.evaluation_results[0].violator_count, Some(2));
    }

    #[test]
    fn test_fms_evaluation_result_limit_exceeded() {
        let eval = aws_sdk_fms::types::EvaluationResult::builder()
            .evaluation_limit_exceeded(true)
            .build();
        let s = aws_sdk_fms::types::PolicyComplianceStatus::builder()
            .evaluation_results(eval)
            .build();
        let result = FmsPolicyComplianceStatus::from(s);
        assert_eq!(result.evaluation_results.len(), 1);
        assert_eq!(
            result.evaluation_results[0].evaluation_limit_exceeded,
            Some(true)
        );
    }
}
