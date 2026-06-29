use async_graphql::SimpleObject;

use crate::aws::acm_pca::PrivateCaInfo;
use crate::schema::ec2::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct PrivateCa {
    pub arn: Option<String>,
    pub type_: Option<String>,
    pub status: Option<String>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    pub serial: Option<String>,
    pub subject: Option<PcaSubject>,
    pub created_at: Option<String>,
    pub last_state_change_at: Option<String>,
    pub revocation_configuration: Option<PcaRevocationConfig>,
    pub tags: Vec<Tag>,
}

impl From<PrivateCaInfo> for PrivateCa {
    fn from(info: PrivateCaInfo) -> Self {
        let PrivateCaInfo { inner: ca, tags } = info;
        Self {
            arn: ca.arn().map(|s| s.to_string()),
            type_: ca.r#type().map(|t| t.as_str().to_string()),
            status: ca.status().map(|s| s.as_str().to_string()),
            not_before: ca.not_before().map(|d| d.to_string()),
            not_after: ca.not_after().map(|d| d.to_string()),
            serial: ca.serial().map(|s| s.to_string()),
            subject: ca
                .certificate_authority_configuration()
                .and_then(|c| c.subject().cloned())
                .map(PcaSubject::from),
            created_at: ca.created_at().map(|d| d.to_string()),
            last_state_change_at: ca.last_state_change_at().map(|d| d.to_string()),
            revocation_configuration: ca
                .revocation_configuration()
                .cloned()
                .map(PcaRevocationConfig::from),
            tags: tags
                .into_iter()
                .map(|(k, v)| Tag { key: k, value: v })
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct PcaSubject {
    pub common_name: Option<String>,
    pub organization: Option<String>,
    pub country: Option<String>,
    pub state: Option<String>,
}

impl From<aws_sdk_acmpca::types::Asn1Subject> for PcaSubject {
    fn from(s: aws_sdk_acmpca::types::Asn1Subject) -> Self {
        Self {
            common_name: s.common_name().map(|v| v.to_string()),
            organization: s.organization().map(|v| v.to_string()),
            country: s.country().map(|v| v.to_string()),
            state: s.state().map(|v| v.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct PcaRevocationConfig {
    pub crl_enabled: bool,
    pub ocsp_enabled: bool,
}

impl From<aws_sdk_acmpca::types::RevocationConfiguration> for PcaRevocationConfig {
    fn from(r: aws_sdk_acmpca::types::RevocationConfiguration) -> Self {
        Self {
            crl_enabled: r.crl_configuration().map(|c| c.enabled()).unwrap_or(false),
            ocsp_enabled: r
                .ocsp_configuration()
                .map(|c| c.enabled())
                .unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::acm_pca::PrivateCaInfo;

    fn make_ca_info(ca: aws_sdk_acmpca::types::CertificateAuthority) -> PrivateCaInfo {
        PrivateCaInfo { inner: ca, tags: vec![] }
    }

    fn make_ca_info_with_tags(
        ca: aws_sdk_acmpca::types::CertificateAuthority,
        tags: Vec<(String, String)>,
    ) -> PrivateCaInfo {
        PrivateCaInfo { inner: ca, tags }
    }

    #[test]
    fn test_private_ca_from_minimal() {
        let ca = aws_sdk_acmpca::types::CertificateAuthority::builder().build();
        let result = PrivateCa::from(make_ca_info(ca));
        assert!(result.arn.is_none());
        assert!(result.type_.is_none());
        assert!(result.status.is_none());
        assert!(result.subject.is_none());
        assert!(result.revocation_configuration.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_private_ca_from_with_arn_and_status() {
        let ca = aws_sdk_acmpca::types::CertificateAuthority::builder()
            .arn("arn:aws:acm-pca:us-east-1:123456789012:certificate-authority/abc123")
            .status(aws_sdk_acmpca::types::CertificateAuthorityStatus::Active)
            .r#type(aws_sdk_acmpca::types::CertificateAuthorityType::Root)
            .serial("01:23:45:67:89")
            .build();
        let result = PrivateCa::from(make_ca_info(ca));
        assert_eq!(
            result.arn,
            Some("arn:aws:acm-pca:us-east-1:123456789012:certificate-authority/abc123".to_string())
        );
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.type_, Some("ROOT".to_string()));
        assert_eq!(result.serial, Some("01:23:45:67:89".to_string()));
    }

    #[test]
    fn test_private_ca_from_with_tags() {
        let ca = aws_sdk_acmpca::types::CertificateAuthority::builder()
            .arn("arn:aws:acm-pca:us-east-1:123456789012:certificate-authority/abc123")
            .build();
        let tags = vec![
            ("env".to_string(), "prod".to_string()),
            ("team".to_string(), "infra".to_string()),
        ];
        let result = PrivateCa::from(make_ca_info_with_tags(ca, tags));
        assert_eq!(result.tags.len(), 2);
        assert_eq!(result.tags[0].key, "env");
        assert_eq!(result.tags[0].value, "prod");
        assert_eq!(result.tags[1].key, "team");
        assert_eq!(result.tags[1].value, "infra");
    }

    #[test]
    fn test_pca_subject_from_minimal() {
        let subject = aws_sdk_acmpca::types::Asn1Subject::builder().build();
        let result = PcaSubject::from(subject);
        assert!(result.common_name.is_none());
        assert!(result.organization.is_none());
        assert!(result.country.is_none());
        assert!(result.state.is_none());
    }

    #[test]
    fn test_pca_subject_from_full() {
        let subject = aws_sdk_acmpca::types::Asn1Subject::builder()
            .common_name("Example Root CA")
            .organization("Example Corp")
            .country("US")
            .state("California")
            .build();
        let result = PcaSubject::from(subject);
        assert_eq!(result.common_name, Some("Example Root CA".to_string()));
        assert_eq!(result.organization, Some("Example Corp".to_string()));
        assert_eq!(result.country, Some("US".to_string()));
        assert_eq!(result.state, Some("California".to_string()));
    }

    #[test]
    fn test_pca_revocation_config_crl_only() {
        let crl = aws_sdk_acmpca::types::CrlConfiguration::builder()
            .enabled(true)
            .build()
            .unwrap();
        let rev = aws_sdk_acmpca::types::RevocationConfiguration::builder()
            .crl_configuration(crl)
            .build();
        let result = PcaRevocationConfig::from(rev);
        assert!(result.crl_enabled);
        assert!(!result.ocsp_enabled);
    }

    #[test]
    fn test_pca_revocation_config_none() {
        let rev = aws_sdk_acmpca::types::RevocationConfiguration::builder().build();
        let result = PcaRevocationConfig::from(rev);
        assert!(!result.crl_enabled);
        assert!(!result.ocsp_enabled);
    }

    #[test]
    fn test_private_ca_empty_tags() {
        let ca = aws_sdk_acmpca::types::CertificateAuthority::builder().build();
        let result = PrivateCa::from(make_ca_info(ca));
        assert!(result.tags.is_empty());
    }
}
