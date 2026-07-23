use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

use crate::schema::time::to_utc;

/// A KMS customer master key with full metadata.
#[derive(SimpleObject, Clone)]
pub struct KmsKey {
    pub key_id: String,
    pub arn: Option<String>,
    pub description: Option<String>,
    /// Enabled | Disabled | PendingDeletion | PendingImport | ...
    pub key_state: Option<String>,
    /// ENCRYPT_DECRYPT | SIGN_VERIFY | GENERATE_VERIFY_MAC
    pub key_usage: Option<String>,
    /// SYMMETRIC_DEFAULT | RSA_2048 | ECC_NIST_P256 | ...
    pub key_spec: Option<String>,
    /// AWS_KMS | EXTERNAL | AWS_CLOUDHSM
    pub origin: Option<String>,
    pub multi_region: Option<bool>,
    pub enabled: Option<bool>,
    pub creation_date: Option<DateTime<Utc>>,
    pub deletion_date: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub custom_key_store_id: Option<String>,
    /// Whether automatic annual key rotation is enabled (CIS 3.8).
    /// `None` for keys that don't support rotation (asymmetric, HMAC, AWS-managed, external).
    pub rotation_enabled: Option<bool>,
}

impl KmsKey {
    pub fn from_sdk(
        m: &aws_sdk_kms::types::KeyMetadata,
        rotation_enabled: Option<bool>,
    ) -> Self {
        Self {
            key_id: m.key_id().to_string(),
            arn: m.arn().map(|s| s.to_string()),
            description: m.description().map(|s| s.to_string()),
            key_state: m.key_state().map(|s| s.as_str().to_string()),
            key_usage: m.key_usage().map(|s| s.as_str().to_string()),
            key_spec: m.key_spec().map(|s| s.as_str().to_string()),
            origin: m.origin().map(|s| s.as_str().to_string()),
            multi_region: m.multi_region(),
            enabled: Some(m.enabled()),
            creation_date: to_utc(m.creation_date()),
            deletion_date: to_utc(m.deletion_date()),
            valid_to: to_utc(m.valid_to()),
            custom_key_store_id: m.custom_key_store_id().map(|s| s.to_string()),
            rotation_enabled,
        }
    }
}

impl From<aws_sdk_kms::types::KeyMetadata> for KmsKey {
    fn from(m: aws_sdk_kms::types::KeyMetadata) -> Self {
        Self::from_sdk(&m, None)
    }
}

/// A KMS alias entry.
#[derive(SimpleObject, Clone)]
pub struct KmsAlias {
    pub alias_name: Option<String>,
    pub alias_arn: Option<String>,
    pub target_key_id: Option<String>,
    pub creation_date: Option<DateTime<Utc>>,
    pub last_updated_date: Option<DateTime<Utc>>,
}

impl From<aws_sdk_kms::types::AliasListEntry> for KmsAlias {
    fn from(a: aws_sdk_kms::types::AliasListEntry) -> Self {
        Self {
            alias_name: a.alias_name().map(|s| s.to_string()),
            alias_arn: a.alias_arn().map(|s| s.to_string()),
            target_key_id: a.target_key_id().map(|s| s.to_string()),
            creation_date: to_utc(a.creation_date()),
            last_updated_date: to_utc(a.last_updated_date()),
        }
    }
}

/// A KMS key policy (raw IAM policy JSON).
#[derive(SimpleObject, Clone)]
pub struct KmsKeyPolicy {
    pub key_id: String,
    pub policy_name: String,
    pub policy: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- KmsKey ---

    #[test]
    fn test_kms_key_from_sdk_full() {
        let sdk = aws_sdk_kms::types::KeyMetadata::builder()
            .key_id("1234abcd-12ab-34cd-56ef-1234567890ab")
            .arn("arn:aws:kms:us-east-1:123456789012:key/1234abcd-12ab-34cd-56ef-1234567890ab")
            .description("My test key")
            .key_state(aws_sdk_kms::types::KeyState::Enabled)
            .key_usage(aws_sdk_kms::types::KeyUsageType::EncryptDecrypt)
            .key_spec(aws_sdk_kms::types::KeySpec::SymmetricDefault)
            .origin(aws_sdk_kms::types::OriginType::AwsKms)
            .multi_region(false)
            .enabled(true)
            .build()
            .expect("required fields provided");

        let key = KmsKey::from(sdk);
        assert_eq!(key.key_id, "1234abcd-12ab-34cd-56ef-1234567890ab");
        assert_eq!(
            key.arn,
            Some("arn:aws:kms:us-east-1:123456789012:key/1234abcd-12ab-34cd-56ef-1234567890ab".to_string())
        );
        assert_eq!(key.description, Some("My test key".to_string()));
        assert_eq!(key.key_state, Some("Enabled".to_string()));
        assert_eq!(key.key_usage, Some("ENCRYPT_DECRYPT".to_string()));
        assert_eq!(key.key_spec, Some("SYMMETRIC_DEFAULT".to_string()));
        assert_eq!(key.origin, Some("AWS_KMS".to_string()));
        assert_eq!(key.multi_region, Some(false));
        assert_eq!(key.enabled, Some(true));
        assert_eq!(key.creation_date, None);
        assert_eq!(key.deletion_date, None);
        assert_eq!(key.valid_to, None);
        assert_eq!(key.custom_key_store_id, None);
        assert_eq!(key.rotation_enabled, None);
    }

