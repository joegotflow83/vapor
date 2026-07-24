#[cfg(feature = "ecr")]
use aws_config::SdkConfig;
#[cfg(feature = "ecr")]
use aws_sdk_ecr::types::{ImageDetail, ImageIdentifier, ImageScanFinding, Repository};

#[cfg(feature = "ecr")]
use crate::error::VaporError;

#[cfg(feature = "ecr")]
/// Aggregated result from a single page of `describe_image_scan_findings`.
/// `image_tags`/`scan_status`/`scan_completed_at`/`finding_severity_counts` are
/// metadata captured from that page's response (AWS repeats them on every page,
/// so this stays correct however many pages are fetched); `findings` is that
/// page's slice, capped/resumed via the returned token like any other list.
#[derive(Debug)]
pub struct ImageScanFindingsResult {
    pub image_tags: Vec<String>,
    pub scan_status: Option<String>,
    pub scan_completed_at: Option<aws_smithy_types::DateTime>,
    pub finding_severity_counts: Vec<(String, i64)>,
    pub findings: Vec<ImageScanFinding>,
}

#[cfg(feature = "ecr")]
pub struct EcrClient {
    inner: aws_sdk_ecr::Client,
}

impl EcrClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ecr::Client::new(config),
        }
    }

    /// Lists ECR repositories, optionally filtered by name, capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    pub async fn describe_repositories(
        &self,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Repository>, Option<String>), VaporError> {
        let mut repos = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_repositories();
            if let Some(ref names_vec) = names {
                for name in names_vec {
                    req = req.repository_names(name);
                }
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - repos.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            repos.extend(output.repositories.unwrap_or_default());
            token = output.next_token.filter(|t| !t.is_empty());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if repos.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((repos, token))
    }

    /// Lists images in an ECR repository, optionally filtered by tag/digest, capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    pub async fn describe_images(
        &self,
        repository_name: &str,
        image_tags: Option<Vec<String>>,
        image_digests: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ImageDetail>, Option<String>), VaporError> {
        let mut images = Vec::new();
        let mut token = next_token;

        loop {
            // Build image identifiers if filters provided (rebuilt each page since
            // `req` is a fresh builder each iteration).
            let mut image_ids: Vec<ImageIdentifier> = Vec::new();
            if let Some(ref tags) = image_tags {
                for tag in tags {
                    image_ids.push(ImageIdentifier::builder().image_tag(tag).build());
                }
            }
            if let Some(ref digests) = image_digests {
                for digest in digests {
                    image_ids.push(ImageIdentifier::builder().image_digest(digest).build());
                }
            }

            let mut req = self
                .inner
                .describe_images()
                .repository_name(repository_name);
            for id in image_ids {
                req = req.image_ids(id);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - images.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            images.extend(output.image_details.unwrap_or_default());
            token = output.next_token.filter(|t| !t.is_empty());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if images.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((images, token))
    }

    /// Fetch image scan findings for a specific image (identified by digest), capped at
    /// `limit` findings (default unlimited) and resumed from `next_token`.
    pub async fn describe_image_scan_findings(
        &self,
        repository_name: &str,
        image_digest: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(ImageScanFindingsResult, Option<String>), VaporError> {
        let mut all_findings: Vec<ImageScanFinding> = Vec::new();
        let mut scan_status: Option<String> = None;
        let mut scan_completed_at: Option<aws_smithy_types::DateTime> = None;
        let mut image_tags: Vec<String> = Vec::new();
        let mut finding_severity_counts: Vec<(String, i64)> = Vec::new();
        let mut counts_collected = false;
        let mut token = next_token;

        loop {
            let image_id = ImageIdentifier::builder()
                .image_digest(image_digest)
                .build();

            let mut req = self
                .inner
                .describe_image_scan_findings()
                .repository_name(repository_name)
                .image_id(image_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - all_findings.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            // Capture metadata only on first page
            if scan_status.is_none() {
                scan_status = output
                    .image_scan_status
                    .as_ref()
                    .and_then(|s| s.status())
                    .map(|s| s.as_str().to_string());
            }
            if let Some(id) = &output.image_id {
                if image_tags.is_empty() {
                    if let Some(tag) = &id.image_tag {
                        image_tags.push(tag.clone());
                    }
                }
            }
            if let Some(findings_obj) = output.image_scan_findings {
                if scan_completed_at.is_none() {
                    scan_completed_at = findings_obj.image_scan_completed_at;
                }
                if !counts_collected {
                    if let Some(counts) = findings_obj.finding_severity_counts {
                        for (severity, count) in counts {
                            finding_severity_counts
                                .push((severity.as_str().to_string(), count.into()));
                        }
                    }
                    counts_collected = true;
                }
                all_findings.extend(findings_obj.findings.unwrap_or_default());
            }

            token = output.next_token.filter(|t| !t.is_empty());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all_findings.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((
            ImageScanFindingsResult {
                image_tags,
                scan_status,
                scan_completed_at,
                finding_severity_counts,
                findings: all_findings,
            },
            token,
        ))
    }
}

#[cfg(test)]
#[cfg(feature = "ecr")]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::error::VaporError;

    // ecr's endpoint is `api.ecr.<region>.amazonaws.com`, not `ecr.<region>...`
    // like most other services (verified against the pinned aws-sdk-ecr
    // crate's `config/endpoint.rs` per gotcha 3 in the sweep notes).
    const ENDPOINT: &str = "https://api.ecr.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_repositories_lists_all_when_no_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"repositories":[{"repositoryName":"foo","repositoryUri":"123.dkr.ecr.us-east-1.amazonaws.com/foo"},{"repositoryName":"bar","repositoryUri":"123.dkr.ecr.us-east-1.amazonaws.com/bar"}]}"#,
            ),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client
            .describe_repositories(None, None, None)
            .await
            .unwrap();

        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].repository_name.as_deref(), Some("foo"));
        assert_eq!(repos[1].repository_name.as_deref(), Some("bar"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_repositories_filters_by_names_and_resumes_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"repositoryNames":["foo","bar"],"nextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"repositories":[{"repositoryName":"foo"}]}"#),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client
            .describe_repositories(
                Some(vec!["foo".to_string(), "bar".to_string()]),
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_repositories_paginates_until_limit_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"maxResults":3}"#),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"a"},{"repositoryName":"b"}],"nextToken":"page-2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"page-2","maxResults":1}"#),
                json_response(200, r#"{"repositories":[{"repositoryName":"c"}]}"#),
            ),
        ]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client
            .describe_repositories(None, Some(3), None)
            .await
            .unwrap();

        assert_eq!(repos.len(), 3);
        assert_eq!(
            repos
                .iter()
                .map(|r| r.repository_name.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("a"), Some("b"), Some("c")]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_repositories_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("RepositoryNotFoundException", "repository not found"),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_repositories(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("RepositoryNotFoundException".to_string()));
                assert_eq!(message, "repository not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_images_lists_all_when_no_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"foo"}"#),
            json_response(
                200,
                r#"{"imageDetails":[{"repositoryName":"foo","imageDigest":"sha256:abc","imageTags":["latest"]}]}"#,
            ),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let (images, token) = client
            .describe_images("foo", None, None, None, None)
            .await
            .unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].image_digest.as_deref(), Some("sha256:abc"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_images_filters_by_tags_and_digests() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"repositoryName":"foo","imageIds":[{"imageTag":"latest"},{"imageDigest":"sha256:abc"}]}"#,
            ),
            json_response(
                200,
                r#"{"imageDetails":[{"repositoryName":"foo","imageDigest":"sha256:abc"}]}"#,
            ),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let (images, token) = client
            .describe_images(
                "foo",
                Some(vec!["latest".to_string()]),
                Some(vec!["sha256:abc".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_images_paginates_until_limit_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"foo","maxResults":2}"#),
                json_response(
                    200,
                    r#"{"imageDetails":[{"imageDigest":"sha256:aaa"}],"nextToken":"page-2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"foo","nextToken":"page-2","maxResults":1}"#,
                ),
                json_response(200, r#"{"imageDetails":[{"imageDigest":"sha256:bbb"}]}"#),
            ),
        ]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let (images, token) = client
            .describe_images("foo", None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(images.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_images_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"foo"}"#),
            json_error_response("ImageNotFoundException", "image not found"),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_images("foo", None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ImageNotFoundException".to_string()));
                assert_eq!(message, "image not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_image_scan_findings_fetches_metadata_and_findings() {
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

        let (result, token) = client
            .describe_image_scan_findings("foo", "sha256:abc", None, None)
            .await
            .unwrap();

        assert_eq!(result.image_tags, vec!["latest".to_string()]);
        assert_eq!(result.scan_status, Some("COMPLETE".to_string()));
        assert_eq!(
            result.finding_severity_counts,
            vec![("HIGH".to_string(), 2)]
        );
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].name.as_deref(), Some("CVE-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_image_scan_findings_preserves_first_page_metadata_across_pages() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"foo","imageId":{"imageDigest":"sha256:abc"},"maxResults":2}"#,
                ),
                json_response(
                    200,
                    r#"{"imageId":{"imageDigest":"sha256:abc","imageTag":"latest"},"imageScanStatus":{"status":"COMPLETE"},"imageScanFindings":{"findingSeverityCounts":{"HIGH":1},"findings":[{"name":"CVE-1"}]},"nextToken":"page-2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"foo","imageId":{"imageDigest":"sha256:abc"},"nextToken":"page-2","maxResults":1}"#,
                ),
                json_response(
                    200,
                    r#"{"imageId":{"imageDigest":"sha256:abc","imageTag":"should-be-ignored"},"imageScanStatus":{"status":"IN_PROGRESS"},"imageScanFindings":{"findingSeverityCounts":{"HIGH":99},"findings":[{"name":"CVE-2"}]}}"#,
                ),
            ),
        ]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let (result, token) = client
            .describe_image_scan_findings("foo", "sha256:abc", Some(2), None)
            .await
            .unwrap();

        // Metadata (tag/status/severity counts) is taken from the first page
        // only, even though the second page's response contains different
        // values for those fields.
        assert_eq!(result.image_tags, vec!["latest".to_string()]);
        assert_eq!(result.scan_status, Some("COMPLETE".to_string()));
        assert_eq!(
            result.finding_severity_counts,
            vec![("HIGH".to_string(), 1)]
        );
        assert_eq!(
            result
                .findings
                .iter()
                .map(|f| f.name.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("CVE-1"), Some("CVE-2")]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_image_scan_findings_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"repositoryName":"foo","imageId":{"imageDigest":"sha256:abc"}}"#,
            ),
            json_error_response("ScanNotFoundException", "scan not found"),
        )]);
        let client = EcrClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_image_scan_findings("foo", "sha256:abc", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ScanNotFoundException".to_string()));
                assert_eq!(message, "scan not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
