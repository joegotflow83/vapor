use aws_config::SdkConfig;
use aws_sdk_redshiftserverless::types::{Namespace, Workgroup};

use crate::error::VaporError;

pub struct RedshiftServerlessClient {
    inner: aws_sdk_redshiftserverless::Client,
}

impl RedshiftServerlessClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_redshiftserverless::Client::new(config),
        }
    }

    pub async fn list_namespaces(&self) -> Result<Vec<Namespace>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_namespaces();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.namespaces().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_workgroups(&self) -> Result<Vec<Workgroup>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_workgroups();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.workgroups().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
