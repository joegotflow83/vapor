use async_graphql::{Context, Object, Result};

use crate::aws::ecr::EcrClient;
use crate::schema::ecr::types::{EcrImage, EcrImageScanFindings, EcrRepository};

#[derive(Default)]
pub struct EcrQuery;

#[Object]
impl EcrQuery {
    /// List ECR repositories. Optionally filter by repository names.
    async fn ecr_repositories(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<EcrRepository>> {
        let ecr = ctx.data::<EcrClient>()?;
        let repos = ecr.describe_repositories(names).await?;
        Ok(repos.into_iter().map(EcrRepository::from).collect())
    }

    /// List images in an ECR repository. Optionally filter by image tags or digests.
    async fn ecr_images(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
        image_tags: Option<Vec<String>>,
        image_digests: Option<Vec<String>>,
    ) -> Result<Vec<EcrImage>> {
        let ecr = ctx.data::<EcrClient>()?;
        let images = ecr
            .describe_images(&repository_name, image_tags, image_digests)
            .await?;
        Ok(images.into_iter().map(EcrImage::from).collect())
    }

    /// Fetch CVE/vulnerability scan findings for a specific image by digest.
    /// Security value: detect vulnerable container images, audit CVE exposure by severity.
    /// Use ecr_images first to get an image digest, then pass it here.
    async fn ecr_image_scan_findings(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
        image_digest: String,
    ) -> Result<EcrImageScanFindings> {
        let ecr = ctx.data::<EcrClient>()?;
        let result = ecr
            .describe_image_scan_findings(&repository_name, &image_digest)
            .await?;
        Ok(EcrImageScanFindings::from(result))
    }
}
