#[cfg(feature = "autoscaling")]
use aws_config::SdkConfig;

#[cfg(feature = "autoscaling")]
use crate::error::VaporError;

#[cfg(feature = "autoscaling")]
pub struct AutoscalingClient {
    inner: aws_sdk_autoscaling::Client,
}

impl AutoscalingClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_autoscaling::Client::new(config),
        }
    }

    pub async fn describe_auto_scaling_groups(
        &self,
        names: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_autoscaling::types::AutoScalingGroup>, VaporError> {
        let mut all_items: Vec<aws_sdk_autoscaling::types::AutoScalingGroup> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_auto_scaling_groups();

            if let Some(ref ns) = names {
                if !ns.is_empty() {
                    request = request.set_auto_scaling_group_names(Some(ns.clone()));
                }
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_items.extend(output.auto_scaling_groups().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }

    pub async fn describe_scaling_activities(
        &self,
        group_name: Option<String>,
    ) -> Result<Vec<aws_sdk_autoscaling::types::Activity>, VaporError> {
        let mut all_items: Vec<aws_sdk_autoscaling::types::Activity> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_scaling_activities();

            if let Some(ref name) = group_name {
                request = request.auto_scaling_group_name(name);
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_items.extend(output.activities().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }
}
