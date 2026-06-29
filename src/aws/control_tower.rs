use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct ControlTowerClient {
    inner: aws_sdk_controltower::Client,
}

impl ControlTowerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_controltower::Client::new(config),
        }
    }

    pub async fn list_landing_zones(
        &self,
    ) -> Result<Vec<aws_sdk_controltower::types::LandingZoneDetail>, VaporError> {
        let mut arns = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_landing_zones();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for summary in output.landing_zones() {
                if let Some(arn) = summary.arn() {
                    arns.push(arn.to_string());
                }
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        let mut details = Vec::new();
        for arn in arns {
            let output = self
                .inner
                .get_landing_zone()
                .landing_zone_identifier(&arn)
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            if let Some(detail) = output.landing_zone() {
                details.push(detail.clone());
            }
        }
        Ok(details)
    }

    pub async fn list_enabled_controls(
        &self,
        target_identifier: Option<String>,
    ) -> Result<Vec<aws_sdk_controltower::types::EnabledControlSummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_enabled_controls();
            if let Some(ref target) = target_identifier {
                req = req.target_identifier(target);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.enabled_controls().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }
        Ok(items)
    }
}
