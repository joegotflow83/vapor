use aws_config::SdkConfig;
use aws_sdk_shield::error::SdkError;

use crate::error::VaporError;

pub struct ShieldClient {
    inner: aws_sdk_shield::Client,
}

impl ShieldClient {
    pub fn new(config: &SdkConfig) -> Self {
        let shield_config = aws_sdk_shield::config::Builder::from(config)
            .region(aws_sdk_shield::config::Region::new("us-east-1"))
            .build();
        Self {
            inner: aws_sdk_shield::Client::from_conf(shield_config),
        }
    }

    pub async fn describe_subscription(
        &self,
    ) -> Result<Option<aws_sdk_shield::types::Subscription>, VaporError> {
        match self.inner.describe_subscription().send().await {
            Ok(output) => Ok(output.subscription().cloned()),
            Err(SdkError::ServiceError(e)) if e.err().is_resource_not_found_exception() => {
                Ok(None)
            }
            Err(e) => Err(VaporError::AwsSdk(e.to_string())),
        }
    }

    pub async fn list_protections(
        &self,
        resource_arn: Option<&str>,
    ) -> Result<Vec<aws_sdk_shield::types::Protection>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_protections();

            if let Some(arn) = resource_arn {
                let filter = aws_sdk_shield::types::InclusionProtectionFilters::builder()
                    .resource_arns(arn)
                    .build();
                req = req.inclusion_filters(filter);
            }

            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.protections().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_protection_groups(
        &self,
    ) -> Result<Vec<aws_sdk_shield::types::ProtectionGroup>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_protection_groups();

            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.protection_groups().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_attacks(
        &self,
        resource_arns: Option<Vec<String>>,
        start_time: Option<String>,
        end_time: Option<String>,
    ) -> Result<Vec<aws_sdk_shield::types::AttackSummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        let start_dt = start_time.as_deref().and_then(parse_datetime);
        let end_dt = end_time.as_deref().and_then(parse_datetime);

        loop {
            let mut req = self.inner.list_attacks();

            if let Some(ref arns) = resource_arns {
                for arn in arns {
                    req = req.resource_arns(arn);
                }
            }

            if let Some(dt) = start_dt {
                let time_range = aws_sdk_shield::types::TimeRange::builder()
                    .from_inclusive(dt)
                    .build();
                req = req.start_time(time_range);
            }

            if let Some(dt) = end_dt {
                let time_range = aws_sdk_shield::types::TimeRange::builder()
                    .to_exclusive(dt)
                    .build();
                req = req.end_time(time_range);
            }

            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.attack_summaries().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}

fn parse_datetime(s: &str) -> Option<aws_sdk_shield::primitives::DateTime> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| aws_sdk_shield::primitives::DateTime::from_secs(dt.timestamp()))
}
