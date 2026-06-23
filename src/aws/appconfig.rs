use aws_config::SdkConfig;
use aws_sdk_appconfig::types::{Application, ConfigurationProfileSummary, Environment};

use crate::error::VaporError;

pub struct AppConfigClient {
    inner: aws_sdk_appconfig::Client,
}

impl AppConfigClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_appconfig::Client::new(config),
        }
    }

    pub async fn list_applications(&self) -> Result<Vec<Application>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_applications();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.items().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_environments(&self, application_id: &str) -> Result<Vec<Environment>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_environments().application_id(application_id);
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.items().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_configuration_profiles(&self, application_id: &str) -> Result<Vec<ConfigurationProfileSummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_configuration_profiles().application_id(application_id);
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.items().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
