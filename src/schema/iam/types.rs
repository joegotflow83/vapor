use async_graphql::SimpleObject;

use crate::schema::common::types::Tag;

/// Decode a percent-encoded string (e.g. URL-encoded IAM policy documents).
fn percent_decode(s: &str) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(val);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// An IAM role.
#[derive(SimpleObject, Clone)]
pub struct IamRole {
    pub arn: String,
    pub role_id: String,
    pub role_name: String,
    pub path: String,
    pub create_date: Option<String>,
    pub description: Option<String>,
    /// Maximum session duration in seconds.
    pub max_session_duration: Option<i32>,
    /// URL-decoded JSON trust policy document.
    pub assume_role_policy_document: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_iam::types::Role> for IamRole {
    fn from(r: aws_sdk_iam::types::Role) -> Self {
        let tags = r
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().to_string(),
                value: t.value().to_string(),
            })
            .collect();

        Self {
            arn: r.arn().to_string(),
            role_id: r.role_id().to_string(),
            role_name: r.role_name().to_string(),
            path: r.path().to_string(),
            create_date: Some(r.create_date().to_string()),
            description: r.description().map(|s| s.to_string()),
            max_session_duration: r.max_session_duration(),
            assume_role_policy_document: r
                .assume_role_policy_document()
                .map(|s| percent_decode(s)),
            tags,
        }
    }
}

/// A customer-managed or AWS-managed IAM policy.
#[derive(SimpleObject, Clone)]
pub struct IamPolicy {
    pub arn: Option<String>,
    pub policy_id: Option<String>,
    pub policy_name: Option<String>,
    pub path: Option<String>,
    pub default_version_id: Option<String>,
    pub attachment_count: Option<i32>,
    pub is_attachable: Option<bool>,
    pub description: Option<String>,
    pub create_date: Option<String>,
    pub update_date: Option<String>,
}

impl From<aws_sdk_iam::types::Policy> for IamPolicy {
    fn from(p: aws_sdk_iam::types::Policy) -> Self {
        Self {
            arn: p.arn().map(|s| s.to_string()),
            policy_id: p.policy_id().map(|s| s.to_string()),
            policy_name: p.policy_name().map(|s| s.to_string()),
            path: p.path().map(|s| s.to_string()),
            default_version_id: p.default_version_id().map(|s| s.to_string()),
            attachment_count: p.attachment_count(),
            is_attachable: Some(p.is_attachable()),
            description: p.description().map(|s| s.to_string()),
            create_date: p.create_date().map(|d| d.to_string()),
            update_date: p.update_date().map(|d| d.to_string()),
        }
    }
}

/// An IAM user.
#[derive(SimpleObject, Clone)]
pub struct IamUser {
    pub arn: String,
    pub user_id: String,
    pub user_name: String,
    pub path: String,
    pub create_date: Option<String>,
    pub password_last_used: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_iam::types::User> for IamUser {
    fn from(u: aws_sdk_iam::types::User) -> Self {
        let tags = u
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().to_string(),
                value: t.value().to_string(),
            })
            .collect();

        Self {
            arn: u.arn().to_string(),
            user_id: u.user_id().to_string(),
            user_name: u.user_name().to_string(),
            path: u.path().to_string(),
            create_date: Some(u.create_date().to_string()),
            password_last_used: u.password_last_used().map(|d| d.to_string()),
            tags,
        }
    }
}

/// An IAM group.
#[derive(SimpleObject, Clone)]
pub struct IamGroup {
    pub arn: String,
    pub group_id: String,
    pub group_name: String,
    pub path: String,
    pub create_date: Option<String>,
}

impl From<aws_sdk_iam::types::Group> for IamGroup {
    fn from(g: aws_sdk_iam::types::Group) -> Self {
        Self {
            arn: g.arn().to_string(),
            group_id: g.group_id().to_string(),
            group_name: g.group_name().to_string(),
            path: g.path().to_string(),
            create_date: Some(g.create_date().to_string()),
        }
    }
}

/// A policy attached to an IAM role.
#[derive(SimpleObject, Clone)]
pub struct IamAttachedPolicy {
    pub policy_arn: Option<String>,
    pub policy_name: Option<String>,
}

