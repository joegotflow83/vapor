use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct RamClient {
    inner: aws_sdk_ram::Client,
}

impl RamClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ram::Client::new(config),
        }
    }

    pub async fn list_resource_shares(
        &self,
        resource_owner: Option<&str>,
    ) -> Result<Vec<aws_sdk_ram::types::ResourceShare>, VaporError> {
        let owner = aws_sdk_ram::types::ResourceOwner::from(resource_owner.unwrap_or("SELF"));
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .get_resource_shares()
                .resource_owner(owner.clone());
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.resource_shares().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_resources(
        &self,
        resource_owner: &str,
        resource_share_arns: Option<Vec<String>>,
        resource_type: Option<String>,
    ) -> Result<Vec<aws_sdk_ram::types::Resource>, VaporError> {
        let owner = aws_sdk_ram::types::ResourceOwner::from(resource_owner);
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_resources().resource_owner(owner.clone());
            if let Some(ref arns) = resource_share_arns {
                req = req.set_resource_share_arns(Some(arns.clone()));
            }
            if let Some(ref rt) = resource_type {
                req = req.resource_type(rt);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.resources().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_principals(
        &self,
        resource_owner: &str,
        resource_share_arns: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_ram::types::Principal>, VaporError> {
        let owner = aws_sdk_ram::types::ResourceOwner::from(resource_owner);
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_principals().resource_owner(owner.clone());
            if let Some(ref arns) = resource_share_arns {
                req = req.set_resource_share_arns(Some(arns.clone()));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.principals().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
