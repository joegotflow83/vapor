use async_graphql::SimpleObject;

use crate::aws::ecr::ImageScanFindingsResult;

/// An ECR repository.
#[derive(SimpleObject, Clone)]
pub struct EcrRepository {
    pub registry_id: Option<String>,
    pub repository_id: Option<String>,
    pub repository_name: Option<String>,
    pub repository_uri: Option<String>,
    pub created_at: Option<String>,
    pub image_tag_mutability: Option<String>,
    pub scan_on_push: Option<bool>,
    pub encryption_type: Option<String>,
    pub kms_key: Option<String>,
}

impl From<aws_sdk_ecr::types::Repository> for EcrRepository {
    fn from(r: aws_sdk_ecr::types::Repository) -> Self {
        Self {
            registry_id: r.registry_id().map(|s| s.to_string()),
            // The SDK `Repository` exposes no `repository_id`; only ARN/name/URI.
            repository_id: None,
            repository_name: r.repository_name().map(|s| s.to_string()),
            repository_uri: r.repository_uri().map(|s| s.to_string()),
            created_at: r.created_at().map(|d| d.to_string()),
            image_tag_mutability: r.image_tag_mutability().map(|m| m.as_str().to_string()),
            scan_on_push: r.image_scanning_configuration().map(|c| c.scan_on_push()),
            encryption_type: r
                .encryption_configuration()
                .map(|c| c.encryption_type().as_str().to_string()),
            kms_key: r
                .encryption_configuration()
                .and_then(|c| c.kms_key())
                .map(|s| s.to_string()),
        }
    }
}

/// Metadata for an image in an ECR repository.
#[derive(SimpleObject, Clone)]
pub struct EcrImage {
    pub registry_id: Option<String>,
    pub repository_name: Option<String>,
    pub image_digest: Option<String>,
    pub image_tags: Vec<String>,
    pub image_size_in_bytes: Option<i64>,
    pub image_pushed_at: Option<String>,
    pub image_scan_status: Option<String>,
    pub artifact_media_type: Option<String>,
    pub image_manifest_media_type: Option<String>,
}

impl From<aws_sdk_ecr::types::ImageDetail> for EcrImage {
    fn from(img: aws_sdk_ecr::types::ImageDetail) -> Self {
        let image_tags = img
            .image_tags()
            .iter()
            .map(|s| s.to_string())
            .collect();

        Self {
            registry_id: img.registry_id().map(|s| s.to_string()),
            repository_name: img.repository_name().map(|s| s.to_string()),
            image_digest: img.image_digest().map(|s| s.to_string()),
            image_tags,
            image_size_in_bytes: img.image_size_in_bytes(),
            image_pushed_at: img.image_pushed_at().map(|d| d.to_string()),
            image_scan_status: img
                .image_scan_status()
                .and_then(|s| s.status())
                .map(|s| s.as_str().to_string()),
            artifact_media_type: img.artifact_media_type().map(|s| s.to_string()),
            image_manifest_media_type: img.image_manifest_media_type().map(|s| s.to_string()),
        }
    }
}

/// A key-value attribute on an ECR image scan finding (e.g. package name, CVE ID).
#[derive(SimpleObject, Clone)]
pub struct EcrFindingAttribute {
    pub key: String,
    pub value: Option<String>,
}

impl From<&aws_sdk_ecr::types::Attribute> for EcrFindingAttribute {
    fn from(a: &aws_sdk_ecr::types::Attribute) -> Self {
        Self {
            key: a.key().to_string(),
            value: a.value().map(|s| s.to_string()),
        }
    }
}

/// A single CVE/vulnerability finding from an ECR image scan.
#[derive(SimpleObject, Clone)]
pub struct EcrImageScanFinding {
    /// CVE identifier or finding name.
    pub name: Option<String>,
    pub description: Option<String>,
    /// Link to the CVE details page.
    pub uri: Option<String>,
    /// Severity: CRITICAL, HIGH, MEDIUM, LOW, INFORMATIONAL, UNDEFINED.
    pub severity: Option<String>,
    pub attributes: Vec<EcrFindingAttribute>,
}

impl From<&aws_sdk_ecr::types::ImageScanFinding> for EcrImageScanFinding {
    fn from(f: &aws_sdk_ecr::types::ImageScanFinding) -> Self {
        Self {
            name: f.name().map(|s| s.to_string()),
            description: f.description().map(|s| s.to_string()),
            uri: f.uri().map(|s| s.to_string()),
            severity: f.severity().map(|s| s.as_str().to_string()),
            attributes: f.attributes().iter().map(EcrFindingAttribute::from).collect(),
        }
    }
}

/// Severity bucket with its finding count.
#[derive(SimpleObject, Clone)]
pub struct EcrSeverityCount {
    pub severity: String,
    pub count: i64,
}

