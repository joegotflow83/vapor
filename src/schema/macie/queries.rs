use async_graphql::{Context, Object, Result};

use crate::aws::macie::MacieClient;
use crate::schema::macie::types::{MacieBucketSummary, MacieFinding};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct MacieQuery;

#[Object]
impl MacieQuery {
    /// `limit` caps the finding-ID list (one page of `list_findings`) before
    /// the `get_findings` detail fan-out, avoiding unnecessary batch-fetch
    /// calls (default unlimited).
    async fn macie_findings(
        &self,
        ctx: &Context<'_>,
        severity: Option<String>,
        finding_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MacieFinding>> {
        let client = ctx.data::<MacieClient>()?;
        let (ids, token) = client
            .list_findings(severity.as_deref(), finding_type.as_deref(), limit, next_token)
            .await?;
        let findings = client.get_findings(ids).await?;
        Ok(Page {
            items: findings.into_iter().map(MacieFinding::from).collect(),
            next_token: token,
        })
    }

    /// `limit` caps the number of buckets returned per page (default
    /// unlimited).
    async fn macie_bucket_summaries(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MacieBucketSummary>> {
        let client = ctx.data::<MacieClient>()?;
        let (buckets, token) = client.describe_buckets(limit, next_token).await?;
        Ok(Page {
            items: buckets.into_iter().map(MacieBucketSummary::from).collect(),
            next_token: token,
        })
    }
}

// `macie_findings` has real logic beyond a bare passthrough: a
// `list_findings` → `get_findings` fan-out (lambda/cloudfront fan-out
// precedent), but unlike lambda's per-item `list_tags` swallow, the
// `get_findings` call uses `?` so a failure propagates as a GraphQL error
// rather than being silently swallowed — earns a dedicated test.
// `macie_bucket_summaries` is a 1:1 passthrough to the already-tested
// `MacieClient::describe_buckets` (see `src/aws/macie.rs`'s own test
// module) and gets one light smoke test (connect/codeartifact precedent).
#[cfg(test)]
mod tests {
    use crate::aws::macie::MacieClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::MacieQuery;

    const BASE: &str = "https://macie2.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn macie_findings_maps_items_and_fans_out_to_get_findings() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/findings"), r#"{"maxResults":1}"#),
                json_response(
                    200,
                    r#"{"findingIds":["f-1"],"nextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/findings/describe"), r#"{"findingIds":["f-1"]}"#),
                json_response(
                    200,
                    r#"{"findings":[{"id":"f-1","title":"Exposed credentials","description":"PII in S3 object","severity":{"description":"High"},"type":"SensitiveData:S3Object/Multiple","category":"CLASSIFICATION","createdAt":"2024-01-01T00:00:00+00:00","updatedAt":"2024-01-02T00:00:00+00:00","archived":false,"resourcesAffected":{"s3Bucket":{"name":"my-bucket"}}}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(MacieQuery)
            .data(MacieClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ macieFindings(limit: 1) { items { id title description severity findingType category resourceType bucketName createdAt updatedAt archived } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["macieFindings"]["items"];
        assert_eq!(items[0]["id"], "f-1");
        assert_eq!(items[0]["title"], "Exposed credentials");
        assert_eq!(items[0]["description"], "PII in S3 object");
        assert_eq!(items[0]["severity"], "High");
        assert_eq!(items[0]["findingType"], "SensitiveData:S3Object/Multiple");
        assert_eq!(items[0]["category"], "CLASSIFICATION");
        assert_eq!(items[0]["resourceType"], "S3Bucket");
        assert_eq!(items[0]["bucketName"], "my-bucket");
        assert_eq!(items[0]["createdAt"], "2024-01-01T00:00:00+00:00");
        assert_eq!(items[0]["updatedAt"], "2024-01-02T00:00:00+00:00");
        assert_eq!(items[0]["archived"], false);
        assert_eq!(json["macieFindings"]["nextToken"], "page2-token");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn macie_findings_skips_get_findings_when_no_ids() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/findings"), "{}"),
            json_response(200, r#"{"findingIds":[]}"#),
        )]);
        let schema = build_query_schema(MacieQuery)
            .data(MacieClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ macieFindings { items { id } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["macieFindings"]["items"].as_array().unwrap().len(), 0);
        assert!(json["macieFindings"]["nextToken"].is_null());
        // Only 1 ReplayEvent was queued (list only) — `get_findings` returns
        // early without a request when `ids` is empty, so if the resolver
        // called it anyway, relaxed_requests_match below would fail with "no
        // more test data available".
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn macie_findings_propagates_get_findings_errors() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/findings"), "{}"),
                json_response(200, r#"{"findingIds":["f-1"]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/findings/describe"), r#"{"findingIds":["f-1"]}"#),
                json_error_response("ResourceNotFoundException", "no such finding"),
            ),
        ]);
        let schema = build_query_schema(MacieQuery)
            .data(MacieClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ macieFindings { items { id } nextToken } }"#)
            .await;

        assert!(!res.errors.is_empty(), "expected an error, got none");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn macie_bucket_summaries_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/datasources/s3"), r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"buckets":[{"bucketName":"my-bucket","accountId":"111122223333","region":"us-east-1","classifiableObjectCount":100,"classifiableSizeInBytes":1048576,"sharedAccess":"NOT_SHARED"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let schema = build_query_schema(MacieQuery)
            .data(MacieClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ macieBucketSummaries(limit: 1) { items { bucketName accountId region classifiableObjectCount classifiableSizeInBytes isPubliclyAccessible sharedAccess errorCode } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["macieBucketSummaries"]["items"];
        assert_eq!(items[0]["bucketName"], "my-bucket");
        assert_eq!(items[0]["accountId"], "111122223333");
        assert_eq!(items[0]["region"], "us-east-1");
        assert_eq!(items[0]["classifiableObjectCount"], 100);
        assert_eq!(items[0]["classifiableSizeInBytes"], 1048576);
        assert_eq!(items[0]["isPubliclyAccessible"], false);
        assert_eq!(items[0]["sharedAccess"], "NOT_SHARED");
        assert!(items[0]["errorCode"].is_null());
        assert_eq!(json["macieBucketSummaries"]["nextToken"], "page2-token");
        http_client.relaxed_requests_match();
    }
}
