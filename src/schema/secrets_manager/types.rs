use async_graphql::SimpleObject;
use base64::{Engine as _, engine::general_purpose};

use crate::schema::common::types::Tag;

/// An AWS Secrets Manager secret with its metadata.
#[derive(SimpleObject, Clone)]
pub struct Secret {
    pub arn: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub kms_key_id: Option<String>,
    pub rotation_enabled: Option<bool>,
    pub rotation_lambda_arn: Option<String>,
    pub last_rotated_date: Option<String>,
    pub last_changed_date: Option<String>,
    pub last_accessed_date: Option<String>,
    pub next_rotation_date: Option<String>,
    pub primary_region: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_secretsmanager::types::SecretListEntry> for Secret {
    fn from(e: aws_sdk_secretsmanager::types::SecretListEntry) -> Self {
        Self {
            arn: e.arn().map(|s| s.to_string()),
            name: e.name().map(|s| s.to_string()),
            description: e.description().map(|s| s.to_string()),
            kms_key_id: e.kms_key_id().map(|s| s.to_string()),
            rotation_enabled: e.rotation_enabled(),
            rotation_lambda_arn: e.rotation_lambda_arn().map(|s| s.to_string()),
            last_rotated_date: e.last_rotated_date().map(|d| d.to_string()),
            last_changed_date: e.last_changed_date().map(|d| d.to_string()),
            last_accessed_date: e.last_accessed_date().map(|d| d.to_string()),
            next_rotation_date: e.next_rotation_date().map(|d| d.to_string()),
            primary_region: e.primary_region().map(|s| s.to_string()),
            tags: e
                .tags()
                .iter()
                .map(|t| Tag {
                    key: t.key().unwrap_or("").to_string(),
                    value: t.value().unwrap_or("").to_string(),
                })
                .collect(),
        }
    }
}

impl From<aws_sdk_secretsmanager::operation::describe_secret::DescribeSecretOutput> for Secret {
    fn from(
        o: aws_sdk_secretsmanager::operation::describe_secret::DescribeSecretOutput,
    ) -> Self {
        Self {
            arn: o.arn().map(|s| s.to_string()),
            name: o.name().map(|s| s.to_string()),
            description: o.description().map(|s| s.to_string()),
            kms_key_id: o.kms_key_id().map(|s| s.to_string()),
            rotation_enabled: o.rotation_enabled(),
            rotation_lambda_arn: o.rotation_lambda_arn().map(|s| s.to_string()),
            last_rotated_date: o.last_rotated_date().map(|d| d.to_string()),
            last_changed_date: o.last_changed_date().map(|d| d.to_string()),
            last_accessed_date: o.last_accessed_date().map(|d| d.to_string()),
            next_rotation_date: o.next_rotation_date().map(|d| d.to_string()),
            primary_region: o.primary_region().map(|s| s.to_string()),
            tags: o
                .tags()
                .iter()
                .map(|t| Tag {
                    key: t.key().unwrap_or("").to_string(),
                    value: t.value().unwrap_or("").to_string(),
                })
                .collect(),
        }
    }
}

/// The value of an AWS Secrets Manager secret.
#[derive(SimpleObject, Clone)]
pub struct SecretValue {
    pub arn: Option<String>,
    pub name: Option<String>,
    pub version_id: Option<String>,
    pub secret_string: Option<String>,
    /// Base64-encoded binary secret value, if present.
    pub secret_binary: Option<String>,
    pub version_stages: Vec<String>,
    pub created_date: Option<String>,
}

impl From<aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueOutput>
    for SecretValue
{
    fn from(
        o: aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueOutput,
    ) -> Self {
        let secret_binary = o
            .secret_binary()
            .map(|b| general_purpose::STANDARD.encode(b.as_ref()));

        Self {
            arn: o.arn().map(|s| s.to_string()),
            name: o.name().map(|s| s.to_string()),
            version_id: o.version_id().map(|s| s.to_string()),
            secret_string: o.secret_string().map(|s| s.to_string()),
            secret_binary,
            version_stages: o
                .version_stages()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            created_date: o.created_date().map(|d| d.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_from_list_entry_minimal() {
        let entry = aws_sdk_secretsmanager::types::SecretListEntry::builder().build();
        let secret = Secret::from(entry);
        assert_eq!(secret.arn, None);
        assert_eq!(secret.name, None);
        assert_eq!(secret.description, None);
        assert_eq!(secret.kms_key_id, None);
        assert_eq!(secret.rotation_enabled, None);
        assert_eq!(secret.rotation_lambda_arn, None);
        assert_eq!(secret.last_rotated_date, None);
        assert_eq!(secret.last_changed_date, None);
        assert_eq!(secret.last_accessed_date, None);
        assert_eq!(secret.next_rotation_date, None);
        assert_eq!(secret.primary_region, None);
        assert!(secret.tags.is_empty());
    }

    #[test]
    fn test_secret_from_list_entry_full() {
        let entry = aws_sdk_secretsmanager::types::SecretListEntry::builder()
            .arn("arn:aws:secretsmanager:us-east-1:123456789012:secret:my-secret-abc123")
            .name("my-secret")
            .description("A test secret")
            .kms_key_id("alias/aws/secretsmanager")
            .rotation_enabled(true)
            .rotation_lambda_arn("arn:aws:lambda:us-east-1:123:function:rotate")
            .primary_region("us-east-1")
            .build();
        let secret = Secret::from(entry);
        assert_eq!(
            secret.arn,
            Some("arn:aws:secretsmanager:us-east-1:123456789012:secret:my-secret-abc123".to_string())
        );
        assert_eq!(secret.name, Some("my-secret".to_string()));
        assert_eq!(secret.description, Some("A test secret".to_string()));
        assert_eq!(
            secret.kms_key_id,
            Some("alias/aws/secretsmanager".to_string())
        );
        assert_eq!(secret.rotation_enabled, Some(true));
        assert_eq!(
            secret.rotation_lambda_arn,
            Some("arn:aws:lambda:us-east-1:123:function:rotate".to_string())
        );
        assert_eq!(secret.primary_region, Some("us-east-1".to_string()));
    }

    #[test]
    fn test_secret_tags_conversion() {
        let tag = aws_sdk_secretsmanager::types::Tag::builder()
            .key("env")
            .value("prod")
            .build();
        let entry = aws_sdk_secretsmanager::types::SecretListEntry::builder()
            .tags(tag)
            .build();
        let secret = Secret::from(entry);
        assert_eq!(secret.tags.len(), 1);
        assert_eq!(secret.tags[0].key, "env");
        assert_eq!(secret.tags[0].value, "prod");
    }

    #[test]
    fn test_secret_value_from_output_string() {
        let output =
            aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueOutput::builder()
                .arn("arn:aws:secretsmanager:us-east-1:123:secret:my-secret-abc123")
                .name("my-secret")
                .version_id("version-1")
                .secret_string(r#"{"password":"hunter2"}"#)
                .version_stages("AWSCURRENT")
                .build();
        let sv = SecretValue::from(output);
        assert_eq!(
            sv.arn,
            Some("arn:aws:secretsmanager:us-east-1:123:secret:my-secret-abc123".to_string())
        );
        assert_eq!(sv.name, Some("my-secret".to_string()));
        assert_eq!(sv.version_id, Some("version-1".to_string()));
        assert_eq!(
            sv.secret_string,
            Some(r#"{"password":"hunter2"}"#.to_string())
        );
        assert_eq!(sv.secret_binary, None);
        assert_eq!(sv.version_stages, vec!["AWSCURRENT".to_string()]);
    }

    #[test]
    fn test_secret_value_binary_is_base64_encoded() {
        let bytes = b"binary\x00data";
        let blob = aws_sdk_secretsmanager::primitives::Blob::new(bytes.as_ref());
        let output =
            aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueOutput::builder()
                .arn("arn:aws:secretsmanager:us-east-1:123:secret:bin-secret-abc123")
                .name("bin-secret")
                .secret_binary(blob)
                .build();
        let sv = SecretValue::from(output);
        assert_eq!(sv.secret_string, None);
        let expected = general_purpose::STANDARD.encode(bytes);
        assert_eq!(sv.secret_binary, Some(expected));
    }

    #[test]
    fn test_secret_value_empty() {
        let output =
            aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueOutput::builder()
                .build();
        let sv = SecretValue::from(output);
        assert_eq!(sv.arn, None);
        assert_eq!(sv.name, None);
        assert_eq!(sv.version_id, None);
        assert_eq!(sv.secret_string, None);
        assert_eq!(sv.secret_binary, None);
        assert!(sv.version_stages.is_empty());
        assert_eq!(sv.created_date, None);
    }
}