impl From<aws_sdk_iam::types::AttachedPolicy> for IamAttachedPolicy {
    fn from(p: aws_sdk_iam::types::AttachedPolicy) -> Self {
        Self {
            policy_arn: p.policy_arn().map(|s| s.to_string()),
            policy_name: p.policy_name().map(|s| s.to_string()),
        }
    }
}

/// A versioned managed IAM policy document.
#[derive(SimpleObject, Clone)]
pub struct IamPolicyDocument {
    pub policy_arn: String,
    pub version_id: String,
    pub is_default_version: bool,
    /// URL-decoded JSON policy document.
    pub document: String,
    pub create_date: Option<String>,
}

impl From<(String, aws_sdk_iam::types::PolicyVersion)> for IamPolicyDocument {
    fn from((policy_arn, version): (String, aws_sdk_iam::types::PolicyVersion)) -> Self {
        Self {
            policy_arn,
            version_id: version.version_id().unwrap_or("").to_string(),
            is_default_version: version.is_default_version(),
            document: version.document().map(|d| percent_decode(d)).unwrap_or_default(),
            create_date: version.create_date().map(|d| d.to_string()),
        }
    }
}

/// An IAM user access key with last-used metadata.
#[derive(SimpleObject, Clone)]
pub struct IamAccessKey {
    pub access_key_id: String,
    pub user_name: String,
    /// "Active" or "Inactive".
    pub status: String,
    pub create_date: Option<String>,
    /// ISO-8601 timestamp of when the key was last used, or None if never used.
    pub last_used_date: Option<String>,
    /// AWS region where the key was last used (e.g. "us-east-1"), or None if never used.
    pub last_used_region: Option<String>,
    /// AWS service where the key was last used (e.g. "s3"), or None if never used.
    pub last_used_service: Option<String>,
}

impl
    From<(
        aws_sdk_iam::types::AccessKeyMetadata,
        Option<aws_sdk_iam::types::AccessKeyLastUsed>,
    )> for IamAccessKey
{
    fn from(
        (key, last_used): (
            aws_sdk_iam::types::AccessKeyMetadata,
            Option<aws_sdk_iam::types::AccessKeyLastUsed>,
        ),
    ) -> Self {
        let last_used_date = last_used
            .as_ref()
            .and_then(|lu| lu.last_used_date())
            .map(|d| d.to_string());
        let last_used_region = last_used.as_ref().map(|lu| lu.region()).and_then(|r| {
            if r == "N/A" {
                None
            } else {
                Some(r.to_string())
            }
        });
        let last_used_service =
            last_used
                .as_ref()
                .map(|lu| lu.service_name())
                .and_then(|s| {
                    if s == "N/A" {
                        None
                    } else {
                        Some(s.to_string())
                    }
                });

        Self {
            access_key_id: key.access_key_id().unwrap_or("").to_string(),
            user_name: key.user_name().unwrap_or("").to_string(),
            status: key
                .status()
                .map(|s| s.as_str().to_string())
                .unwrap_or_default(),
            create_date: key.create_date().map(|d| d.to_string()),
            last_used_date,
            last_used_region,
            last_used_service,
        }
    }
}

/// An inline policy embedded directly in an IAM role.
#[derive(SimpleObject, Clone)]
pub struct IamInlinePolicy {
    pub policy_name: String,
    /// URL-decoded JSON policy document.
    pub document: String,
}

impl From<(String, String)> for IamInlinePolicy {
    fn from((policy_name, encoded_doc): (String, String)) -> Self {
        Self {
            policy_name,
            document: percent_decode(&encoded_doc),
        }
    }
}

/// An MFA device enrolled for an IAM user.
#[derive(SimpleObject, Clone)]
pub struct IamMfaDevice {
    pub user_name: String,
    /// For virtual MFA devices this is the ARN; for hardware tokens it is the serial number.
    pub serial_number: String,
    /// ISO-8601 timestamp when the device was associated with the user.
    pub enable_date: Option<String>,
}

impl From<aws_sdk_iam::types::MfaDevice> for IamMfaDevice {
    fn from(d: aws_sdk_iam::types::MfaDevice) -> Self {
        Self {
            user_name: d.user_name().to_string(),
            serial_number: d.serial_number().to_string(),
            enable_date: Some(d.enable_date().to_string()),
        }
    }
}

