use async_graphql::SimpleObject;
use aws_sdk_inspector2::types::{CoveredResource, Finding};

#[derive(SimpleObject, Clone)]
pub struct InspectorFinding {
    pub finding_arn: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub finding_type: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub first_observed_at: Option<String>,
    pub last_observed_at: Option<String>,
    pub fix_available: Option<String>,
}

impl From<Finding> for InspectorFinding {
    fn from(f: Finding) -> Self {
        let resource = f.resources().first();
        Self {
            finding_arn: f.finding_arn().to_string(),
            title: f.title().map(|s| s.to_string()),
            description: Some(f.description().to_string()),
            severity: Some(f.severity().as_str().to_string()),
            status: Some(f.status().as_str().to_string()),
            finding_type: Some(f.r#type().as_str().to_string()),
            resource_type: resource.map(|r| r.r#type().as_str().to_string()),
            resource_id: resource.map(|r| r.id().to_string()),
            first_observed_at: Some(f.first_observed_at().to_string()),
            last_observed_at: Some(f.last_observed_at().to_string()),
            fix_available: f.fix_available().map(|v| v.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct InspectorCoverage {
    pub resource_id: String,
    pub resource_type: Option<String>,
    pub scan_status: Option<String>,
    pub scan_status_reason: Option<String>,
}

impl From<CoveredResource> for InspectorCoverage {
    fn from(c: CoveredResource) -> Self {
        let status = c.scan_status();
        Self {
            resource_id: c.resource_id().to_string(),
            resource_type: Some(c.resource_type().as_str().to_string()),
            scan_status: status.map(|s| s.status_code().as_str().to_string()),
            scan_status_reason: status.map(|s| s.reason().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspector_finding_fields() {
        let finding = InspectorFinding {
            finding_arn: "arn:aws:inspector2:us-east-1:123456789012:finding/abc123".to_string(),
            title: Some("Critical vulnerability in package".to_string()),
            description: Some("A critical vulnerability was found.".to_string()),
            severity: Some("CRITICAL".to_string()),
            status: Some("ACTIVE".to_string()),
            finding_type: Some("PACKAGE_VULNERABILITY".to_string()),
            resource_type: Some("AWS_EC2_INSTANCE".to_string()),
            resource_id: Some("i-0123456789abcdef0".to_string()),
            first_observed_at: Some("2024-01-15T10:00:00Z".to_string()),
            last_observed_at: Some("2024-01-20T12:00:00Z".to_string()),
            fix_available: Some("YES".to_string()),
        };
        assert_eq!(
            finding.finding_arn,
            "arn:aws:inspector2:us-east-1:123456789012:finding/abc123"
        );
        assert_eq!(finding.severity, Some("CRITICAL".to_string()));
        assert_eq!(finding.status, Some("ACTIVE".to_string()));
        assert_eq!(finding.finding_type, Some("PACKAGE_VULNERABILITY".to_string()));
        assert_eq!(finding.resource_type, Some("AWS_EC2_INSTANCE".to_string()));
        assert_eq!(finding.fix_available, Some("YES".to_string()));
    }

    #[test]
    fn test_inspector_finding_minimal() {
        let finding = InspectorFinding {
            finding_arn: "arn:aws:inspector2:us-east-1:123456789012:finding/xyz".to_string(),
            title: None,
            description: None,
            severity: None,
            status: None,
            finding_type: None,
            resource_type: None,
            resource_id: None,
            first_observed_at: None,
            last_observed_at: None,
            fix_available: None,
        };
        assert_eq!(
            finding.finding_arn,
            "arn:aws:inspector2:us-east-1:123456789012:finding/xyz"
        );
        assert!(finding.title.is_none());
        assert!(finding.severity.is_none());
        assert!(finding.fix_available.is_none());
    }

    #[test]
    fn test_inspector_coverage_fields() {
        let coverage = InspectorCoverage {
            resource_id: "i-0123456789abcdef0".to_string(),
            resource_type: Some("AWS_EC2_INSTANCE".to_string()),
            scan_status: Some("ACTIVE".to_string()),
            scan_status_reason: None,
        };
        assert_eq!(coverage.resource_id, "i-0123456789abcdef0");
        assert_eq!(coverage.resource_type, Some("AWS_EC2_INSTANCE".to_string()));
        assert_eq!(coverage.scan_status, Some("ACTIVE".to_string()));
        assert!(coverage.scan_status_reason.is_none());
    }

    #[test]
    fn test_inspector_coverage_inactive() {
        let coverage = InspectorCoverage {
            resource_id: "arn:aws:lambda:us-east-1:123:function:my-fn".to_string(),
            resource_type: Some("AWS_LAMBDA_FUNCTION".to_string()),
            scan_status: Some("INACTIVE".to_string()),
            scan_status_reason: Some("UNSUPPORTED_RUNTIME".to_string()),
        };
        assert_eq!(coverage.scan_status, Some("INACTIVE".to_string()));
        assert_eq!(
            coverage.scan_status_reason,
            Some("UNSUPPORTED_RUNTIME".to_string())
        );
    }
}