/// Aggregated image scan findings for a specific image digest.
/// Security value: detect vulnerable container images with CVE details.
#[derive(SimpleObject, Clone)]
pub struct EcrImageScanFindings {
    pub image_tags: Vec<String>,
    pub scan_status: Option<String>,
    pub scan_completed_at: Option<String>,
    /// Finding counts broken down by severity (CRITICAL, HIGH, MEDIUM, LOW, etc.).
    pub finding_severity_counts: Vec<EcrSeverityCount>,
    pub findings: Vec<EcrImageScanFinding>,
}

impl From<ImageScanFindingsResult> for EcrImageScanFindings {
    fn from(r: ImageScanFindingsResult) -> Self {
        let finding_severity_counts = r
            .finding_severity_counts
            .into_iter()
            .map(|(severity, count)| EcrSeverityCount { severity, count })
            .collect();
        let findings = r.findings.iter().map(EcrImageScanFinding::from).collect();
        Self {
            image_tags: r.image_tags,
            scan_status: r.scan_status,
            scan_completed_at: r.scan_completed_at,
            finding_severity_counts,
            findings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_ecr::types::{
        EncryptionConfiguration, EncryptionType, ImageDetail, ImageScanningConfiguration,
        ImageScanStatus, ImageTagMutability, Repository, ScanStatus,
    };

    #[test]
    fn test_ecr_repository_all_fields() {
        let scan_config = ImageScanningConfiguration::builder()
            .scan_on_push(true)
            .build();
        let encryption_config = EncryptionConfiguration::builder()
            .encryption_type(EncryptionType::Aes256)
            .build()
            .unwrap();

        let sdk = Repository::builder()
            .registry_id("123456789012")
            .repository_name("my-app")
            .repository_uri("123456789012.dkr.ecr.us-east-1.amazonaws.com/my-app")
            .image_tag_mutability(ImageTagMutability::Mutable)
            .image_scanning_configuration(scan_config)
            .encryption_configuration(encryption_config)
            .build();

        let repo = EcrRepository::from(sdk);
        assert_eq!(repo.registry_id, Some("123456789012".to_string()));
        // `Repository` exposes no repository_id; mapping always yields None.
        assert!(repo.repository_id.is_none());
        assert_eq!(repo.repository_name, Some("my-app".to_string()));
        assert_eq!(repo.repository_uri, Some("123456789012.dkr.ecr.us-east-1.amazonaws.com/my-app".to_string()));
        assert_eq!(repo.image_tag_mutability, Some("MUTABLE".to_string()));
        assert_eq!(repo.scan_on_push, Some(true));
        assert_eq!(repo.encryption_type, Some("AES256".to_string()));
        assert!(repo.kms_key.is_none());
    }

    #[test]
    fn test_ecr_repository_immutable_tags() {
        let sdk = Repository::builder()
            .repository_name("locked-repo")
            .image_tag_mutability(ImageTagMutability::Immutable)
            .build();

        let repo = EcrRepository::from(sdk);
        assert_eq!(repo.repository_name, Some("locked-repo".to_string()));
        assert_eq!(repo.image_tag_mutability, Some("IMMUTABLE".to_string()));
    }

    #[test]
    fn test_ecr_repository_with_kms_encryption() {
        let encryption_config = EncryptionConfiguration::builder()
            .encryption_type(EncryptionType::Kms)
            .kms_key("arn:aws:kms:us-east-1:123456789012:key/abc-123")
            .build()
            .unwrap();

        let sdk = Repository::builder()
            .repository_name("encrypted-repo")
            .encryption_configuration(encryption_config)
            .build();

        let repo = EcrRepository::from(sdk);
        assert_eq!(repo.repository_name, Some("encrypted-repo".to_string()));
        assert_eq!(repo.encryption_type, Some("KMS".to_string()));
        assert_eq!(repo.kms_key, Some("arn:aws:kms:us-east-1:123456789012:key/abc-123".to_string()));
    }

    #[test]
    fn test_ecr_repository_minimal() {
        let sdk = Repository::builder().build();

        let repo = EcrRepository::from(sdk);
        assert!(repo.registry_id.is_none());
        assert!(repo.repository_name.is_none());
        assert!(repo.scan_on_push.is_none());
        assert!(repo.encryption_type.is_none());
        assert!(repo.kms_key.is_none());
    }

    #[test]
    fn test_ecr_image_all_fields() {
        let scan_status = ImageScanStatus::builder()
            .status(ScanStatus::Complete)
            .build();

        let sdk = ImageDetail::builder()
            .registry_id("123456789012")
            .repository_name("my-app")
            .image_digest("sha256:abc123def456")
            .image_tags("latest")
            .image_tags("v1.0")
            .image_size_in_bytes(10_485_760i64)
            .image_scan_status(scan_status)
            .artifact_media_type("application/vnd.oci.image.manifest.v1+json")
            .image_manifest_media_type("application/vnd.docker.distribution.manifest.v2+json")
            .build();

        let img = EcrImage::from(sdk);
        assert_eq!(img.registry_id, Some("123456789012".to_string()));
        assert_eq!(img.repository_name, Some("my-app".to_string()));
        assert_eq!(img.image_digest, Some("sha256:abc123def456".to_string()));
        assert_eq!(img.image_tags, vec!["latest".to_string(), "v1.0".to_string()]);
        assert_eq!(img.image_size_in_bytes, Some(10_485_760i64));
        assert_eq!(img.image_scan_status, Some("COMPLETE".to_string()));
        assert_eq!(img.artifact_media_type, Some("application/vnd.oci.image.manifest.v1+json".to_string()));
        assert_eq!(img.image_manifest_media_type, Some("application/vnd.docker.distribution.manifest.v2+json".to_string()));
    }

    #[test]
    fn test_ecr_image_no_tags() {
        let sdk = ImageDetail::builder()
            .image_digest("sha256:def456")
            .build();

        let img = EcrImage::from(sdk);
        assert_eq!(img.image_digest, Some("sha256:def456".to_string()));
        assert!(img.image_tags.is_empty());
        assert!(img.image_scan_status.is_none());
        assert!(img.artifact_media_type.is_none());
    }

    #[test]
    fn test_ecr_image_failed_scan() {
        let scan_status = ImageScanStatus::builder()
            .status(ScanStatus::Failed)
            .build();

        let sdk = ImageDetail::builder()
            .repository_name("my-app")
            .image_scan_status(scan_status)
            .build();

        let img = EcrImage::from(sdk);
        assert_eq!(img.image_scan_status, Some("FAILED".to_string()));
    }

    #[test]
    fn test_ecr_finding_attribute() {
        use aws_sdk_ecr::types::Attribute;
        let attr = Attribute::builder()
            .key("package_name")
            .value("openssl")
            .build()
            .unwrap();
        let out = EcrFindingAttribute::from(&attr);
        assert_eq!(out.key, "package_name");
        assert_eq!(out.value, Some("openssl".to_string()));
    }

    #[test]
    fn test_ecr_finding_attribute_no_value() {
        use aws_sdk_ecr::types::Attribute;
        let attr = Attribute::builder()
            .key("cve_id")
            .build()
            .unwrap();
        let out = EcrFindingAttribute::from(&attr);
        assert_eq!(out.key, "cve_id");
        assert!(out.value.is_none());
    }

    #[test]
    fn test_ecr_image_scan_finding() {
        use aws_sdk_ecr::types::{Attribute, FindingSeverity, ImageScanFinding};
        let attr = Attribute::builder()
            .key("package_name")
            .value("openssl")
            .build()
            .unwrap();
        let sdk = ImageScanFinding::builder()
            .name("CVE-2023-0001")
            .description("OpenSSL vulnerability")
            .uri("https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2023-0001")
            .severity(FindingSeverity::High)
            .attributes(attr)
            .build();
        let out = EcrImageScanFinding::from(&sdk);
        assert_eq!(out.name, Some("CVE-2023-0001".to_string()));
        assert_eq!(out.description, Some("OpenSSL vulnerability".to_string()));
        assert_eq!(out.severity, Some("HIGH".to_string()));
        assert_eq!(out.attributes.len(), 1);
        assert_eq!(out.attributes[0].key, "package_name");
    }

    #[test]
    fn test_ecr_image_scan_findings_from_result() {
        use crate::aws::ecr::ImageScanFindingsResult;
        use aws_sdk_ecr::types::{FindingSeverity, ImageScanFinding as SdkImageScanFinding};
        let sdk_finding = SdkImageScanFinding::builder()
            .name("CVE-2023-9999")
            .severity(FindingSeverity::Critical)
            .build();
        let result = ImageScanFindingsResult {
            image_tags: vec!["latest".to_string()],
            scan_status: Some("COMPLETE".to_string()),
            scan_completed_at: Some("2026-06-16T00:00:00Z".to_string()),
            finding_severity_counts: vec![
                ("CRITICAL".to_string(), 1i64),
                ("HIGH".to_string(), 3i64),
            ],
            findings: vec![sdk_finding],
        };
        let out = EcrImageScanFindings::from(result);
        assert_eq!(out.image_tags, vec!["latest".to_string()]);
        assert_eq!(out.scan_status, Some("COMPLETE".to_string()));
        assert_eq!(out.finding_severity_counts.len(), 2);
        assert_eq!(out.finding_severity_counts[0].severity, "CRITICAL");
        assert_eq!(out.finding_severity_counts[0].count, 1);
        assert_eq!(out.findings.len(), 1);
        assert_eq!(out.findings[0].name, Some("CVE-2023-9999".to_string()));
        assert_eq!(out.findings[0].severity, Some("CRITICAL".to_string()));
    }
}