/// The account-wide IAM password policy.
/// All boolean fields default to `false` when not configured.
/// `None` for `max_password_age` / `password_reuse_prevention` means
/// the setting is disabled (no expiry / no reuse restriction).
#[derive(SimpleObject, Clone)]
pub struct IamPasswordPolicy {
    /// Minimum number of characters required in passwords.
    pub minimum_password_length: Option<i32>,
    pub require_symbols: bool,
    pub require_numbers: bool,
    pub require_uppercase_characters: bool,
    pub require_lowercase_characters: bool,
    pub allow_users_to_change_password: bool,
    /// True if passwords expire (i.e. `max_password_age` is set).
    pub expire_passwords: bool,
    /// Maximum password age in days before it must be changed.
    pub max_password_age: Option<i32>,
    /// Number of previous passwords users are prevented from reusing.
    pub password_reuse_prevention: Option<i32>,
    /// If true, IAM users cannot set a new password after theirs expires —
    /// an administrator must reset it. Critical for breach containment.
    pub hard_expiry: Option<bool>,
}

impl From<aws_sdk_iam::types::PasswordPolicy> for IamPasswordPolicy {
    fn from(p: aws_sdk_iam::types::PasswordPolicy) -> Self {
        Self {
            minimum_password_length: p.minimum_password_length(),
            require_symbols: p.require_symbols(),
            require_numbers: p.require_numbers(),
            require_uppercase_characters: p.require_uppercase_characters(),
            require_lowercase_characters: p.require_lowercase_characters(),
            allow_users_to_change_password: p.allow_users_to_change_password(),
            expire_passwords: p.expire_passwords(),
            max_password_age: p.max_password_age(),
            password_reuse_prevention: p.password_reuse_prevention(),
            hard_expiry: p.hard_expiry(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_decode_basic() {
        let encoded = "%7B%22Version%22%3A%222012-10-17%22%7D";
        let decoded = percent_decode(encoded);
        assert_eq!(decoded, r#"{"Version":"2012-10-17"}"#);
    }

    #[test]
    fn test_percent_decode_passthrough() {
        let plain = "no encoding here";
        assert_eq!(percent_decode(plain), plain);
    }

    #[test]
    fn test_percent_decode_plus_not_space() {
        // IAM uses %20 for spaces, not +; + should remain as-is
        let s = "a+b%20c";
        assert_eq!(percent_decode(s), "a+b c");
    }

    #[test]
    fn test_iam_inline_policy_from_tuple_decodes_document() {
        let encoded = "%7B%22Version%22%3A%222012-10-17%22%7D".to_string();
        let policy = IamInlinePolicy::from(("AllowS3Read".to_string(), encoded));
        assert_eq!(policy.policy_name, "AllowS3Read");
        assert_eq!(policy.document, r#"{"Version":"2012-10-17"}"#);
    }

    #[test]
    fn test_iam_inline_policy_plain_document() {
        let plain = r#"{"Version":"2012-10-17","Statement":[]}"#.to_string();
        let policy = IamInlinePolicy::from(("EmptyPolicy".to_string(), plain.clone()));
        assert_eq!(policy.policy_name, "EmptyPolicy");
        assert_eq!(policy.document, plain);
    }

    #[test]
    fn test_iam_access_key_from_metadata_never_used() {
        use aws_sdk_iam::types::{AccessKeyMetadata, StatusType};
        let meta = AccessKeyMetadata::builder()
            .access_key_id("AKIAIOSFODNN7EXAMPLE")
            .user_name("alice")
            .status(StatusType::Active)
            .build();
        let key = IamAccessKey::from((meta, None));
        assert_eq!(key.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(key.user_name, "alice");
        assert_eq!(key.status, "Active");
        assert!(key.last_used_date.is_none());
        assert!(key.last_used_region.is_none());
        assert!(key.last_used_service.is_none());
    }

    #[test]
    fn test_iam_access_key_na_region_mapped_to_none() {
        use aws_sdk_iam::types::{AccessKeyLastUsed, AccessKeyMetadata, StatusType};
        let meta = AccessKeyMetadata::builder()
            .access_key_id("AKIAIOSFODNN7EXAMPLE")
            .user_name("bob")
            .status(StatusType::Inactive)
            .build();
        let last_used = AccessKeyLastUsed::builder()
            .region("N/A")
            .service_name("N/A")
            .build()
            .unwrap();
        let key = IamAccessKey::from((meta, Some(last_used)));
        assert_eq!(key.status, "Inactive");
        assert!(key.last_used_region.is_none(), "N/A region should be None");
        assert!(key.last_used_service.is_none(), "N/A service should be None");
    }

    #[test]
    fn test_iam_password_policy_from_defaults() {
        use aws_sdk_iam::types::PasswordPolicy;
        // Builder with no setters → all booleans false, all Option fields None
        let policy = PasswordPolicy::builder().build();
        let gql = IamPasswordPolicy::from(policy);
        assert!(!gql.require_symbols);
        assert!(!gql.require_numbers);
        assert!(!gql.require_uppercase_characters);
        assert!(!gql.require_lowercase_characters);
        assert!(!gql.allow_users_to_change_password);
        assert!(!gql.expire_passwords);
        assert!(gql.minimum_password_length.is_none());
        assert!(gql.max_password_age.is_none());
        assert!(gql.password_reuse_prevention.is_none());
        assert!(gql.hard_expiry.is_none());
    }

    #[test]
    fn test_iam_password_policy_from_configured() {
        use aws_sdk_iam::types::PasswordPolicy;
        let policy = PasswordPolicy::builder()
            .minimum_password_length(14)
            .require_symbols(true)
            .require_numbers(true)
            .require_uppercase_characters(true)
            .require_lowercase_characters(true)
            .allow_users_to_change_password(true)
            .expire_passwords(true)
            .max_password_age(90)
            .password_reuse_prevention(24)
            .hard_expiry(false)
            .build();
        let gql = IamPasswordPolicy::from(policy);
        assert_eq!(gql.minimum_password_length, Some(14));
        assert!(gql.require_symbols);
        assert!(gql.require_numbers);
        assert!(gql.require_uppercase_characters);
        assert!(gql.require_lowercase_characters);
        assert!(gql.allow_users_to_change_password);
        assert!(gql.expire_passwords);
        assert_eq!(gql.max_password_age, Some(90));
        assert_eq!(gql.password_reuse_prevention, Some(24));
        assert_eq!(gql.hard_expiry, Some(false));
    }

    #[test]
    fn test_iam_mfa_device_from_sdk_no_enable_date() {
        use aws_sdk_iam::types::MfaDevice;
        let device = MfaDevice::builder()
            .user_name("alice")
            .serial_number("arn:aws:iam::123456789012:mfa/alice")
            .enable_date(aws_sdk_iam::primitives::DateTime::from_secs(1_700_000_000))
            .build()
            .unwrap();
        let gql = IamMfaDevice::from(device);
        assert_eq!(gql.user_name, "alice");
        assert_eq!(gql.serial_number, "arn:aws:iam::123456789012:mfa/alice");
        // `enable_date` is a required field on `MfaDevice`, so it is always present.
        assert!(gql.enable_date.is_some());
    }

    #[test]
    fn test_iam_mfa_device_hardware_serial() {
        use aws_sdk_iam::types::MfaDevice;
        let device = MfaDevice::builder()
            .user_name("bob")
            .serial_number("GAHT12345678")
            .enable_date(aws_sdk_iam::primitives::DateTime::from_secs(1_700_000_000))
            .build()
            .unwrap();
        let gql = IamMfaDevice::from(device);
        assert_eq!(gql.user_name, "bob");
        assert_eq!(gql.serial_number, "GAHT12345678");
    }

    #[test]
    fn test_iam_policy_document_from_tuple() {
        use aws_sdk_iam::types::PolicyVersion;
        let version = PolicyVersion::builder()
            .version_id("v3")
            .is_default_version(true)
            .document("%7B%22Version%22%3A%222012-10-17%22%7D")
            .build();
        let doc = IamPolicyDocument::from((
            "arn:aws:iam::123456789012:policy/MyPolicy".to_string(),
            version,
        ));
        assert_eq!(doc.policy_arn, "arn:aws:iam::123456789012:policy/MyPolicy");
        assert_eq!(doc.version_id, "v3");
        assert!(doc.is_default_version);
        assert_eq!(doc.document, r#"{"Version":"2012-10-17"}"#);
        assert!(doc.create_date.is_none());
    }
}
