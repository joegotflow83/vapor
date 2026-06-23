use async_graphql::SimpleObject;

use crate::schema::ec2::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct Detector {
    pub id: String,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub finding_publishing_frequency: Option<String>,
    pub tags: Vec<Tag>,
}

impl Detector {
    pub fn from_output(
        id: String,
        output: aws_sdk_guardduty::operation::get_detector::GetDetectorOutput,
    ) -> Self {
        let tags = output
            .tags()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| Tag {
                        key: k.to_string(),
                        value: v.to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            id,
            status: output.status().map(|s| s.as_str().to_string()),
            created_at: output.created_at().map(|s| s.to_string()),
            updated_at: output.updated_at().map(|s| s.to_string()),
            finding_publishing_frequency: output
                .finding_publishing_frequency()
                .map(|f| f.as_str().to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Finding {
    pub id: String,
    pub account_id: Option<String>,
    pub region: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub severity: f64,
    pub finding_type: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub resource_type: Option<String>,
    pub archived: bool,
}

impl From<aws_sdk_guardduty::types::Finding> for Finding {
    fn from(f: aws_sdk_guardduty::types::Finding) -> Self {
        let resource_type = f.resource().and_then(|r| r.resource_type().map(|s| s.to_string()));
        let archived = f
            .service()
            .and_then(|s| s.archived)
            .unwrap_or(false);

        Self {
            id: f.id().unwrap_or_default().to_string(),
            account_id: f.account_id().map(|s| s.to_string()),
            region: f.region().map(|s| s.to_string()),
            title: f.title().map(|s| s.to_string()),
            description: f.description().map(|s| s.to_string()),
            severity: f.severity().unwrap_or(0.0),
            finding_type: f.r#type().map(|s| s.to_string()),
            created_at: f.created_at().map(|s| s.to_string()),
            updated_at: f.updated_at().map(|s| s.to_string()),
            resource_type,
            archived,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_from_output_minimal() {
        let output = aws_sdk_guardduty::operation::get_detector::GetDetectorOutput::builder()
            .status(aws_sdk_guardduty::types::DetectorStatus::Enabled)
            .build();
        let detector = Detector::from_output("abc123".to_string(), output);
        assert_eq!(detector.id, "abc123");
        assert_eq!(detector.status, Some("ENABLED".to_string()));
        assert!(detector.created_at.is_none());
        assert!(detector.updated_at.is_none());
        assert!(detector.finding_publishing_frequency.is_none());
        assert!(detector.tags.is_empty());
    }

    #[test]
    fn test_detector_from_output_with_tags() {
        let output = aws_sdk_guardduty::operation::get_detector::GetDetectorOutput::builder()
            .status(aws_sdk_guardduty::types::DetectorStatus::Enabled)
            .created_at("2024-01-01T00:00:00Z")
            .updated_at("2024-06-01T00:00:00Z")
            .finding_publishing_frequency(
                aws_sdk_guardduty::types::FindingPublishingFrequency::SixHours,
            )
            .tags("Environment", "production")
            .build();
        let detector = Detector::from_output("det-1".to_string(), output);
        assert_eq!(detector.id, "det-1");
        assert_eq!(detector.created_at, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(detector.updated_at, Some("2024-06-01T00:00:00Z".to_string()));
        assert_eq!(
            detector.finding_publishing_frequency,
            Some("SIX_HOURS".to_string())
        );
        assert_eq!(detector.tags.len(), 1);
        assert_eq!(detector.tags[0].key, "Environment");
        assert_eq!(detector.tags[0].value, "production");
    }

    #[test]
    fn test_finding_from_sdk() {
        let finding = aws_sdk_guardduty::types::Finding::builder()
            .id("finding-1")
            .account_id("123456789012")
            .region("us-east-1")
            .title("Unusual API call")
            .description("An unusual API call was detected")
            .severity(7.5)
            .r#type("UnauthorizedAccess:EC2/SSHBruteForce")
            .created_at("2024-01-15T10:00:00Z")
            .updated_at("2024-01-15T12:00:00Z")
            .build();

        let result = Finding::from(finding);
        assert_eq!(result.id, "finding-1");
        assert_eq!(result.account_id, Some("123456789012".to_string()));
        assert_eq!(result.region, Some("us-east-1".to_string()));
        assert_eq!(result.title, Some("Unusual API call".to_string()));
        assert_eq!(result.severity, 7.5);
        assert_eq!(
            result.finding_type,
            Some("UnauthorizedAccess:EC2/SSHBruteForce".to_string())
        );
        assert!(!result.archived);
        assert!(result.resource_type.is_none());
    }

    #[test]
    fn test_finding_archived() {
        let service = aws_sdk_guardduty::types::Service::builder()
            .archived(true)
            .build();
        let finding = aws_sdk_guardduty::types::Finding::builder()
            .id("finding-2")
            .account_id("123456789012")
            .region("us-east-1")
            .title("Archived finding")
            .description("desc")
            .severity(2.0)
            .r#type("Recon:EC2/PortProbeUnprotectedPort")
            .created_at("2024-01-01T00:00:00Z")
            .updated_at("2024-01-02T00:00:00Z")
            .service(service)
            .build();

        let result = Finding::from(finding);
        assert!(result.archived);
    }
}
