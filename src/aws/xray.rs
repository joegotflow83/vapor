use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct XRayGroupInsightsConfig {
    pub insights_enabled: Option<bool>,
    pub notifications_enabled: Option<bool>,
}

pub struct XRayGroupInfo {
    pub group_name: Option<String>,
    pub group_arn: Option<String>,
    pub filter_expression: Option<String>,
    pub insights_configuration: Option<XRayGroupInsightsConfig>,
}

pub struct XRaySamplingRuleInfo {
    pub rule_name: Option<String>,
    pub rule_arn: Option<String>,
    pub priority: Option<i32>,
    pub fixed_rate: Option<f64>,
    pub reservoir_size: Option<i32>,
    pub service_name: Option<String>,
    pub service_type: Option<String>,
    pub host: Option<String>,
    pub http_method: Option<String>,
    pub url_path: Option<String>,
    pub resource_arn: Option<String>,
    pub version: Option<i32>,
}

pub struct XRayEncryptionConfigInfo {
    pub key_id: Option<String>,
    pub status: Option<String>,
    pub type_: Option<String>,
}

pub struct XRayClient {
    inner: aws_sdk_xray::Client,
}

impl XRayClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_xray::Client::new(config),
        }
    }

    pub async fn get_groups(&self) -> Result<Vec<XRayGroupInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.get_groups();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for g in output.groups() {
                items.push(XRayGroupInfo {
                    group_name: g.group_name().map(|s| s.to_string()),
                    group_arn: g.group_arn().map(|s| s.to_string()),
                    filter_expression: g.filter_expression().map(|s| s.to_string()),
                    insights_configuration: g.insights_configuration().map(|ic| {
                        XRayGroupInsightsConfig {
                            insights_enabled: ic.insights_enabled(),
                            notifications_enabled: ic.notifications_enabled(),
                        }
                    }),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_sampling_rules(&self) -> Result<Vec<XRaySamplingRuleInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.get_sampling_rules();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for record in output.sampling_rule_records() {
                if let Some(rule) = record.sampling_rule() {
                    items.push(XRaySamplingRuleInfo {
                        rule_name: rule.rule_name().map(|s| s.to_string()),
                        rule_arn: rule.rule_arn().map(|s| s.to_string()),
                        priority: Some(rule.priority()),
                        fixed_rate: Some(rule.fixed_rate()),
                        reservoir_size: Some(rule.reservoir_size()),
                        service_name: Some(rule.service_name().to_string()),
                        service_type: Some(rule.service_type().to_string()),
                        host: Some(rule.host().to_string()),
                        http_method: Some(rule.http_method().to_string()),
                        url_path: Some(rule.url_path().to_string()),
                        resource_arn: Some(rule.resource_arn().to_string()),
                        version: Some(rule.version()),
                    });
                }
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_encryption_config(
        &self,
    ) -> Result<Option<XRayEncryptionConfigInfo>, VaporError> {
        let output = self
            .inner
            .get_encryption_config()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.encryption_config().map(|ec| XRayEncryptionConfigInfo {
            key_id: ec.key_id().map(|s| s.to_string()),
            status: ec.status().map(|s| s.as_str().to_string()),
            type_: ec.r#type().map(|t| t.as_str().to_string()),
        }))
    }
}
