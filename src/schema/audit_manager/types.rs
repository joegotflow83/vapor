use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct AuditManagerAssessment {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub creation_time: Option<String>,
    pub last_updated: Option<String>,
    pub compliance_type: Option<String>,
}

impl From<aws_sdk_auditmanager::types::AssessmentMetadataItem> for AuditManagerAssessment {
    fn from(a: aws_sdk_auditmanager::types::AssessmentMetadataItem) -> Self {
        Self {
            id: a.id().map(|v| v.to_string()),
            // AssessmentMetadataItem has no ARN field.
            arn: None,
            name: a.name().map(|v| v.to_string()),
            status: a.status().map(|v| v.as_str().to_string()),
            creation_time: a.creation_time().map(|t| t.to_string()),
            last_updated: a.last_updated().map(|t| t.to_string()),
            compliance_type: a.compliance_type().map(|v| v.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AuditManagerFramework {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub type_: Option<String>,
    pub compliance_type: Option<String>,
    pub controls_count: Option<i32>,
    pub created_at: Option<String>,
    pub last_updated_at: Option<String>,
}

impl From<aws_sdk_auditmanager::types::AssessmentFrameworkMetadata> for AuditManagerFramework {
    fn from(f: aws_sdk_auditmanager::types::AssessmentFrameworkMetadata) -> Self {
        Self {
            id: f.id().map(|v| v.to_string()),
            arn: f.arn().map(|v| v.to_string()),
            name: f.name().map(|v| v.to_string()),
            type_: f.r#type().map(|v| v.as_str().to_string()),
            compliance_type: f.compliance_type().map(|v| v.to_string()),
            controls_count: Some(f.controls_count()),
            created_at: f.created_at().map(|t| t.to_string()),
            last_updated_at: f.last_updated_at().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AuditManagerControl {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub type_: Option<String>,
    pub description: Option<String>,
    pub testing_information: Option<String>,
}

impl From<aws_sdk_auditmanager::types::ControlMetadata> for AuditManagerControl {
    fn from(c: aws_sdk_auditmanager::types::ControlMetadata) -> Self {
        Self {
            id: c.id().map(|v| v.to_string()),
            arn: c.arn().map(|v| v.to_string()),
            name: c.name().map(|v| v.to_string()),
            // ControlMetadata exposes no type/control_type field.
            type_: None,
            description: None,
            testing_information: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_manager_assessment_from_minimal() {
        let a = aws_sdk_auditmanager::types::AssessmentMetadataItem::builder().build();
        let result = AuditManagerAssessment::from(a);
        assert!(result.id.is_none());
        assert!(result.arn.is_none());
        assert!(result.name.is_none());
        assert!(result.status.is_none());
        assert!(result.creation_time.is_none());
        assert!(result.last_updated.is_none());
        assert!(result.compliance_type.is_none());
    }

    #[test]
    fn test_audit_manager_assessment_from_full() {
        let a = aws_sdk_auditmanager::types::AssessmentMetadataItem::builder()
            .id("assess-123")
            .name("MyAssessment")
            .status(aws_sdk_auditmanager::types::AssessmentStatus::Active)
            .compliance_type("PCI DSS")
            .build();
        let result = AuditManagerAssessment::from(a);
        assert_eq!(result.id, Some("assess-123".to_string()));
        // AssessmentMetadataItem has no ARN field, so this is always None.
        assert!(result.arn.is_none());
        assert_eq!(result.name, Some("MyAssessment".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.compliance_type, Some("PCI DSS".to_string()));
    }

    #[test]
    fn test_audit_manager_framework_from_minimal() {
        let f = aws_sdk_auditmanager::types::AssessmentFrameworkMetadata::builder().build();
        let result = AuditManagerFramework::from(f);
        assert!(result.id.is_none());
        assert!(result.arn.is_none());
        assert!(result.name.is_none());
        assert!(result.type_.is_none());
        assert!(result.compliance_type.is_none());
        assert_eq!(result.controls_count, Some(0));
        assert!(result.created_at.is_none());
        assert!(result.last_updated_at.is_none());
    }

    #[test]
    fn test_audit_manager_framework_from_full() {
        let f = aws_sdk_auditmanager::types::AssessmentFrameworkMetadata::builder()
            .id("fw-123")
            .arn("arn:aws:auditmanager:us-east-1:123456789012:assessmentFramework/fw-123")
            .name("PCI DSS v3.2.1")
            .r#type(aws_sdk_auditmanager::types::FrameworkType::Standard)
            .compliance_type("PCI DSS")
            .controls_count(133)
            .build();
        let result = AuditManagerFramework::from(f);
        assert_eq!(result.id, Some("fw-123".to_string()));
        assert_eq!(result.name, Some("PCI DSS v3.2.1".to_string()));
        assert_eq!(result.type_, Some("Standard".to_string()));
        assert_eq!(result.compliance_type, Some("PCI DSS".to_string()));
        assert_eq!(result.controls_count, Some(133));
    }

    #[test]
    fn test_audit_manager_control_from_minimal() {
        let c = aws_sdk_auditmanager::types::ControlMetadata::builder().build();
        let result = AuditManagerControl::from(c);
        assert!(result.id.is_none());
        assert!(result.arn.is_none());
        assert!(result.name.is_none());
        assert!(result.type_.is_none());
        assert!(result.description.is_none());
        assert!(result.testing_information.is_none());
    }

    #[test]
    fn test_audit_manager_control_from_full() {
        let c = aws_sdk_auditmanager::types::ControlMetadata::builder()
            .id("ctrl-123")
            .arn("arn:aws:auditmanager:us-east-1:123456789012:control/ctrl-123")
            .name("Firewall Protection")
            .build();
        let result = AuditManagerControl::from(c);
        assert_eq!(result.id, Some("ctrl-123".to_string()));
        assert_eq!(
            result.arn,
            Some("arn:aws:auditmanager:us-east-1:123456789012:control/ctrl-123".to_string())
        );
        assert_eq!(result.name, Some("Firewall Protection".to_string()));
        // ControlMetadata exposes no type/control_type field, so this is always None.
        assert!(result.type_.is_none());
        assert!(result.description.is_none());
        assert!(result.testing_information.is_none());
    }
}
