use async_graphql::SimpleObject;

use crate::schema::ec2::types::Tag;

/// An ACM certificate with full metadata.
///
/// Note: Certificates used by CloudFront must be in us-east-1. If vapor targets
/// another region those certificates will not appear in this list.
#[derive(SimpleObject, Clone)]
pub struct AcmCertificate {
    pub certificate_arn: Option<String>,
    /// Primary domain name (CN).
    pub domain_name: Option<String>,
    /// Subject alternative names (SANs).
    pub subject_alternative_names: Vec<String>,
    /// ISSUED | PENDING_VALIDATION | EXPIRED | FAILED | ...
    pub status: Option<String>,
    /// AMAZON_ISSUED | IMPORTED | PRIVATE
    pub type_: Option<String>,
    /// RSA_2048 | RSA_4096 | EC_prime256v1 | ...
    pub key_algorithm: Option<String>,
    pub issued_at: Option<String>,
    pub not_before: Option<String>,
    /// Certificate expiry timestamp — critical for alerting.
    pub not_after: Option<String>,
    /// ELIGIBLE | INELIGIBLE
    pub renewal_eligibility: Option<String>,
    /// ARNs of resources using this certificate (ELB, CloudFront, ...).
    pub in_use_by: Vec<String>,
    pub tags: Vec<Tag>,
}

impl AcmCertificate {
    pub fn from_detail_and_tags(
        detail: aws_sdk_acm::types::CertificateDetail,
        acm_tags: Vec<aws_sdk_acm::types::Tag>,
    ) -> Self {
        let tags = acm_tags
            .into_iter()
            .map(|t| Tag {
                key: t.key().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        Self {
            certificate_arn: detail.certificate_arn().map(|s| s.to_string()),
            domain_name: detail.domain_name().map(|s| s.to_string()),
            subject_alternative_names: detail
                .subject_alternative_names()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            status: detail.status().map(|s| s.as_str().to_string()),
            type_: detail.r#type().map(|s| s.as_str().to_string()),
            key_algorithm: detail.key_algorithm().map(|s| s.as_str().to_string()),
            issued_at: detail.issued_at().map(|d| d.to_string()),
            not_before: detail.not_before().map(|d| d.to_string()),
            not_after: detail.not_after().map(|d| d.to_string()),
            renewal_eligibility: detail.renewal_eligibility().map(|s| s.as_str().to_string()),
            in_use_by: detail.in_use_by().iter().map(|s| s.to_string()).collect(),
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acm_certificate_from_detail_minimal() {
        let detail = aws_sdk_acm::types::CertificateDetail::builder().build();
        let cert = AcmCertificate::from_detail_and_tags(detail, vec![]);
        assert!(cert.certificate_arn.is_none());
        assert!(cert.domain_name.is_none());
        assert!(cert.subject_alternative_names.is_empty());
        assert!(cert.status.is_none());
        assert!(cert.type_.is_none());
        assert!(cert.key_algorithm.is_none());
        assert!(cert.issued_at.is_none());
        assert!(cert.not_before.is_none());
        assert!(cert.not_after.is_none());
        assert!(cert.renewal_eligibility.is_none());
        assert!(cert.in_use_by.is_empty());
        assert!(cert.tags.is_empty());
    }

    #[test]
    fn test_acm_certificate_fields_populated() {
        let ts = aws_sdk_acm::primitives::DateTime::from_secs(1_700_000_000);
        let detail = aws_sdk_acm::types::CertificateDetail::builder()
            .certificate_arn("arn:aws:acm:us-east-1:123456789012:certificate/abc-123")
            .domain_name("example.com")
            .subject_alternative_names("www.example.com")
            .status(aws_sdk_acm::types::CertificateStatus::Issued)
            .r#type(aws_sdk_acm::types::CertificateType::AmazonIssued)
            .key_algorithm(aws_sdk_acm::types::KeyAlgorithm::Rsa2048)
            .issued_at(ts.clone())
            .not_before(ts.clone())
            .not_after(ts.clone())
            .renewal_eligibility(aws_sdk_acm::types::RenewalEligibility::Eligible)
            .in_use_by("arn:aws:elasticloadbalancing:us-east-1:123456789012:loadbalancer/app/my-lb/abc")
            .build();

        let cert = AcmCertificate::from_detail_and_tags(detail, vec![]);
        assert_eq!(
            cert.certificate_arn,
            Some("arn:aws:acm:us-east-1:123456789012:certificate/abc-123".to_string())
        );
        assert_eq!(cert.domain_name, Some("example.com".to_string()));
        assert_eq!(cert.subject_alternative_names, vec!["www.example.com"]);
        assert_eq!(cert.status, Some("ISSUED".to_string()));
        assert_eq!(cert.type_, Some("AMAZON_ISSUED".to_string()));
        assert_eq!(cert.key_algorithm, Some("RSA_2048".to_string()));
        assert!(cert.issued_at.is_some());
        assert!(cert.not_before.is_some());
        assert!(cert.not_after.is_some());
        assert_eq!(cert.renewal_eligibility, Some("ELIGIBLE".to_string()));
        assert_eq!(cert.in_use_by.len(), 1);
    }

    #[test]
    fn test_acm_certificate_tags_conversion() {
        let detail = aws_sdk_acm::types::CertificateDetail::builder().build();
        let acm_tag = aws_sdk_acm::types::Tag::builder()
            .key("Environment")
            .value("production")
            .build()
            .expect("key and value required");
        let cert = AcmCertificate::from_detail_and_tags(detail, vec![acm_tag]);
        assert_eq!(cert.tags.len(), 1);
        assert_eq!(cert.tags[0].key, "Environment");
        assert_eq!(cert.tags[0].value, "production");
    }

    #[test]
    fn test_acm_certificate_not_after_never_none_when_set() {
        let ts = aws_sdk_acm::primitives::DateTime::from_secs(1_800_000_000);
        let detail = aws_sdk_acm::types::CertificateDetail::builder()
            .not_after(ts)
            .build();
        let cert = AcmCertificate::from_detail_and_tags(detail, vec![]);
        assert!(cert.not_after.is_some(), "not_after must be exposed when present");
    }
}
