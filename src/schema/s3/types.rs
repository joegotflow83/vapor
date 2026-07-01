use async_graphql::SimpleObject;

use crate::schema::common::types::Tag;

/// Public access block configuration for an S3 bucket.
/// All four settings default to false when not configured.
#[derive(SimpleObject, Clone)]
pub struct S3PublicAccessBlock {
    pub block_public_acls: bool,
    pub ignore_public_acls: bool,
    pub block_public_policy: bool,
    pub restrict_public_buckets: bool,
}

/// An S3 bucket with bucket-level metadata. Object listing is intentionally
/// excluded to avoid accidental data exposure and unbounded result sets.
#[derive(SimpleObject)]
pub struct S3Bucket {
    pub name: String,
    pub creation_date: Option<String>,
    /// Normalized bucket region. Always populated — us-east-1 classic buckets
    /// are normalized from the SDK's None response to "us-east-1".
    pub region: Option<String>,
    /// "Enabled" | "Suspended" | null (versioning never enabled).
    pub versioning: Option<String>,
    /// Default SSE algorithm: "AES256", "aws:kms", or null if not configured.
    pub encryption: Option<String>,
    /// Public access block settings, or null if no configuration exists.
    pub public_access_block: Option<S3PublicAccessBlock>,
    pub tags: Vec<Tag>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_bucket_all_fields() {
        let bucket = S3Bucket {
            name: "my-bucket".to_string(),
            creation_date: Some("2024-01-01T00:00:00Z".to_string()),
            region: Some("us-east-1".to_string()),
            versioning: Some("Enabled".to_string()),
            encryption: Some("AES256".to_string()),
            public_access_block: Some(S3PublicAccessBlock {
                block_public_acls: true,
                ignore_public_acls: true,
                block_public_policy: true,
                restrict_public_buckets: true,
            }),
            tags: vec![Tag {
                key: "Environment".to_string(),
                value: "production".to_string(),
            }],
        };
        assert_eq!(bucket.name, "my-bucket");
        assert_eq!(bucket.region, Some("us-east-1".to_string()));
        assert_eq!(bucket.versioning, Some("Enabled".to_string()));
        assert_eq!(bucket.encryption, Some("AES256".to_string()));
        let pab = bucket.public_access_block.unwrap();
        assert!(pab.block_public_acls);
        assert!(pab.restrict_public_buckets);
        assert_eq!(bucket.tags.len(), 1);
        assert_eq!(bucket.tags[0].key, "Environment");
    }

    #[test]
    fn test_s3_bucket_no_security_config() {
        let bucket = S3Bucket {
            name: "unversioned".to_string(),
            creation_date: None,
            region: Some("us-west-2".to_string()),
            versioning: None,
            encryption: None,
            public_access_block: None,
            tags: vec![],
        };
        assert!(bucket.versioning.is_none());
        assert!(bucket.encryption.is_none());
        assert!(bucket.public_access_block.is_none());
        assert!(bucket.creation_date.is_none());
        assert!(bucket.tags.is_empty());
    }

    #[test]
    fn test_s3_bucket_kms_encryption() {
        let bucket = S3Bucket {
            name: "encrypted-bucket".to_string(),
            creation_date: None,
            region: Some("eu-west-1".to_string()),
            versioning: Some("Suspended".to_string()),
            encryption: Some("aws:kms".to_string()),
            public_access_block: Some(S3PublicAccessBlock {
                block_public_acls: true,
                ignore_public_acls: true,
                block_public_policy: false,
                restrict_public_buckets: false,
            }),
            tags: vec![],
        };
        assert_eq!(bucket.encryption, Some("aws:kms".to_string()));
        let pab = bucket.public_access_block.unwrap();
        assert!(pab.block_public_acls);
        assert!(!pab.block_public_policy);
    }
}
