use async_graphql::{Context, Object, Result};

use crate::aws::ecr::EcrClient;
use crate::schema::ecr::types::{EcrImage, EcrImageScanFindings, EcrRepository};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct EcrQuery;

#[Object]
impl EcrQuery {
    /// List ECR repositories. Optionally filter by repository names. `limit` caps the
    /// number of results returned (default unlimited) and resumes from `next_token`.
    async fn ecr_repositories(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EcrRepository>> {
        let ecr = ctx.data::<EcrClient>()?;
        let (repos, token) = ecr.describe_repositories(names, limit, next_token).await?;
        Ok(Page {
            items: repos.into_iter().map(EcrRepository::from).collect(),
            next_token: token,
        })
    }

    /// List images in an ECR repository. Optionally filter by image tags or digests.
    /// `limit` caps the number of results returned (default unlimited) and resumes from
    /// `next_token`.
    async fn ecr_images(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
        image_tags: Option<Vec<String>>,
        image_digests: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EcrImage>> {
        let ecr = ctx.data::<EcrClient>()?;
        let (images, token) = ecr
            .describe_images(&repository_name, image_tags, image_digests, limit, next_token)
            .await?;
        Ok(Page {
            items: images.into_iter().map(EcrImage::from).collect(),
            next_token: token,
        })
    }

    /// Fetch CVE/vulnerability scan findings for a specific image by digest.
    /// Security value: detect vulnerable container images, audit CVE exposure by severity.
    /// Use ecr_images first to get an image digest, then pass it here. `limit` caps the
    /// number of findings returned (default unlimited); the result's `nextToken` resumes
    /// the findings list (unlike a plain list query, this also carries per-image metadata
    /// alongside the findings, so it isn't wrapped in the generic `Page` type).
    async fn ecr_image_scan_findings(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
        image_digest: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<EcrImageScanFindings> {
        let ecr = ctx.data::<EcrClient>()?;
        let (result, token) = ecr
            .describe_image_scan_findings(&repository_name, &image_digest, limit, next_token)
            .await?;
        let mut findings = EcrImageScanFindings::from(result);
        findings.next_token = token;
        Ok(findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::ecr::EcrClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    // Matches `src/aws/ecr.rs`'s own test module: ecr's endpoint is
    // `api.ecr.<region>.amazonaws.com`, not `ecr.<region>...` like most
    // other services.
    const ENDPOINT: &str = "https://api.ecr.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn ecr_repositories_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"repositories":[{"repositoryName":"foo","repositoryUri":"123.dkr.ecr.us-east-1.amazonaws.com/foo"}],"nextToken":"cursor-a"}"#,
            ),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EcrQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ ecrRepositories(limit: 1) { items { repositoryName repositoryUri } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["ecrRepositories"]["items"][0]["repositoryName"], "foo");
        assert_eq!(
            data["ecrRepositories"]["items"][0]["repositoryUri"],
            "123.dkr.ecr.us-east-1.amazonaws.com/foo"
        );
        assert_eq!(data["ecrRepositories"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ecr_images_maps_items_and_forwards_repository_name() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"foo"}"#),
            json_response(
                200,
                r#"{"imageDetails":[{"repositoryName":"foo","imageDigest":"sha256:abc","imageTags":["latest"]}]}"#,
            ),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EcrQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ ecrImages(repositoryName: "foo") { items { imageDigest imageTags } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["ecrImages"]["items"][0]["imageDigest"], "sha256:abc");
        assert_eq!(
            data["ecrImages"]["items"][0]["imageTags"],
            serde_json::json!(["latest"])
        );
        assert_eq!(data["ecrImages"]["nextToken"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ecr_image_scan_findings_maps_findings_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"repositoryName":"foo","imageId":{"imageDigest":"sha256:abc"}}"#,
            ),
            json_response(
                200,
                r#"{"imageId":{"imageDigest":"sha256:abc","imageTag":"latest"},"imageScanStatus":{"status":"COMPLETE"},"imageScanFindings":{"findingSeverityCounts":{"HIGH":2},"findings":[{"name":"CVE-1","severity":"HIGH"}]}}"#,
            ),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EcrQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ ecrImageScanFindings(repositoryName: "foo", imageDigest: "sha256:abc") { scanStatus findings { name severity } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["ecrImageScanFindings"]["scanStatus"], "COMPLETE");
        assert_eq!(data["ecrImageScanFindings"]["findings"][0]["name"], "CVE-1");
        assert_eq!(
            data["ecrImageScanFindings"]["findings"][0]["severity"],
            "HIGH"
        );
        assert_eq!(data["ecrImageScanFindings"]["nextToken"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }
}
