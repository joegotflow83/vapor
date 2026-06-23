use aws_config::SdkConfig;
use aws_sdk_cloudfront::types::{DistributionSummary, Tag};

use crate::error::VaporError;

pub struct CloudFrontClient {
    inner: aws_sdk_cloudfront::Client,
}

impl CloudFrontClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cloudfront::Client::new(config),
        }
    }

    /// Paginate list_distributions via distribution_list.next_marker.
    pub async fn list_distributions(&self) -> Result<Vec<DistributionSummary>, VaporError> {
        let mut distributions = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_distributions();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            if let Some(dist_list) = output.distribution_list() {
                for d in dist_list.items() {
                    distributions.push(d.clone());
                }
                if dist_list.is_truncated() {
                    marker = dist_list.next_marker().map(|s| s.to_string());
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(distributions)
    }

    /// Fetch tags for a distribution by ARN.
    pub async fn list_tags_for_resource(&self, arn: &str) -> Result<Vec<Tag>, VaporError> {
        let output = self
            .inner
            .list_tags_for_resource()
            .resource(arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.tags().map(|t| t.items().to_vec()).unwrap_or_default())
    }

    /// Fetch a single distribution by ID. Returns None if not found.
    pub async fn get_distribution(
        &self,
        id: &str,
    ) -> Result<Option<aws_sdk_cloudfront::types::Distribution>, VaporError> {
        match self.inner.get_distribution().id(id).send().await {
            Ok(output) => Ok(output.distribution().cloned()),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_no_such_distribution() {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(svc_err.to_string()))
                }
            }
        }
    }
}
