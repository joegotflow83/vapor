use aws_config::SdkConfig;
use aws_sdk_globalaccelerator::types::{Accelerator, EndpointGroup, Listener};

use crate::error::VaporError;

pub struct GlobalAcceleratorClient {
    inner: aws_sdk_globalaccelerator::Client,
}

impl GlobalAcceleratorClient {
    pub fn new(config: &SdkConfig) -> Self {
        let ga_config = aws_sdk_globalaccelerator::config::Builder::from(config)
            .region(aws_sdk_globalaccelerator::config::Region::new("us-west-2"))
            .build();
        Self {
            inner: aws_sdk_globalaccelerator::Client::from_conf(ga_config),
        }
    }

    pub async fn list_accelerators(&self) -> Result<Vec<Accelerator>, VaporError> {
        let mut accelerators = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_accelerators();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            accelerators.extend(output.accelerators().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(accelerators)
    }

    pub async fn list_listeners(&self, accelerator_arn: &str) -> Result<Vec<Listener>, VaporError> {
        let mut listeners = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_listeners()
                .accelerator_arn(accelerator_arn);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            listeners.extend(output.listeners().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(listeners)
    }

    pub async fn list_endpoint_groups(
        &self,
        listener_arn: &str,
    ) -> Result<Vec<EndpointGroup>, VaporError> {
        let mut groups = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_endpoint_groups()
                .listener_arn(listener_arn);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            groups.extend(output.endpoint_groups().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(groups)
    }
}
