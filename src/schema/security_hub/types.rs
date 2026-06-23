use async_graphql::SimpleObject;
use aws_sdk_securityhub::types::AwsSecurityFinding;

#[derive(SimpleObject, Clone)]
pub struct SecurityHubFinding {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub severity_label: Option<String>,
    pub workflow_status: Option<String>,
    pub record_state: Option<String>,
    pub product_name: Option<String>,
    pub company_name: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub region: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub compliance_status: Option<String>,
}

impl From<AwsSecurityFinding> for SecurityHubFinding {
    fn from(f: AwsSecurityFinding) -> Self {
        let resource = f.resources().first();
        Self {
            id: f.id().unwrap_or_default().to_string(),
            title: f.title().map(|s| s.to_string()),
            description: f.description().map(|s| s.to_string()),
            severity_label: f
                .severity()
                .and_then(|s| s.label())
                .map(|l| l.as_str().to_string()),
            workflow_status: f
                .workflow()
                .and_then(|w| w.status())
                .map(|s| s.as_str().to_string()),
            record_state: f.record_state().map(|s| s.as_str().to_string()),
            product_name: f.product_name().map(|s| s.to_string()),
            company_name: f.company_name().map(|s| s.to_string()),
            resource_type: resource.and_then(|r| r.r#type()).map(|s| s.to_string()),
            resource_id: resource.and_then(|r| r.id()).map(|s| s.to_string()),
            region: f.region().map(|s| s.to_string()),
            created_at: f.created_at().map(|s| s.to_string()),
            updated_at: f.updated_at().map(|s| s.to_string()),
            compliance_status: f
                .compliance()
                .and_then(|c| c.status())
                .map(|s| s.as_str().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_hub_finding_full() {
        let finding = SecurityHubFinding {
            id: "arn:aws:securityhub:us-east-1:123456789012:subscription/aws-foundational-security-best-practices/v/1.0.0/IAM.1/finding/abc123".to_string(),
            title: Some("IAM root user access key should not exist".to_string()),
            description: Some("This control checks whether the root user access key is available.".to_string()),
            severity_label: Some("CRITICAL".to_string()),
            workflow_status: Some("NEW".to_string()),
            record_state: Some("ACTIVE".to_string()),
            product_name: Some("Security Hub".to_string()),
            company_name: Some("AWS".to_string()),
            resource_type: Some("AwsAccount".to_string()),
            resource_id: Some("AWS::::Account:123456789012".to_string()),
            region: Some("us-east-1".to_string()),
            created_at: Some("2024-01-15T10:00:00.000Z".to_string()),
            updated_at: Some("2024-01-20T12:00:00.000Z".to_string()),
            compliance_status: Some("FAILED".to_string()),
        };

        assert!(finding.id.contains("abc123"));
        assert_eq!(finding.severity_label, Some("CRITICAL".to_string()));
        assert_eq!(finding.workflow_status, Some("NEW".to_string()));
        assert_eq!(finding.record_state, Some("ACTIVE".to_string()));
        assert_eq!(finding.compliance_status, Some("FAILED".to_string()));
        assert_eq!(finding.region, Some("us-east-1".to_string()));
    }

    #[test]
    fn test_security_hub_finding_minimal() {
        let finding = SecurityHubFinding {
            id: "finding-id-minimal".to_string(),
            title: None,
            description: None,
            severity_label: None,
            workflow_status: None,
            record_state: None,
            product_name: None,
            company_name: None,
            resource_type: None,
            resource_id: None,
            region: None,
            created_at: None,
            updated_at: None,
            compliance_status: None,
        };

        assert_eq!(finding.id, "finding-id-minimal");
        assert!(finding.title.is_none());
        assert!(finding.severity_label.is_none());
        assert!(finding.compliance_status.is_none());
    }

    #[test]
    fn test_security_hub_finding_guardduty_source() {
        let finding = SecurityHubFinding {
            id: "guardduty-finding-001".to_string(),
            title: Some("Unusual API calls from EC2 instance".to_string()),
            description: Some("An EC2 instance is making unusual API calls.".to_string()),
            severity_label: Some("HIGH".to_string()),
            workflow_status: Some("NOTIFIED".to_string()),
            record_state: Some("ACTIVE".to_string()),
            product_name: Some("GuardDuty".to_string()),
            company_name: Some("AWS".to_string()),
            resource_type: Some("AwsEc2Instance".to_string()),
            resource_id: Some("i-0abc1234def567890".to_string()),
            region: Some("us-west-2".to_string()),
            created_at: Some("2024-02-01T08:00:00.000Z".to_string()),
            updated_at: Some("2024-02-01T08:00:00.000Z".to_string()),
            compliance_status: None,
        };

        assert_eq!(finding.severity_label, Some("HIGH".to_string()));
        assert_eq!(finding.product_name, Some("GuardDuty".to_string()));
        assert_eq!(finding.workflow_status, Some("NOTIFIED".to_string()));
        assert!(finding.compliance_status.is_none());
    }

    #[test]
    fn test_security_hub_finding_resolved() {
        let finding = SecurityHubFinding {
            id: "resolved-finding-001".to_string(),
            title: Some("S3 bucket should have block public access settings enabled".to_string()),
            description: Some("This control checks if Amazon S3 buckets have block public access settings.".to_string()),
            severity_label: Some("MEDIUM".to_string()),
            workflow_status: Some("RESOLVED".to_string()),
            record_state: Some("ARCHIVED".to_string()),
            product_name: Some("Security Hub".to_string()),
            company_name: Some("AWS".to_string()),
            resource_type: Some("AwsS3Bucket".to_string()),
            resource_id: Some("my-secure-bucket".to_string()),
            region: Some("us-east-1".to_string()),
            created_at: Some("2024-01-01T00:00:00.000Z".to_string()),
            updated_at: Some("2024-01-30T00:00:00.000Z".to_string()),
            compliance_status: Some("PASSED".to_string()),
        };

        assert_eq!(finding.workflow_status, Some("RESOLVED".to_string()));
        assert_eq!(finding.record_state, Some("ARCHIVED".to_string()));
        assert_eq!(finding.compliance_status, Some("PASSED".to_string()));
    }
}
