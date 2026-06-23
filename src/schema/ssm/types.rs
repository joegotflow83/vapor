use async_graphql::{Enum, InputObject, SimpleObject};

use crate::schema::ec2::types::Tag;

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PingStatus {
    Online,
    ConnectionLost,
    Inactive,
}

impl PingStatus {
    pub fn from_sdk(s: &aws_sdk_ssm::types::PingStatus) -> Self {
        match s {
            aws_sdk_ssm::types::PingStatus::Online => Self::Online,
            aws_sdk_ssm::types::PingStatus::ConnectionLost => Self::ConnectionLost,
            aws_sdk_ssm::types::PingStatus::Inactive => Self::Inactive,
            _ => Self::Inactive,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PlatformType {
    Windows,
    Linux,
    MacOs,
}

impl PlatformType {
    pub fn from_sdk(s: &aws_sdk_ssm::types::PlatformType) -> Self {
        match s {
            aws_sdk_ssm::types::PlatformType::Windows => Self::Windows,
            aws_sdk_ssm::types::PlatformType::Linux => Self::Linux,
            aws_sdk_ssm::types::PlatformType::Macos => Self::MacOs,
            _ => Self::Linux,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ParameterType {
    String,
    StringList,
    SecureString,
}

impl ParameterType {
    pub fn from_sdk(s: &aws_sdk_ssm::types::ParameterType) -> Self {
        match s {
            aws_sdk_ssm::types::ParameterType::String => Self::String,
            aws_sdk_ssm::types::ParameterType::StringList => Self::StringList,
            aws_sdk_ssm::types::ParameterType::SecureString => Self::SecureString,
            _ => Self::String,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ParameterTier {
    Standard,
    Advanced,
    IntelligentTiering,
}

impl ParameterTier {
    pub fn from_sdk(s: &aws_sdk_ssm::types::ParameterTier) -> Self {
        match s {
            aws_sdk_ssm::types::ParameterTier::Standard => Self::Standard,
            aws_sdk_ssm::types::ParameterTier::Advanced => Self::Advanced,
            aws_sdk_ssm::types::ParameterTier::IntelligentTiering => Self::IntelligentTiering,
            _ => Self::Standard,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ManagedInstance {
    pub instance_id: String,
    pub ping_status: PingStatus,
    pub last_ping_time: Option<String>,
    pub platform_type: Option<PlatformType>,
    pub platform_name: Option<String>,
    pub platform_version: Option<String>,
    pub agent_version: Option<String>,
    pub ip_address: Option<String>,
    pub computer_name: Option<String>,
    pub name: Option<String>,
    pub resource_type: Option<String>,
    pub iam_role: Option<String>,
    pub registration_date: Option<String>,
}

impl From<aws_sdk_ssm::types::InstanceInformation> for ManagedInstance {
    fn from(info: aws_sdk_ssm::types::InstanceInformation) -> Self {
        Self {
            instance_id: info.instance_id().unwrap_or_default().to_string(),
            ping_status: info
                .ping_status()
                .map(PingStatus::from_sdk)
                .unwrap_or(PingStatus::Inactive),
            last_ping_time: info.last_ping_date_time().map(|d| d.to_string()),
            platform_type: info.platform_type().map(PlatformType::from_sdk),
            platform_name: info.platform_name().map(|s| s.to_string()),
            platform_version: info.platform_version().map(|s| s.to_string()),
            agent_version: info.agent_version().map(|s| s.to_string()),
            ip_address: info.ip_address().map(|s| s.to_string()),
            computer_name: info.computer_name().map(|s| s.to_string()),
            name: info.name().map(|s| s.to_string()),
            resource_type: info.resource_type().map(|r| r.as_str().to_string()),
            iam_role: info.iam_role().map(|s| s.to_string()),
            registration_date: info.registration_date().map(|d| d.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Parameter {
    pub name: Option<String>,
    pub parameter_type: Option<ParameterType>,
    pub value: Option<String>,
    pub version: Option<i64>,
    pub last_modified_date: Option<String>,
    pub arn: Option<String>,
    pub data_type: Option<String>,
    pub tier: Option<ParameterTier>,
}

impl From<aws_sdk_ssm::types::Parameter> for Parameter {
    fn from(p: aws_sdk_ssm::types::Parameter) -> Self {
        Self {
            name: p.name().map(|s| s.to_string()),
            parameter_type: p.r#type().map(ParameterType::from_sdk),
            value: p.value().map(|s| s.to_string()),
            version: Some(p.version()),
            last_modified_date: p.last_modified_date().map(|d| d.to_string()),
            arn: p.arn().map(|s| s.to_string()),
            data_type: p.data_type().map(|s| s.to_string()),
            tier: None,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ParameterMeta {
    pub name: Option<String>,
    pub parameter_type: Option<ParameterType>,
    pub tier: Option<ParameterTier>,
    pub version: Option<i64>,
    pub last_modified_date: Option<String>,
    pub description: Option<String>,
    pub arn: Option<String>,
    pub data_type: Option<String>,
    pub key_id: Option<String>,
    pub policies: Vec<String>,
}

impl From<aws_sdk_ssm::types::ParameterMetadata> for ParameterMeta {
    fn from(m: aws_sdk_ssm::types::ParameterMetadata) -> Self {
        Self {
            name: m.name().map(|s| s.to_string()),
            parameter_type: m.r#type().map(ParameterType::from_sdk),
            tier: m.tier().map(ParameterTier::from_sdk),
            version: Some(m.version()),
            last_modified_date: m.last_modified_date().map(|d| d.to_string()),
            description: m.description().map(|s| s.to_string()),
            arn: m.arn().map(|s| s.to_string()),
            data_type: m.data_type().map(|s| s.to_string()),
            key_id: m.key_id().map(|s| s.to_string()),
            policies: m
                .policies()
                .iter()
                .filter_map(|p| p.policy_text().map(|t| t.to_string()))
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SsmDocument {
    pub name: Option<String>,
    pub document_type: Option<String>,
    pub document_format: Option<String>,
    pub document_version: Option<String>,
    pub status: Option<String>,
    pub owner: Option<String>,
    pub created_date: Option<String>,
    pub description: Option<String>,
    pub platform_types: Vec<String>,
    pub schema_version: Option<String>,
    pub target_type: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ssm::types::DocumentIdentifier> for SsmDocument {
    fn from(doc: aws_sdk_ssm::types::DocumentIdentifier) -> Self {
        Self {
            name: doc.name().map(|s| s.to_string()),
            document_type: doc.document_type().map(|t| t.as_str().to_string()),
            document_format: doc.document_format().map(|f| f.as_str().to_string()),
            document_version: doc.document_version().map(|s| s.to_string()),
            // DocumentIdentifier from list_documents does not carry status or description;
            // those are only available via describe_document / get_document.
            status: None,
            owner: doc.owner().map(|s| s.to_string()),
            created_date: doc.created_date().map(|d| d.to_string()),
            description: None,
            platform_types: doc
                .platform_types()
                .iter()
                .map(|p| p.as_str().to_string())
                .collect(),
            schema_version: doc.schema_version().map(|s| s.to_string()),
            target_type: doc.target_type().map(|s| s.to_string()),
            tags: doc
                .tags()
                .iter()
                .map(|t| Tag {
                    key: t.key().to_string(),
                    value: t.value().to_string(),
                })
                .collect(),
        }
    }
}

#[derive(InputObject)]
pub struct ParameterFilter {
    pub key: String,
    pub option: Option<String>,
    pub values: Vec<String>,
}

impl ParameterFilter {
    pub fn to_sdk_filter(&self) -> aws_sdk_ssm::types::ParameterStringFilter {
        aws_sdk_ssm::types::ParameterStringFilter::builder()
            .key(&self.key)
            .set_option(self.option.clone())
            .set_values(Some(self.values.clone()))
            .build()
            .expect("key is always provided")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- PingStatus ---

    #[test]
    fn test_ping_status_all_variants() {
        assert_eq!(
            PingStatus::from_sdk(&aws_sdk_ssm::types::PingStatus::Online),
            PingStatus::Online
        );
        assert_eq!(
            PingStatus::from_sdk(&aws_sdk_ssm::types::PingStatus::ConnectionLost),
            PingStatus::ConnectionLost
        );
        assert_eq!(
            PingStatus::from_sdk(&aws_sdk_ssm::types::PingStatus::Inactive),
            PingStatus::Inactive
        );
    }

    // --- PlatformType ---

    #[test]
    fn test_platform_type_all_variants() {
        assert_eq!(
            PlatformType::from_sdk(&aws_sdk_ssm::types::PlatformType::Windows),
            PlatformType::Windows
        );
        assert_eq!(
            PlatformType::from_sdk(&aws_sdk_ssm::types::PlatformType::Linux),
            PlatformType::Linux
        );
        assert_eq!(
            PlatformType::from_sdk(&aws_sdk_ssm::types::PlatformType::Macos),
            PlatformType::MacOs
        );
    }

    // --- ParameterType ---

    #[test]
    fn test_parameter_type_all_variants() {
        assert_eq!(
            ParameterType::from_sdk(&aws_sdk_ssm::types::ParameterType::String),
            ParameterType::String
        );
        assert_eq!(
            ParameterType::from_sdk(&aws_sdk_ssm::types::ParameterType::StringList),
            ParameterType::StringList
        );
        assert_eq!(
            ParameterType::from_sdk(&aws_sdk_ssm::types::ParameterType::SecureString),
            ParameterType::SecureString
        );
    }

    // --- ParameterTier ---

    #[test]
    fn test_parameter_tier_all_variants() {
        assert_eq!(
            ParameterTier::from_sdk(&aws_sdk_ssm::types::ParameterTier::Standard),
            ParameterTier::Standard
        );
        assert_eq!(
            ParameterTier::from_sdk(&aws_sdk_ssm::types::ParameterTier::Advanced),
            ParameterTier::Advanced
        );
        assert_eq!(
            ParameterTier::from_sdk(&aws_sdk_ssm::types::ParameterTier::IntelligentTiering),
            ParameterTier::IntelligentTiering
        );
    }

    // --- ManagedInstance ---

    #[test]
    fn test_managed_instance_from_sdk() {
        let sdk = aws_sdk_ssm::types::InstanceInformation::builder()
            .instance_id("i-1234567890abcdef0")
            .ping_status(aws_sdk_ssm::types::PingStatus::Online)
            .platform_type(aws_sdk_ssm::types::PlatformType::Linux)
            .platform_name("Amazon Linux 2")
            .platform_version("2.0")
            .agent_version("3.1.1804.0")
            .ip_address("10.0.1.42")
            .computer_name("ip-10-0-1-42")
            .name("my-managed-instance")
            .resource_type(aws_sdk_ssm::types::ResourceType::ManagedInstance)
            .iam_role("arn:aws:iam::123456789012:role/SSMRole")
            .build();

        let mi = ManagedInstance::from(sdk);
        assert_eq!(mi.instance_id, "i-1234567890abcdef0");
        assert_eq!(mi.ping_status, PingStatus::Online);
        assert_eq!(mi.platform_type, Some(PlatformType::Linux));
        assert_eq!(mi.platform_name, Some("Amazon Linux 2".to_string()));
        assert_eq!(mi.platform_version, Some("2.0".to_string()));
        assert_eq!(mi.agent_version, Some("3.1.1804.0".to_string()));
        assert_eq!(mi.ip_address, Some("10.0.1.42".to_string()));
        assert_eq!(mi.computer_name, Some("ip-10-0-1-42".to_string()));
        assert_eq!(mi.name, Some("my-managed-instance".to_string()));
        assert_eq!(mi.resource_type, Some("ManagedInstance".to_string()));
        assert_eq!(
            mi.iam_role,
            Some("arn:aws:iam::123456789012:role/SSMRole".to_string())
        );
        assert_eq!(mi.last_ping_time, None);
        assert_eq!(mi.registration_date, None);
    }

    #[test]
    fn test_managed_instance_from_sdk_empty() {
        let sdk = aws_sdk_ssm::types::InstanceInformation::builder().build();
        let mi = ManagedInstance::from(sdk);
        assert_eq!(mi.instance_id, "");
        assert_eq!(mi.ping_status, PingStatus::Inactive);
        assert_eq!(mi.platform_type, None);
        assert_eq!(mi.platform_name, None);
        assert_eq!(mi.platform_version, None);
        assert_eq!(mi.agent_version, None);
        assert_eq!(mi.ip_address, None);
        assert_eq!(mi.computer_name, None);
        assert_eq!(mi.name, None);
        assert_eq!(mi.resource_type, None);
        assert_eq!(mi.iam_role, None);
        assert_eq!(mi.last_ping_time, None);
        assert_eq!(mi.registration_date, None);
    }

    #[test]
    fn test_managed_instance_with_ping_time_and_registration() {
        let ping_ts = aws_sdk_ssm::primitives::DateTime::from_secs(1_700_000_000);
        let reg_ts = aws_sdk_ssm::primitives::DateTime::from_secs(1_600_000_000);
        let sdk = aws_sdk_ssm::types::InstanceInformation::builder()
            .instance_id("mi-abc123")
            .ping_status(aws_sdk_ssm::types::PingStatus::Online)
            .last_ping_date_time(ping_ts)
            .registration_date(reg_ts)
            .build();

        let mi = ManagedInstance::from(sdk);
        assert_eq!(mi.instance_id, "mi-abc123");
        assert!(mi.last_ping_time.is_some());
        assert!(mi.registration_date.is_some());
    }

    // --- Parameter ---

    #[test]
    fn test_parameter_from_sdk() {
        let ts = aws_sdk_ssm::primitives::DateTime::from_secs(1_000_000);
        let sdk = aws_sdk_ssm::types::Parameter::builder()
            .name("/app/db/password")
            .r#type(aws_sdk_ssm::types::ParameterType::SecureString)
            .value("s3cr3t")
            .version(7)
            .last_modified_date(ts)
            .arn("arn:aws:ssm:us-east-1:123456789012:parameter/app/db/password")
            .data_type("text")
            .build();

        let p = Parameter::from(sdk);
        assert_eq!(p.name, Some("/app/db/password".to_string()));
        assert_eq!(p.parameter_type, Some(ParameterType::SecureString));
        assert_eq!(p.value, Some("s3cr3t".to_string()));
        assert_eq!(p.version, Some(7));
        assert!(p.last_modified_date.is_some());
        assert_eq!(
            p.arn,
            Some("arn:aws:ssm:us-east-1:123456789012:parameter/app/db/password".to_string())
        );
        assert_eq!(p.data_type, Some("text".to_string()));
        assert_eq!(p.tier, None);
    }

    #[test]
    fn test_parameter_from_sdk_empty() {
        let sdk = aws_sdk_ssm::types::Parameter::builder().build();
        let p = Parameter::from(sdk);
        assert_eq!(p.name, None);
        assert_eq!(p.parameter_type, None);
        assert_eq!(p.value, None);
        assert_eq!(p.version, Some(0));
        assert_eq!(p.last_modified_date, None);
        assert_eq!(p.arn, None);
        assert_eq!(p.data_type, None);
        assert_eq!(p.tier, None);
    }

    // --- ParameterMeta ---

    #[test]
    fn test_parameter_meta_from_sdk() {
        let ts = aws_sdk_ssm::primitives::DateTime::from_secs(2_000_000);
        let sdk = aws_sdk_ssm::types::ParameterMetadata::builder()
            .name("/app/config/timeout")
            .r#type(aws_sdk_ssm::types::ParameterType::String)
            .tier(aws_sdk_ssm::types::ParameterTier::Advanced)
            .version(3)
            .last_modified_date(ts)
            .description("Connection timeout in seconds")
            .arn("arn:aws:ssm:us-east-1:123456789012:parameter/app/config/timeout")
            .data_type("text")
            .key_id("alias/my-key")
            .build();

        let m = ParameterMeta::from(sdk);
        assert_eq!(m.name, Some("/app/config/timeout".to_string()));
        assert_eq!(m.parameter_type, Some(ParameterType::String));
        assert_eq!(m.tier, Some(ParameterTier::Advanced));
        assert_eq!(m.version, Some(3));
        assert!(m.last_modified_date.is_some());
        assert_eq!(m.description, Some("Connection timeout in seconds".to_string()));
        assert_eq!(
            m.arn,
            Some("arn:aws:ssm:us-east-1:123456789012:parameter/app/config/timeout".to_string())
        );
        assert_eq!(m.data_type, Some("text".to_string()));
        assert_eq!(m.key_id, Some("alias/my-key".to_string()));
        assert!(m.policies.is_empty());
    }

    #[test]
    fn test_parameter_meta_from_sdk_empty() {
        let sdk = aws_sdk_ssm::types::ParameterMetadata::builder().build();
        let m = ParameterMeta::from(sdk);
        assert_eq!(m.name, None);
        assert_eq!(m.parameter_type, None);
        assert_eq!(m.tier, None);
        assert_eq!(m.version, Some(0));
        assert_eq!(m.last_modified_date, None);
        assert_eq!(m.description, None);
        assert_eq!(m.arn, None);
        assert_eq!(m.data_type, None);
        assert_eq!(m.key_id, None);
        assert!(m.policies.is_empty());
    }

    // --- SsmDocument ---

    #[test]
    fn test_ssm_document_from_sdk() {
        let ts = aws_sdk_ssm::primitives::DateTime::from_secs(1_600_000_000);
        let sdk = aws_sdk_ssm::types::DocumentIdentifier::builder()
            .name("AWS-RunShellScript")
            .document_type(aws_sdk_ssm::types::DocumentType::Command)
            .owner("Amazon")
            .set_platform_types(Some(vec![
                aws_sdk_ssm::types::PlatformType::Linux,
                aws_sdk_ssm::types::PlatformType::Windows,
            ]))
            .schema_version("2.2")
            .document_format(aws_sdk_ssm::types::DocumentFormat::Json)
            .target_type("/AWS::EC2::Instance")
            .document_version("1")
            .created_date(ts)
            .build();

        let doc = SsmDocument::from(sdk);
        assert_eq!(doc.name, Some("AWS-RunShellScript".to_string()));
        assert_eq!(doc.document_type, Some("Command".to_string()));
        assert_eq!(doc.owner, Some("Amazon".to_string()));
        assert_eq!(doc.platform_types.len(), 2);
        assert!(doc.platform_types.contains(&"Linux".to_string()));
        assert!(doc.platform_types.contains(&"Windows".to_string()));
        assert_eq!(doc.schema_version, Some("2.2".to_string()));
        assert_eq!(doc.document_format, Some("JSON".to_string()));
        assert_eq!(doc.target_type, Some("/AWS::EC2::Instance".to_string()));
        assert_eq!(doc.document_version, Some("1".to_string()));
        assert!(doc.created_date.is_some());
        assert_eq!(doc.status, None); // Not available from list_documents
        assert_eq!(doc.description, None); // Not available from list_documents
        assert!(doc.tags.is_empty());
    }

    #[test]
    fn test_ssm_document_from_sdk_empty() {
        let sdk = aws_sdk_ssm::types::DocumentIdentifier::builder().build();
        let doc = SsmDocument::from(sdk);
        assert_eq!(doc.name, None);
        assert_eq!(doc.document_type, None);
        assert_eq!(doc.owner, None);
        assert!(doc.platform_types.is_empty());
        assert_eq!(doc.schema_version, None);
        assert_eq!(doc.document_format, None);
        assert_eq!(doc.target_type, None);
        assert_eq!(doc.document_version, None);
        assert_eq!(doc.created_date, None);
        assert_eq!(doc.status, None);
        assert_eq!(doc.description, None);
        assert!(doc.tags.is_empty());
    }

    #[test]
    fn test_ssm_document_with_tags() {
        let sdk = aws_sdk_ssm::types::DocumentIdentifier::builder()
            .name("My-Runbook")
            .set_tags(Some(vec![aws_sdk_ssm::types::Tag::builder()
                .key("Environment")
                .value("prod")
                .build()
                .unwrap()]))
            .build();

        let doc = SsmDocument::from(sdk);
        assert_eq!(doc.tags.len(), 1);
        assert_eq!(doc.tags[0].key, "Environment");
        assert_eq!(doc.tags[0].value, "prod");
    }
}
