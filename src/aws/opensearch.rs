use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct OpenSearchClient {
    inner: aws_sdk_opensearch::Client,
}

impl OpenSearchClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_opensearch::Client::new(config),
        }
    }

    /// List all OpenSearch domain names (single call, no pagination, max 100).
    pub async fn list_domain_names(&self) -> Result<Vec<String>, VaporError> {
        let output = self
            .inner
            .list_domain_names()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output
            .domain_names()
            .iter()
            .filter_map(|d| d.domain_name().map(|s| s.to_string()))
            .collect())
    }

    /// Describe OpenSearch domains in batches of 5 (hard API limit per request).
    pub async fn describe_domains(
        &self,
        domain_names: &[String],
    ) -> Result<Vec<aws_sdk_opensearch::types::DomainStatus>, VaporError> {
        let mut results = Vec::new();
        for chunk in domain_names.chunks(5) {
            let output = self
                .inner
                .describe_domains()
                .set_domain_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.domain_status_list().iter().cloned());
        }
        Ok(results)
    }

    /// List tags for a domain by ARN (single call, no pagination).
    pub async fn list_tags(
        &self,
        arn: &str,
    ) -> Result<Vec<aws_sdk_opensearch::types::Tag>, VaporError> {
        let output = self
            .inner
            .list_tags()
            .arn(arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.tag_list().to_vec())
    }
}
