#[cfg(feature = "ecr")]
use aws_config::SdkConfig;
#[cfg(feature = "ecr")]
use aws_sdk_ecr::types::{ImageDetail, ImageIdentifier, ImageScanFinding, Repository};

#[cfg(feature = "ecr")]
use crate::error::VaporError;

#[cfg(feature = "ecr")]
/// Aggregated result from paginated describe_image_scan_findings calls.
pub struct ImageScanFindingsResult {
    pub image_tags: Vec<String>,
    pub scan_status: Option<String>,
    pub scan_completed_at: Option<String>,
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

    pub async fn describe_repositories(
        &self,
        names: Option<Vec<String>>,
    ) -> Result<Vec<Repository>, VaporError> {
        let mut repos = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_repositories();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            if let Some(ref names_vec) = names {
                for name in names_vec {
                    req = req.repository_names(name);
                }
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for r in output.repositories() {
                repos.push(r.clone());
            }
            match output.next_token() {
                Some(tok) => next_token = Some(tok.to_string()),
                None => break,
            }
        }

        Ok(repos)
    }

    pub async fn describe_images(
        &self,
        repository_name: &str,
        image_tags: Option<Vec<String>>,
        image_digests: Option<Vec<String>>,
    ) -> Result<Vec<ImageDetail>, VaporError> {
        let mut images = Vec::new();
        let mut next_token: Option<String> = None;

        // Build image identifiers if filters provided
        let mut image_ids: Vec<ImageIdentifier> = Vec::new();
        if let Some(ref tags) = image_tags {
            for tag in tags {
                image_ids.push(
                    ImageIdentifier::builder()
                        .image_tag(tag)
                        .build(),
                );
            }
        }
        if let Some(ref digests) = image_digests {
            for digest in digests {
                image_ids.push(
                    ImageIdentifier::builder()
                        .image_digest(digest)
                        .build(),
                );
            }
        }

        loop {
            let mut req = self
                .inner
                .describe_images()
                .repository_name(repository_name);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            for id in &image_ids {
                req = req.image_ids(id.clone());
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for img in output.image_details() {
                images.push(img.clone());
            }
            match output.next_token() {
                Some(tok) => next_token = Some(tok.to_string()),
                None => break,
            }
        }

        Ok(images)
    }

    /// Fetch image scan findings for a specific image (identified by digest).
    /// Paginates through all findings and returns an aggregated result.
    pub async fn describe_image_scan_findings(
        &self,
        repository_name: &str,
        image_digest: &str,
    ) -> Result<ImageScanFindingsResult, VaporError> {
        let image_id = ImageIdentifier::builder()
            .image_digest(image_digest)
            .build();

        let mut all_findings: Vec<ImageScanFinding> = Vec::new();
        let mut next_token: Option<String> = None;
        let mut scan_status: Option<String> = None;
        let mut scan_completed_at: Option<String> = None;
        let mut image_tags: Vec<String> = Vec::new();
        let mut finding_severity_counts: Vec<(String, i64)> = Vec::new();
        let mut counts_collected = false;

        loop {
            let mut req = self
                .inner
                .describe_image_scan_findings()
                .repository_name(repository_name)
                .image_id(image_id.clone());
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            // Capture metadata only on first page
            if scan_status.is_none() {
                scan_status = output
                    .image_scan_status()
                    .and_then(|s| s.status())
                    .map(|s| s.as_str().to_string());
            }
            if let Some(id) = output.image_id() {
                if image_tags.is_empty() {
                    if let Some(tag) = id.image_tag() {
                        image_tags.push(tag.to_string());
                    }
                }
            }
            if let Some(findings_obj) = output.image_scan_findings() {
                if scan_completed_at.is_none() {
                    scan_completed_at = findings_obj
                        .image_scan_completed_at()
                        .map(|d| d.to_string());
                }
                if !counts_collected {
                    if let Some(counts) = findings_obj.finding_severity_counts() {
                        for (severity, count) in counts {
                            finding_severity_counts
                                .push((severity.as_str().to_string(), (*count).into()));
                        }
                    }
                    counts_collected = true;
                }
                for f in findings_obj.findings() {
                    all_findings.push(f.clone());
                }
            }

            match output.next_token() {
                Some(tok) => next_token = Some(tok.to_string()),
                None => break,
            }
        }

        Ok(ImageScanFindingsResult {
            image_tags,
            scan_status,
            scan_completed_at,
            finding_severity_counts,
            findings: all_findings,
        })
    }
}
