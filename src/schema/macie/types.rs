use async_graphql::SimpleObject;
use aws_sdk_macie2::primitives::DateTime;
use aws_sdk_macie2::types::{BucketMetadata, Finding};

fn format_datetime(dt: &DateTime) -> String {
    chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos())
        .map(|d: chrono::DateTime<chrono::Utc>| d.to_rfc3339())
        .unwrap_or_default()
}

#[derive(SimpleObject, Clone)]
pub struct MacieFinding {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
    pub finding_type: Option<String>,
    pub category: Option<String>,
    pub resource_type: Option<String>,
    pub bucket_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub archived: bool,
}

impl From<Finding> for MacieFinding {
    fn from(f: Finding) -> Self {
        let bucket_name = f
            .resources_affected()
            .and_then(|r| r.s3_bucket())
            .and_then(|b| b.name())
            .map(|s| s.to_string());

        let resource_type = if f
            .resources_affected()
            .and_then(|r| r.s3_object())
            .is_some()
        {
            Some("S3Object".to_string())
        } else if f
            .resources_affected()
            .and_then(|r| r.s3_bucket())
            .is_some()
        {
            Some("S3Bucket".to_string())
        } else {
            None
        };

        Self {
            id: f.id().unwrap_or_default().to_string(),
            title: f.title().map(|s| s.to_string()),
            description: f.description().map(|s| s.to_string()),
            severity: f
                .severity()
                .and_then(|s| s.description())
                .map(|d| d.as_str().to_string()),
            finding_type: f.r#type().map(|t| t.as_str().to_string()),
            category: f.category().map(|c| c.as_str().to_string()),
            resource_type,
            bucket_name,
            created_at: f.created_at().map(format_datetime),
            updated_at: f.updated_at().map(format_datetime),
            archived: f.archived().unwrap_or(false),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct MacieBucketSummary {
    pub bucket_name: String,
    pub account_id: Option<String>,
    pub region: Option<String>,
    pub classifiable_object_count: Option<i64>,
    pub classifiable_size_in_bytes: Option<i64>,
    pub is_publicly_accessible: bool,
    pub shared_access: Option<String>,
    pub error_code: Option<String>,
}

impl From<BucketMetadata> for MacieBucketSummary {
    fn from(m: BucketMetadata) -> Self {
        let is_publicly_accessible = m
            .public_access()
            .and_then(|pa| pa.effective_permission())
            .map(|ep| ep.as_str().eq_ignore_ascii_case("PUBLIC"))
            .unwrap_or(false);

        Self {
            bucket_name: m.bucket_name().unwrap_or_default().to_string(),
            account_id: m.account_id().map(|s| s.to_string()),
            region: m.region().map(|s| s.to_string()),
            classifiable_object_count: m.classifiable_object_count(),
            classifiable_size_in_bytes: m.classifiable_size_in_bytes(),
            is_publicly_accessible,
            shared_access: m.shared_access().map(|s| s.as_str().to_string()),
            error_code: m.error_code().map(|e| e.as_str().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macie_finding_fields() {
        let finding = MacieFinding {
            id: "abc123".to_string(),
            title: Some("Sensitive data found".to_string()),
            description: Some("PII data found in S3 object".to_string()),
            severity: Some("High".to_string()),
            finding_type: Some("SensitiveData:S3Object/Personal".to_string()),
            category: Some("CLASSIFICATION".to_string()),
            resource_type: Some("S3Object".to_string()),
            bucket_name: Some("my-bucket".to_string()),
            created_at: Some("2024-01-01T00:00:00+00:00".to_string()),
            updated_at: Some("2024-01-02T00:00:00+00:00".to_string()),
            archived: false,
        };
        assert_eq!(finding.id, "abc123");
        assert_eq!(finding.severity, Some("High".to_string()));
        assert_eq!(finding.finding_type, Some("SensitiveData:S3Object/Personal".to_string()));
        assert_eq!(finding.category, Some("CLASSIFICATION".to_string()));
        assert_eq!(finding.bucket_name, Some("my-bucket".to_string()));
        assert!(!finding.archived);
    }

    #[test]
    fn test_macie_finding_archived() {
        let finding = MacieFinding {
            id: "xyz789".to_string(),
            title: None,
            description: None,
            severity: Some("Low".to_string()),
            finding_type: None,
            category: Some("POLICY".to_string()),
            resource_type: None,
            bucket_name: None,
            created_at: None,
            updated_at: None,
            archived: true,
        };
        assert!(finding.archived);
        assert!(finding.title.is_none());
        assert!(finding.bucket_name.is_none());
    }

    #[test]
    fn test_macie_bucket_summary_fields() {
        let summary = MacieBucketSummary {
            bucket_name: "secure-bucket".to_string(),
            account_id: Some("123456789012".to_string()),
            region: Some("us-east-1".to_string()),
            classifiable_object_count: Some(100),
            classifiable_size_in_bytes: Some(1048576),
            is_publicly_accessible: false,
            shared_access: Some("NOT_SHARED".to_string()),
            error_code: None,
        };
        assert_eq!(summary.bucket_name, "secure-bucket");
        assert_eq!(summary.account_id, Some("123456789012".to_string()));
        assert_eq!(summary.classifiable_object_count, Some(100));
        assert_eq!(summary.classifiable_size_in_bytes, Some(1048576));
        assert!(!summary.is_publicly_accessible);
        assert_eq!(summary.shared_access, Some("NOT_SHARED".to_string()));
        assert!(summary.error_code.is_none());
    }

    #[test]
    fn test_macie_bucket_summary_public() {
        let summary = MacieBucketSummary {
            bucket_name: "public-bucket".to_string(),
            account_id: None,
            region: Some("us-west-2".to_string()),
            classifiable_object_count: None,
            classifiable_size_in_bytes: None,
            is_publicly_accessible: true,
            shared_access: Some("EXTERNAL".to_string()),
            error_code: Some("ACCESS_DENIED".to_string()),
        };
        assert!(summary.is_publicly_accessible);
        assert_eq!(summary.shared_access, Some("EXTERNAL".to_string()));
        assert_eq!(summary.error_code, Some("ACCESS_DENIED".to_string()));
    }
}
