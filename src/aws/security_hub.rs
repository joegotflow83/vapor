use aws_config::SdkConfig;
use aws_sdk_securityhub::types::{
    AwsSecurityFinding, AwsSecurityFindingFilters, StringFilter, StringFilterComparison,
};

use crate::error::VaporError;

pub struct SecurityHubClient {
    inner: aws_sdk_securityhub::Client,
}

impl SecurityHubClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_securityhub::Client::new(config),
        }
    }

    pub async fn get_findings(
        &self,
        severity_label: Option<String>,
        workflow_status: Option<String>,
        record_state: Option<String>,
        max_results: Option<i32>,
    ) -> Result<Vec<AwsSecurityFinding>, VaporError> {
        let has_filters =
            severity_label.is_some() || workflow_status.is_some() || record_state.is_some();

        let filters = if has_filters {
            let mut fb = AwsSecurityFindingFilters::builder();

            if let Some(sl) = severity_label {
                let f = StringFilter::builder()
                    .value(sl)
                    .comparison(StringFilterComparison::Equals)
                    .build();
                fb = fb.severity_label(f);
            }

            if let Some(ws) = workflow_status {
                let f = StringFilter::builder()
                    .value(ws)
                    .comparison(StringFilterComparison::Equals)
                    .build();
                fb = fb.workflow_status(f);
            }

            if let Some(rs) = record_state {
                let f = StringFilter::builder()
                    .value(rs)
                    .comparison(StringFilterComparison::Equals)
                    .build();
                fb = fb.record_state(f);
            }

            Some(fb.build())
        } else {
            None
        };

        let mut findings = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.get_findings();
            if let Some(ref f) = filters {
                req = req.filters(f.clone());
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            if let Some(max) = max_results {
                req = req.max_results(max);
            }

            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            findings.extend(output.findings().to_vec());

            // If max_results was set, stop after first page
            if max_results.is_some() {
                break;
            }

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(findings)
    }
}
