use aws_config::SdkConfig;
use aws_sdk_organizations::types::PolicyType;

use crate::error::VaporError;

pub struct OrganizationsClient {
    inner: aws_sdk_organizations::Client,
}

impl OrganizationsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_organizations::Client::new(config),
        }
    }

    pub async fn list_accounts(
        &self,
    ) -> Result<Vec<aws_sdk_organizations::types::Account>, VaporError> {
        let mut accounts = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_accounts();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            accounts.extend(output.accounts().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(accounts)
    }

    pub async fn list_organizational_units_for_parent(
        &self,
        parent_id: &str,
    ) -> Result<Vec<aws_sdk_organizations::types::OrganizationalUnit>, VaporError> {
        let mut ous = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_organizational_units_for_parent().parent_id(parent_id);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            ous.extend(output.organizational_units().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(ous)
    }

    pub async fn list_policies(
        &self,
        policy_type: PolicyType,
    ) -> Result<Vec<aws_sdk_organizations::types::PolicySummary>, VaporError> {
        let mut policies = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_policies().filter(policy_type.clone());
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            policies.extend(output.policies().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(policies)
    }
}
