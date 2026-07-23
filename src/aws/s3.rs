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

    /// Lists buckets, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `limit` is handed to AWS via
    /// `ListBucketsInput::max_buckets` (this operation's `max_results`-
    /// equivalent) so a capped page boundary lands exactly on the returned
    /// token, matching `specs/plan-2-schema-v2-pagination-timestamps.md`'s
    /// client-layer pattern.
    pub async fn list_buckets(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Bucket>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_buckets();
            if let Some(ref t) = token {
                req = req.continuation_token(t);
            }
            if let Some(l) = limit {
                req = req.max_buckets(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.buckets().to_vec());
            token = output.continuation_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
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
            .map_err(crate::error::sdk_err)?;
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
            .map_err(crate::error::sdk_err)?;
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
                    Err(VaporError::AwsSdk {
                        code: svc_err.code().map(String::from),
                        message: svc_err
                            .message()
                            .map(String::from)
                            .unwrap_or_else(|| svc_err.to_string()),
                    })
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
                    Err(VaporError::AwsSdk {
                        code: svc_err.code().map(String::from),
                        message: svc_err
                            .message()
                            .map(String::from)
                            .unwrap_or_else(|| svc_err.to_string()),
                    })
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
                    Err(VaporError::AwsSdk {
                        code: svc_err.code().map(String::from),
                        message: svc_err
                            .message()
                            .map(String::from)
                            .unwrap_or_else(|| svc_err.to_string()),
                    })
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
                    Err(VaporError::AwsSdk {
                        code: svc_err.code().map(String::from),
                        message: svc_err
                            .message()
                            .map(String::from)
                            .unwrap_or_else(|| svc_err.to_string()),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_response, request, s3_error_response, sdk_config, xml_response, ReplayEvent,
        StaticReplayClient,
    };

    const LIST_BASE: &str = "https://s3.us-east-1.amazonaws.com";
    const BUCKET_BASE: &str = "https://my-bucket.s3.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_buckets_returns_items_single_page() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{LIST_BASE}/?x-id=ListBuckets"), ""),
            xml_response(
                200,
                r#"<ListAllMyBucketsResult><Buckets><Bucket><Name>bucket-a</Name><CreationDate>2023-01-15T00:00:00Z</CreationDate><BucketRegion>us-east-1</BucketRegion></Bucket><Bucket><Name>bucket-b</Name><CreationDate>2023-02-20T00:00:00Z</CreationDate><BucketRegion>us-west-2</BucketRegion></Bucket></Buckets></ListAllMyBucketsResult>"#,
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let (buckets, token) = client.list_buckets(None, None).await.unwrap();

        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].name(), Some("bucket-a"));
        assert_eq!(buckets[1].name(), Some("bucket-b"));
        assert_eq!(buckets[1].bucket_region(), Some("us-west-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_buckets_caps_at_limit_without_second_request() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{LIST_BASE}/?x-id=ListBuckets&max-buckets=1"), ""),
            xml_response(
                200,
                r#"<ListAllMyBucketsResult><Buckets><Bucket><Name>bucket-a</Name><CreationDate>2023-01-15T00:00:00Z</CreationDate><BucketRegion>us-east-1</BucketRegion></Bucket></Buckets><ContinuationToken>cursor-1</ContinuationToken></ListAllMyBucketsResult>"#,
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let (buckets, token) = client.list_buckets(Some(1), None).await.unwrap();

        assert_eq!(buckets.len(), 1);
        assert_eq!(token, Some("cursor-1".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_buckets_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{LIST_BASE}/?x-id=ListBuckets&continuation-token=cursor-x"),
                "",
            ),
            xml_response(
                200,
                r#"<ListAllMyBucketsResult><Buckets><Bucket><Name>bucket-c</Name><CreationDate>2023-03-01T00:00:00Z</CreationDate><BucketRegion>eu-west-1</BucketRegion></Bucket></Buckets></ListAllMyBucketsResult>"#,
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let (buckets, token) = client
            .list_buckets(None, Some("cursor-x".to_string()))
            .await
            .unwrap();

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].name(), Some("bucket-c"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_buckets_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{LIST_BASE}/?x-id=ListBuckets"), ""),
            s3_error_response(400, "AccessDenied", "not authorized"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let err = client.list_buckets(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("AccessDenied".to_string())),
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_bucket_location_returns_region() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?location"), ""),
            xml_response(200, "<LocationConstraint>eu-west-1</LocationConstraint>"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let region = client.get_bucket_location("my-bucket").await.unwrap();

        assert_eq!(region, Some("eu-west-1".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_location_normalizes_empty_constraint_to_us_east_1() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?location"), ""),
            xml_response(200, "<LocationConstraint></LocationConstraint>"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let region = client.get_bucket_location("my-bucket").await.unwrap();

        assert_eq!(region, Some("us-east-1".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_location_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?location"), ""),
            s3_error_response(404, "NoSuchBucket", "bucket does not exist"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let err = client.get_bucket_location("my-bucket").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("NoSuchBucket".to_string())),
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_bucket_versioning_returns_status() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?versioning"), ""),
            xml_response(
                200,
                "<VersioningConfiguration><Status>Enabled</Status></VersioningConfiguration>",
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let status = client.get_bucket_versioning("my-bucket").await.unwrap();

        assert_eq!(status, Some("Enabled".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_versioning_returns_none_when_never_enabled() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?versioning"), ""),
            xml_response(200, "<VersioningConfiguration></VersioningConfiguration>"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let status = client.get_bucket_versioning("my-bucket").await.unwrap();

        assert_eq!(status, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_versioning_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?versioning"), ""),
            s3_error_response(400, "AccessDenied", "not authorized"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let err = client
            .get_bucket_versioning("my-bucket")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("AccessDenied".to_string())),
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_bucket_tagging_returns_tags() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?tagging"), ""),
            xml_response(
                200,
                "<Tagging><TagSet><Tag><Key>env</Key><Value>prod</Value></Tag><Tag><Key>team</Key><Value>platform</Value></Tag></TagSet></Tagging>",
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let tags = client.get_bucket_tagging("my-bucket").await.unwrap();

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].key(), "env");
        assert_eq!(tags[0].value(), "prod");
        assert_eq!(tags[1].key(), "team");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_tagging_returns_empty_on_no_such_tag_set() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?tagging"), ""),
            s3_error_response(404, "NoSuchTagSet", "The TagSet does not exist"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let tags = client.get_bucket_tagging("my-bucket").await.unwrap();

        assert_eq!(tags, Vec::new());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_tagging_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?tagging"), ""),
            s3_error_response(400, "AccessDenied", "not authorized"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let err = client.get_bucket_tagging("my-bucket").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("AccessDenied".to_string())),
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_public_access_block_returns_config() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?publicAccessBlock"), ""),
            xml_response(
                200,
                "<PublicAccessBlockConfiguration><BlockPublicAcls>true</BlockPublicAcls><IgnorePublicAcls>true</IgnorePublicAcls><BlockPublicPolicy>false</BlockPublicPolicy><RestrictPublicBuckets>false</RestrictPublicBuckets></PublicAccessBlockConfiguration>",
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let config = client
            .get_public_access_block("my-bucket")
            .await
            .unwrap()
            .expect("config present");

        assert_eq!(config.block_public_acls(), Some(true));
        assert_eq!(config.ignore_public_acls(), Some(true));
        assert_eq!(config.block_public_policy(), Some(false));
        assert_eq!(config.restrict_public_buckets(), Some(false));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_public_access_block_returns_none_on_no_such_configuration() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?publicAccessBlock"), ""),
            s3_error_response(
                404,
                "NoSuchPublicAccessBlockConfiguration",
                "no configuration set",
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let config = client.get_public_access_block("my-bucket").await.unwrap();

        assert_eq!(config, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_public_access_block_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?publicAccessBlock"), ""),
            s3_error_response(400, "AccessDenied", "not authorized"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let err = client
            .get_public_access_block("my-bucket")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("AccessDenied".to_string())),
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_bucket_policy_returns_policy() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?policy"), ""),
            json_response(200, r#"{"Version":"2012-10-17","Statement":[]}"#),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let policy = client.get_bucket_policy("my-bucket").await.unwrap();

        assert_eq!(
            policy,
            Some(r#"{"Version":"2012-10-17","Statement":[]}"#.to_string())
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_policy_returns_none_on_no_such_bucket_policy() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?policy"), ""),
            s3_error_response(404, "NoSuchBucketPolicy", "no policy set"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let policy = client.get_bucket_policy("my-bucket").await.unwrap();

        assert_eq!(policy, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_policy_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?policy"), ""),
            s3_error_response(400, "AccessDenied", "not authorized"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let err = client.get_bucket_policy("my-bucket").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("AccessDenied".to_string())),
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_bucket_encryption_returns_algorithm() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?encryption"), ""),
            xml_response(
                200,
                "<ServerSideEncryptionConfiguration><Rule><ApplyServerSideEncryptionByDefault><SSEAlgorithm>AES256</SSEAlgorithm></ApplyServerSideEncryptionByDefault></Rule></ServerSideEncryptionConfiguration>",
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let algorithm = client.get_bucket_encryption("my-bucket").await.unwrap();

        assert_eq!(algorithm, Some("AES256".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_encryption_returns_none_on_not_found_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?encryption"), ""),
            s3_error_response(
                404,
                "ServerSideEncryptionConfigurationNotFoundError",
                "no encryption configuration",
            ),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let algorithm = client.get_bucket_encryption("my-bucket").await.unwrap();

        assert_eq!(algorithm, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_bucket_encryption_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?encryption"), ""),
            s3_error_response(400, "AccessDenied", "not authorized"),
        )]);
        let client = S3Client::new(&sdk_config(http_client.clone()));

        let err = client
            .get_bucket_encryption("my-bucket")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("AccessDenied".to_string())),
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
    }
}
