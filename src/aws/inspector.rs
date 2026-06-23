use aws_config::SdkConfig;
use aws_sdk_inspector2::types::{
    CoveredResource, FilterCriteria, Finding, StringComparison, StringFilter,
};

use crate::error::VaporError;

pub struct InspectorClient {
    inner: aws_sdk_inspector2::Client,
}

impl InspectorClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_inspector2::Client::new(config),
        }
    }

    pub async fn list_findings(
        &self,
        severity: Option<String>,
        resource_type: Option<String>,
    ) -> Result<Vec<Finding>, VaporError> {
        let filter = if severity.is_some() || resource_type.is_some() {
            let mut fb = FilterCriteria::builder();
            if let Some(sev) = severity {
                let f = StringFilter::builder()
                    .value(sev)
                    .comparison(StringComparison::Equals)
                    .build()
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                fb = fb.severity(f);
            }
            if let Some(rt) = resource_type {
                let f = StringFilter::builder()
                    .value(rt)
                    .comparison(StringComparison::Equals)
                    .build()
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                fb = fb.resource_type(f);
            }
            Some(fb.build())
        } else {
            None
        };

        let mut findings = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_findings();
            if let Some(ref f) = filter {
                req = req.filter_criteria(f.clone());
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            findings.extend(output.findings().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(findings)
    }

    pub async fn list_coverage(&self) -> Result<Vec<CoveredResource>, VaporError> {
        let mut resources = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_coverage();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            resources.extend(output.covered_resources().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(resources)
    }
}
