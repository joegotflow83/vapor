use aws_config::SdkConfig;
use aws_sdk_macie2::types::{BucketMetadata, CriterionAdditionalProperties, Finding, FindingCriteria};

use crate::error::VaporError;

pub struct MacieClient {
    inner: aws_sdk_macie2::Client,
}

impl MacieClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_macie2::Client::new(config),
        }
    }

    pub async fn list_findings(
        &self,
        severity: Option<&str>,
        finding_type: Option<&str>,
    ) -> Result<Vec<String>, VaporError> {
        let criteria = if severity.is_some() || finding_type.is_some() {
            let mut b = FindingCriteria::builder();
            if let Some(sev) = severity {
                b = b.criterion(
                    "severity.description",
                    CriterionAdditionalProperties::builder()
                        .set_eq(Some(vec![sev.to_string()]))
                        .build(),
                );
            }
            if let Some(ft) = finding_type {
                b = b.criterion(
                    "type",
                    CriterionAdditionalProperties::builder()
                        .set_eq(Some(vec![ft.to_string()]))
                        .build(),
                );
            }
            Some(b.build())
        } else {
            None
        };

        let mut ids = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_findings();
            if let Some(ref c) = criteria {
                req = req.finding_criteria(c.clone());
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token.clone());
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            ids.extend(output.finding_ids().iter().cloned());
            next_token = output.next_token().map(|t| t.to_string());
            if next_token.is_none() {
                break;
            }
        }
        Ok(ids)
    }

    pub async fn get_findings(&self, ids: Vec<String>) -> Result<Vec<Finding>, VaporError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let mut results = Vec::new();
        for chunk in ids.chunks(25) {
            let output = self
                .inner
                .get_findings()
                .set_finding_ids(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.findings().iter().cloned());
        }
        Ok(results)
    }

    pub async fn describe_buckets(&self) -> Result<Vec<BucketMetadata>, VaporError> {
        let mut buckets = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.describe_buckets();
            if let Some(ref token) = next_token {
                req = req.next_token(token.clone());
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            buckets.extend(output.buckets().iter().cloned());
            next_token = output.next_token().map(|t| t.to_string());
            if next_token.is_none() {
                break;
            }
        }
        Ok(buckets)
    }
}
