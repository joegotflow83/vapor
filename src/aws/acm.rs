#[cfg(feature = "acm")]
use aws_config::SdkConfig;
#[cfg(feature = "acm")]
use aws_sdk_acm::types::{CertificateDetail, CertificateStatus, Tag as AcmTag};

#[cfg(feature = "acm")]
use crate::error::VaporError;

#[cfg(feature = "acm")]
pub struct AcmClient {
    inner: aws_sdk_acm::Client,
}

impl AcmClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_acm::Client::new(config),
        }
    }

    /// Paginate list_certificates. If statuses is non-empty, filter by those statuses.
    /// Returns certificate ARNs.
    pub async fn list_certificates(&self, statuses: Vec<String>) -> Result<Vec<String>, VaporError> {
        let mut arns: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_certificates();
            if !statuses.is_empty() {
                let status_enums: Vec<CertificateStatus> = statuses
                    .iter()
                    .map(|s| CertificateStatus::from(s.as_str()))
                    .collect();
                req = req.set_certificate_statuses(Some(status_enums));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for summary in output.certificate_summary_list() {
                if let Some(arn) = summary.certificate_arn() {
                    arns.push(arn.to_string());
                }
            }

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(arns)
    }

    /// Fetch full certificate metadata. Returns None if the certificate does not exist.
    pub async fn describe_certificate(&self, arn: &str) -> Result<Option<CertificateDetail>, VaporError> {
        match self.inner.describe_certificate().certificate_arn(arn).send().await {
            Ok(output) => Ok(output.certificate),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_resource_not_found_exception() {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(svc_err.to_string()))
                }
            }
        }
    }

    /// Fetch tags for a certificate.
    pub async fn list_tags_for_certificate(&self, arn: &str) -> Result<Vec<AcmTag>, VaporError> {
        let output = self
            .inner
            .list_tags_for_certificate()
            .certificate_arn(arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.tags().to_vec())
    }
}
