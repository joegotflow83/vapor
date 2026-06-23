use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::s3::S3Client;
use crate::schema::ec2::types::Tag;
use crate::schema::s3::types::{S3Bucket, S3PublicAccessBlock};

#[derive(Default)]
pub struct S3Query;

#[Object]
impl S3Query {
    /// List all S3 buckets with location, versioning, encryption, public access block,
    /// and tags fetched concurrently per bucket. Object listing is excluded to prevent
    /// data exposure and unbounded result sizes.
    async fn s3_buckets(&self, ctx: &Context<'_>) -> Result<Vec<S3Bucket>> {
        let s3 = ctx.data::<S3Client>()?;
        let buckets = s3.list_buckets().await?;

        let futures: Vec<_> = buckets
            .iter()
            .map(|b| {
                let name = b.name().unwrap_or("").to_string();
                let creation_date = b.creation_date().map(|d| d.to_string());
                async move {
                    let (region_res, versioning_res, tags_res, pab_res, enc_res) = tokio::join!(
                        s3.get_bucket_location(&name),
                        s3.get_bucket_versioning(&name),
                        s3.get_bucket_tagging(&name),
                        s3.get_public_access_block(&name),
                        s3.get_bucket_encryption(&name),
                    );
                    let tags = tags_res
                        .unwrap_or_default()
                        .into_iter()
                        .map(|t| Tag {
                            key: t.key().to_string(),
                            value: t.value().to_string(),
                        })
                        .collect();
                    let public_access_block = pab_res.unwrap_or(None).map(|cfg| S3PublicAccessBlock {
                        block_public_acls: cfg.block_public_acls().unwrap_or(false),
                        ignore_public_acls: cfg.ignore_public_acls().unwrap_or(false),
                        block_public_policy: cfg.block_public_policy().unwrap_or(false),
                        restrict_public_buckets: cfg.restrict_public_buckets().unwrap_or(false),
                    });
                    S3Bucket {
                        name,
                        creation_date,
                        region: region_res.unwrap_or(None),
                        versioning: versioning_res.unwrap_or(None),
                        encryption: enc_res.unwrap_or(None),
                        public_access_block,
                        tags,
                    }
                }
            })
            .collect();

        Ok(join_all(futures).await)
    }

    /// Fetch a single S3 bucket by name. Uses list_buckets to verify existence
    /// and get creation_date, then fetches location, versioning, encryption,
    /// public access block, and tags in parallel.
    async fn s3_bucket(&self, ctx: &Context<'_>, name: String) -> Result<Option<S3Bucket>> {
        let s3 = ctx.data::<S3Client>()?;
        let buckets = s3.list_buckets().await?;
        let bucket = match buckets.iter().find(|b| b.name() == Some(name.as_str())) {
            Some(b) => b,
            None => return Ok(None),
        };
        let creation_date = bucket.creation_date().map(|d| d.to_string());
        let (region_res, versioning_res, tags_res, pab_res, enc_res) = tokio::join!(
            s3.get_bucket_location(&name),
            s3.get_bucket_versioning(&name),
            s3.get_bucket_tagging(&name),
            s3.get_public_access_block(&name),
            s3.get_bucket_encryption(&name),
        );
        let tags = tags_res
            .unwrap_or_default()
            .into_iter()
            .map(|t| Tag {
                key: t.key().to_string(),
                value: t.value().to_string(),
            })
            .collect();
        let public_access_block = pab_res.unwrap_or(None).map(|cfg| S3PublicAccessBlock {
            block_public_acls: cfg.block_public_acls().unwrap_or(false),
            ignore_public_acls: cfg.ignore_public_acls().unwrap_or(false),
            block_public_policy: cfg.block_public_policy().unwrap_or(false),
            restrict_public_buckets: cfg.restrict_public_buckets().unwrap_or(false),
        });
        Ok(Some(S3Bucket {
            name,
            creation_date,
            region: region_res.unwrap_or(None),
            versioning: versioning_res.unwrap_or(None),
            encryption: enc_res.unwrap_or(None),
            public_access_block,
            tags,
        }))
    }

    /// Fetch the resource-based policy document for a bucket.
    /// Returns the raw JSON policy string, or null if no policy is attached.
    /// Policy documents reveal cross-account access and public grants not
    /// captured by the public access block settings.
    async fn s3_bucket_policy(&self, ctx: &Context<'_>, name: String) -> Result<Option<String>> {
        let s3 = ctx.data::<S3Client>()?;
        Ok(s3.get_bucket_policy(&name).await?)
    }
}