    #[test]
    fn test_kms_key_from_sdk_minimal() {
        let sdk = aws_sdk_kms::types::KeyMetadata::builder()
            .key_id("min-key-id")
            .build()
            .expect("key_id provided");

        let key = KmsKey::from(sdk);
        assert_eq!(key.key_id, "min-key-id");
        assert_eq!(key.arn, None);
        assert_eq!(key.description, None);
        assert_eq!(key.key_state, None);
        assert_eq!(key.key_usage, None);
        assert_eq!(key.key_spec, None);
        assert_eq!(key.origin, None);
        assert_eq!(key.multi_region, None);
        // enabled defaults to false when not set
        assert_eq!(key.enabled, Some(false));
        assert_eq!(key.creation_date, None);
        assert_eq!(key.deletion_date, None);
        assert_eq!(key.valid_to, None);
        assert_eq!(key.custom_key_store_id, None);
        // From<KeyMetadata> always sets rotation_enabled = None; enriched via from_sdk
        assert_eq!(key.rotation_enabled, None);
    }

    #[test]
    fn test_kms_key_with_dates() {
        let ts = aws_sdk_kms::primitives::DateTime::from_secs(1_700_000_000);
        let sdk = aws_sdk_kms::types::KeyMetadata::builder()
            .key_id("dated-key-id")
            .creation_date(ts.clone())
            .deletion_date(ts.clone())
            .valid_to(ts)
            .build()
            .expect("key_id provided");

        let key = KmsKey::from(sdk);
        assert_eq!(key.creation_date.unwrap().timestamp(), 1_700_000_000);
        assert_eq!(key.deletion_date.unwrap().timestamp(), 1_700_000_000);
        assert_eq!(key.valid_to.unwrap().timestamp(), 1_700_000_000);
    }

    #[test]
    fn test_kms_key_from_sdk_rotation_enabled() {
        let sdk = aws_sdk_kms::types::KeyMetadata::builder()
            .key_id("sym-key-rotating")
            .key_spec(aws_sdk_kms::types::KeySpec::SymmetricDefault)
            .build()
            .expect("key_id provided");

        let key = KmsKey::from_sdk(&sdk, Some(true));
        assert_eq!(key.key_id, "sym-key-rotating");
        assert_eq!(key.rotation_enabled, Some(true));
    }

    #[test]
    fn test_kms_key_from_sdk_rotation_not_applicable() {
        // Asymmetric keys return None — UnsupportedOperationException handled at AWS layer
        let sdk = aws_sdk_kms::types::KeyMetadata::builder()
            .key_id("asym-key")
            .key_spec(aws_sdk_kms::types::KeySpec::Rsa2048)
            .build()
            .expect("key_id provided");

        let key = KmsKey::from_sdk(&sdk, None);
        assert_eq!(key.key_id, "asym-key");
        assert_eq!(key.rotation_enabled, None);
    }

    // --- KmsAlias ---

    #[test]
    fn test_kms_alias_from_sdk_full() {
        let ts = aws_sdk_kms::primitives::DateTime::from_secs(1_600_000_000);
        let sdk = aws_sdk_kms::types::AliasListEntry::builder()
            .alias_name("alias/my-key")
            .alias_arn("arn:aws:kms:us-east-1:123456789012:alias/my-key")
            .target_key_id("1234abcd-12ab-34cd-56ef-1234567890ab")
            .creation_date(ts.clone())
            .last_updated_date(ts)
            .build();

        let alias = KmsAlias::from(sdk);
        assert_eq!(alias.alias_name, Some("alias/my-key".to_string()));
        assert_eq!(
            alias.alias_arn,
            Some("arn:aws:kms:us-east-1:123456789012:alias/my-key".to_string())
        );
        assert_eq!(
            alias.target_key_id,
            Some("1234abcd-12ab-34cd-56ef-1234567890ab".to_string())
        );
        assert_eq!(alias.creation_date.unwrap().timestamp(), 1_600_000_000);
        assert_eq!(alias.last_updated_date.unwrap().timestamp(), 1_600_000_000);
    }

    #[test]
    fn test_kms_alias_from_sdk_empty() {
        let sdk = aws_sdk_kms::types::AliasListEntry::builder().build();
        let alias = KmsAlias::from(sdk);
        assert_eq!(alias.alias_name, None);
        assert_eq!(alias.alias_arn, None);
        assert_eq!(alias.target_key_id, None);
        assert_eq!(alias.creation_date, None);
        assert_eq!(alias.last_updated_date, None);
    }

    // --- KmsKeyPolicy ---

    #[test]
    fn test_kms_key_policy_fields() {
        let policy = KmsKeyPolicy {
            key_id: "key-123".to_string(),
            policy_name: "default".to_string(),
            policy: r#"{"Version":"2012-10-17"}"#.to_string(),
        };
        assert_eq!(policy.key_id, "key-123");
        assert_eq!(policy.policy_name, "default");
        assert_eq!(policy.policy, r#"{"Version":"2012-10-17"}"#);
    }
}
