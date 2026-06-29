use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct PrivateCaInfo {
    pub inner: aws_sdk_acmpca::types::CertificateAuthority,
    pub tags: Vec<(String, String)>,
}

pub struct AcmPcaClient {
    inner: aws_sdk_acmpca::Client,
}

impl AcmPcaClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_acmpca::Client::new(config),
        }
    }

    async fn fetch_tags(&self, arn: &str) -> Vec<(String, String)> {
        let mut tags = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_tags().certificate_authority_arn(arn);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            match req.send().await {
                Ok(output) => {
                    for tag in output.tags() {
                        tags.push((
                            tag.key().to_string(),
                            tag.value().unwrap_or_default().to_string(),
                        ));
                    }
                    match output.next_token() {
                        Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                        _ => break,
                    }
                }
                Err(_) => break,
            }
        }
        tags
    }

    pub async fn list_certificate_authorities(&self) -> Result<Vec<PrivateCaInfo>, VaporError> {
        let mut cas = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_certificate_authorities();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            cas.extend(output.certificate_authorities().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        let mut result = Vec::with_capacity(cas.len());
        for ca in cas {
            let arn = ca.arn().unwrap_or_default().to_string();
            let tags = if arn.is_empty() {
                vec![]
            } else {
                self.fetch_tags(&arn).await
            };
            result.push(PrivateCaInfo { inner: ca, tags });
        }
        Ok(result)
    }

    pub async fn describe_certificate_authority(
        &self,
        certificate_authority_arn: &str,
    ) -> Result<Option<PrivateCaInfo>, VaporError> {
        let output = self
            .inner
            .describe_certificate_authority()
            .certificate_authority_arn(certificate_authority_arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        match output.certificate_authority().cloned() {
            Some(ca) => {
                let tags = self.fetch_tags(certificate_authority_arn).await;
                Ok(Some(PrivateCaInfo { inner: ca, tags }))
            }
            None => Ok(None),
        }
    }
}
