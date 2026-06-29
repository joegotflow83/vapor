use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct FmsClient {
    inner: aws_sdk_fms::Client,
}

impl FmsClient {
    pub fn new(config: &SdkConfig) -> Self {
        let fms_config = aws_sdk_fms::config::Builder::from(config)
            .region(aws_sdk_fms::config::Region::new("us-east-1"))
            .build();
        Self {
            inner: aws_sdk_fms::Client::from_conf(fms_config),
        }
    }

    pub async fn list_policies(
        &self,
    ) -> Result<Vec<aws_sdk_fms::types::PolicySummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_policies();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.policy_list().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_compliance_status(
        &self,
        policy_id: &str,
    ) -> Result<Vec<aws_sdk_fms::types::PolicyComplianceStatus>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_compliance_status()
                .policy_id(policy_id);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.policy_compliance_status_list().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_member_accounts(&self) -> Result<Vec<String>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_member_accounts();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.member_accounts().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
