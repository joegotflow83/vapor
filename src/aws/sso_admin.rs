use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct SsoAdminClient {
    inner: aws_sdk_ssoadmin::Client,
}

impl SsoAdminClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ssoadmin::Client::new(config),
        }
    }

    pub async fn list_instances(
        &self,
    ) -> Result<Vec<aws_sdk_ssoadmin::types::InstanceMetadata>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_instances();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.instances().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_permission_sets(
        &self,
        instance_arn: &str,
    ) -> Result<Vec<String>, VaporError> {
        let mut arns = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_permission_sets().instance_arn(instance_arn);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            arns.extend(output.permission_sets().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(arns)
    }

    pub async fn describe_permission_set(
        &self,
        instance_arn: &str,
        permission_set_arn: &str,
    ) -> Result<Option<aws_sdk_ssoadmin::types::PermissionSet>, VaporError> {
        let output = self
            .inner
            .describe_permission_set()
            .instance_arn(instance_arn)
            .permission_set_arn(permission_set_arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.permission_set().cloned())
    }

    pub async fn list_account_assignments(
        &self,
        instance_arn: &str,
        account_id: &str,
        permission_set_arn: &str,
    ) -> Result<Vec<aws_sdk_ssoadmin::types::AccountAssignment>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_account_assignments()
                .instance_arn(instance_arn)
                .account_id(account_id)
                .permission_set_arn(permission_set_arn);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.account_assignments().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
