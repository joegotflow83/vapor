#[cfg(feature = "s3")]
use aws_config::SdkConfig;
#[cfg(feature = "s3")]
use aws_sdk_s3::types::{Bucket, PublicAccessBlockConfiguration, Tag};
#[cfg(feature = "s3")]
use aws_sdk_s3::error::ProvideErrorMetadata;

#[cfg(feature = "s3")]
use crate::error::VaporError;

#[cfg(feature = "s3")]
pub struct S3Client {
    inner: aws_sdk_s3::Client,
}

impl S3Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_s3::Client::new(config),
        }
    }

    /// List all buckets. No pagination — the API returns all buckets at once.
    pub async fn list_buckets(&self) -> Result<Vec<Bucket>, VaporError> {
        let output = self
            .inner
            .list_buckets()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.buckets().to_vec())
    }

    /// Get the region for a bucket. Normalizes the us-east-1 API quirk where
    /// the SDK returns None for the classic region.
    pub async fn get_bucket_location(&self, bucket: &str) -> Result<Option<String>, VaporError> {
        let output = self
            .inner
            .get_bucket_location()
            .bucket(bucket)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        let region = match output.location_constraint() {
            None => Some("us-east-1".to_string()),
            Some(c) => {
                let s = c.as_str();
                if s.is_empty() {
                    Some("us-east-1".to_string())
                } else {
                    Some(s.to_string())
                }
            }
        };
        Ok(region)
    }

    /// Get versioning status for a bucket. Returns "Enabled", "Suspended", or None
    /// (never enabled).
    pub async fn get_bucket_versioning(&self, bucket: &str) -> Result<Option<String>, VaporError> {
        let output = self
            .inner
            .get_bucket_versioning()
            .bucket(bucket)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.status().map(|s| s.as_str().to_string()))
    }

    /// Get tags for a bucket. NoSuchTagSet is not an error — returns empty Vec.
    pub async fn get_bucket_tagging(&self, bucket: &str) -> Result<Vec<Tag>, VaporError> {
        match self
            .inner
            .get_bucket_tagging()
            .bucket(bucket)
            .send()
            .await
        {
            Ok(output) => Ok(output.tag_set().to_vec()),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.code() == Some("NoSuchTagSet") {
                    Ok(vec![])
                } else {
                    Err(VaporError::AwsSdk(svc_err.to_string()))
                }
            }
        }
    }

    /// Get the public access block configuration for a bucket.
    /// NoSuchPublicAccessBlockConfiguration is not an error — returns None.
    pub async fn get_public_access_block(
        &self,
        bucket: &str,
    ) -> Result<Option<PublicAccessBlockConfiguration>, VaporError> {
        match self
            .inner
            .get_public_access_block()
            .bucket(bucket)
            .send()
            .await
        {
            Ok(output) => Ok(output.public_access_block_configuration().cloned()),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.code() == Some("NoSuchPublicAccessBlockConfiguration") {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(svc_err.to_string()))
                }
            }
        }
    }

    /// Get the bucket policy JSON document.
    /// NoSuchBucketPolicy is not an error — returns None when no resource policy is attached.
    pub async fn get_bucket_policy(&self, bucket: &str) -> Result<Option<String>, VaporError> {
        match self
            .inner
            .get_bucket_policy()
            .bucket(bucket)
            .send()
            .await
        {
            Ok(output) => Ok(output.policy().map(|s| s.to_string())),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.code() == Some("NoSuchBucketPolicy") {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(svc_err.to_string()))
                }
            }
        }
    }

    /// Get the default server-side encryption algorithm for a bucket.
    /// Returns the SSE algorithm string (e.g. "AES256" or "aws:kms"), or None if not configured.
    pub async fn get_bucket_encryption(&self, bucket: &str) -> Result<Option<String>, VaporError> {
        match self
            .inner
            .get_bucket_encryption()
            .bucket(bucket)
            .send()
            .await
        {
            Ok(output) => {
                let algorithm = output
                    .server_side_encryption_configuration()
                    .and_then(|c| c.rules().first())
                    .and_then(|r| r.apply_server_side_encryption_by_default())
                    .map(|d| d.sse_algorithm().as_str().to_string());
                Ok(algorithm)
            }
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.code() == Some("ServerSideEncryptionConfigurationNotFoundError") {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(svc_err.to_string()))
                }
            }
        }
    }
}
