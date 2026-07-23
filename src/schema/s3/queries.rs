use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::s3::S3Client;
use crate::schema::common::types::Tag;
use crate::schema::pagination::Page;
use crate::schema::s3::types::{S3Bucket, S3PublicAccessBlock};
use crate::schema::time::to_utc;

#[derive(Default)]
pub struct S3Query;

#[Object]
impl S3Query {
    /// Lists S3 buckets with location, versioning, encryption, public access block,
    /// and tags fetched concurrently per bucket, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. Object listing is excluded to
    /// prevent data exposure and unbounded result sizes.
    async fn s3_buckets(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<S3Bucket>> {
        let s3 = ctx.data::<S3Client>()?;
        let (buckets, next_token) = s3.list_buckets(limit, next_token).await?;

        let futures: Vec<_> = buckets
            .iter()
            .map(|b| {
                let name = b.name().unwrap_or("").to_string();
                let creation_date = to_utc(b.creation_date());
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

        Ok(Page {
            items: join_all(futures).await,
            next_token,
        })
    }

    /// Fetch a single S3 bucket by name. Uses list_buckets (full drain, since a
    /// bucket name lookup has no dedicated describe-by-name API) to verify
    /// existence and get creation_date, then fetches location, versioning,
    /// encryption, public access block, and tags in parallel.
    async fn s3_bucket(&self, ctx: &Context<'_>, name: String) -> Result<Option<S3Bucket>> {
        let s3 = ctx.data::<S3Client>()?;
        let (buckets, _) = s3.list_buckets(None, None).await?;
        let bucket = match buckets.iter().find(|b| b.name() == Some(name.as_str())) {
            Some(b) => b,
            None => return Ok(None),
        };
        let creation_date = to_utc(bucket.creation_date());
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

#[cfg(test)]
mod tests {
    use crate::aws::s3::S3Client;
    use crate::aws::test_util::{request, s3_error_response, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::S3Query;

    const LIST_BASE: &str = "https://s3.us-east-1.amazonaws.com";
    const BUCKET_BASE: &str = "https://my-bucket.s3.us-east-1.amazonaws.com";

    /// Per-bucket fan-out order from `tokio::join!` in the resolver: location,
    /// versioning, tagging, public access block, encryption (deterministic
    /// under `StaticReplayClient` — memory gotcha on fan-out ordering).
    fn full_fan_out_events() -> Vec<ReplayEvent> {
        vec![
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?location"), ""),
                xml_response(200, "<LocationConstraint>eu-west-1</LocationConstraint>"),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?versioning"), ""),
                xml_response(
                    200,
                    "<VersioningConfiguration><Status>Enabled</Status></VersioningConfiguration>",
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?tagging"), ""),
                xml_response(
                    200,
                    "<Tagging><TagSet><Tag><Key>env</Key><Value>prod</Value></Tag></TagSet></Tagging>",
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?publicAccessBlock"), ""),
                xml_response(
                    200,
                    "<PublicAccessBlockConfiguration><BlockPublicAcls>true</BlockPublicAcls><IgnorePublicAcls>true</IgnorePublicAcls><BlockPublicPolicy>false</BlockPublicPolicy><RestrictPublicBuckets>false</RestrictPublicBuckets></PublicAccessBlockConfiguration>",
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?encryption"), ""),
                xml_response(
                    200,
                    "<ServerSideEncryptionConfiguration><Rule><ApplyServerSideEncryptionByDefault><SSEAlgorithm>AES256</SSEAlgorithm></ApplyServerSideEncryptionByDefault></Rule></ServerSideEncryptionConfiguration>",
                ),
            ),
        ]
    }

    #[tokio::test]
    async fn s3_buckets_maps_full_fan_out_detail() {
        let mut events = vec![ReplayEvent::new(
            request(&format!("{LIST_BASE}/?x-id=ListBuckets"), ""),
            xml_response(
                200,
                r#"<ListAllMyBucketsResult><Buckets><Bucket><Name>my-bucket</Name><CreationDate>2023-01-15T00:00:00Z</CreationDate></Bucket></Buckets></ListAllMyBucketsResult>"#,
            ),
        )];
        events.extend(full_fan_out_events());
        let http_client = StaticReplayClient::new(events);
        let schema = build_query_schema(S3Query)
            .data(S3Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ s3Buckets { items { name creationDate region versioning encryption \
                 publicAccessBlock { blockPublicAcls ignorePublicAcls blockPublicPolicy \
                 restrictPublicBuckets } tags { key value } } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["s3Buckets"]["items"][0];
        assert_eq!(item["name"], "my-bucket");
        assert_eq!(item["region"], "eu-west-1");
        assert_eq!(item["versioning"], "Enabled");
        assert_eq!(item["encryption"], "AES256");
        assert_eq!(item["publicAccessBlock"]["blockPublicAcls"], true);
        assert_eq!(item["publicAccessBlock"]["restrictPublicBuckets"], false);
        assert_eq!(item["tags"][0]["key"], "env");
        assert_eq!(item["tags"][0]["value"], "prod");
        assert!(json["s3Buckets"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn s3_buckets_swallows_individual_fetch_errors() {
        // Every per-bucket fetch in the resolver uses `unwrap_or(None)` /
        // `unwrap_or_default()` on its `Result` — an error on any of the 5
        // fanned-out calls degrades that field to null/empty rather than
        // failing the whole query.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{LIST_BASE}/?x-id=ListBuckets"), ""),
                xml_response(
                    200,
                    r#"<ListAllMyBucketsResult><Buckets><Bucket><Name>my-bucket</Name><CreationDate>2023-01-15T00:00:00Z</CreationDate></Bucket></Buckets></ListAllMyBucketsResult>"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?location"), ""),
                s3_error_response(403, "AccessDenied", "not authorized"),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?versioning"), ""),
                s3_error_response(403, "AccessDenied", "not authorized"),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?tagging"), ""),
                s3_error_response(404, "NoSuchTagSet", "no tags"),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?publicAccessBlock"), ""),
                s3_error_response(
                    404,
                    "NoSuchPublicAccessBlockConfiguration",
                    "no configuration set",
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BUCKET_BASE}/?encryption"), ""),
                s3_error_response(
                    404,
                    "ServerSideEncryptionConfigurationNotFoundError",
                    "no encryption configuration",
                ),
            ),
        ]);
        let schema = build_query_schema(S3Query)
            .data(S3Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ s3Buckets { items { name region versioning encryption \
                 publicAccessBlock { blockPublicAcls } tags { key } } } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["s3Buckets"]["items"][0];
        assert_eq!(item["name"], "my-bucket");
        assert!(item["region"].is_null());
        assert!(item["versioning"].is_null());
        assert!(item["encryption"].is_null());
        assert!(item["publicAccessBlock"].is_null());
        assert_eq!(item["tags"].as_array().unwrap().len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn s3_buckets_forwards_limit_and_next_token_to_discovery_call_with_no_fan_out() {
        // Zero items with no `ContinuationToken` in the response so
        // `list_buckets`'s own pagination loop breaks after one request
        // (memory gotcha 29: a `NextToken`-bearing zero/under-limit response
        // would otherwise trigger a second discovery request here).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{LIST_BASE}/?x-id=ListBuckets&max-buckets=1&continuation-token=cursor-a"),
                "",
            ),
            xml_response(200, "<ListAllMyBucketsResult><Buckets></Buckets></ListAllMyBucketsResult>"),
        )]);
        let schema = build_query_schema(S3Query)
            .data(S3Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ s3Buckets(limit: 1, nextToken: "cursor-a") { items { name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["s3Buckets"]["items"].as_array().unwrap().len(), 0);
        assert!(json["s3Buckets"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn s3_bucket_returns_full_detail_when_found() {
        let mut events = vec![ReplayEvent::new(
            request(&format!("{LIST_BASE}/?x-id=ListBuckets"), ""),
            xml_response(
                200,
                r#"<ListAllMyBucketsResult><Buckets><Bucket><Name>my-bucket</Name><CreationDate>2023-01-15T00:00:00Z</CreationDate></Bucket></Buckets></ListAllMyBucketsResult>"#,
            ),
        )];
        events.extend(full_fan_out_events());
        let http_client = StaticReplayClient::new(events);
        let schema = build_query_schema(S3Query)
            .data(S3Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ s3Bucket(name: "my-bucket") { name region versioning encryption } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["s3Bucket"]["name"], "my-bucket");
        assert_eq!(json["s3Bucket"]["region"], "eu-west-1");
        assert_eq!(json["s3Bucket"]["versioning"], "Enabled");
        assert_eq!(json["s3Bucket"]["encryption"], "AES256");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn s3_bucket_returns_null_when_not_found_with_no_fan_out_calls() {
        // Only the discovery `ListBuckets` event is queued — if the resolver
        // fetched detail for a non-matching name, `StaticReplayClient` would
        // fail with "no more test data available".
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{LIST_BASE}/?x-id=ListBuckets"), ""),
            xml_response(
                200,
                r#"<ListAllMyBucketsResult><Buckets><Bucket><Name>other-bucket</Name></Bucket></Buckets></ListAllMyBucketsResult>"#,
            ),
        )]);
        let schema = build_query_schema(S3Query)
            .data(S3Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ s3Bucket(name: "missing-bucket") { name } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["s3Bucket"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn s3_bucket_policy_returns_policy_string() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?policy"), ""),
            crate::aws::test_util::json_response(200, r#"{"Version":"2012-10-17","Statement":[]}"#),
        )]);
        let schema = build_query_schema(S3Query)
            .data(S3Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ s3BucketPolicy(name: "my-bucket") }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["s3BucketPolicy"], r#"{"Version":"2012-10-17","Statement":[]}"#);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn s3_bucket_policy_returns_null_when_no_policy_attached() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BUCKET_BASE}/?policy"), ""),
            s3_error_response(404, "NoSuchBucketPolicy", "no policy set"),
        )]);
        let schema = build_query_schema(S3Query)
            .data(S3Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ s3BucketPolicy(name: "my-bucket") }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["s3BucketPolicy"].is_null());
        http_client.relaxed_requests_match();
    }
}
