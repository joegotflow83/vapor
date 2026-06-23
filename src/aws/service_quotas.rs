use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct ServiceQuotasClient {
    inner: aws_sdk_servicequotas::Client,
}

impl ServiceQuotasClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_servicequotas::Client::new(config),
        }
    }

    pub async fn list_service_quotas(
        &self,
        service_code: &str,
    ) -> Result<Vec<aws_sdk_servicequotas::types::ServiceQuota>, VaporError> {
        let mut quotas = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_service_quotas().service_code(service_code);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            quotas.extend(output.quotas().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(quotas)
    }

    pub async fn list_services(
        &self,
    ) -> Result<Vec<aws_sdk_servicequotas::types::ServiceInfo>, VaporError> {
        let mut services = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_services();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            services.extend(output.services().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(services)
    }
}
