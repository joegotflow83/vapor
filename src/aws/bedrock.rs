use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct BedrockFoundationModelInfo {
    pub model_id: String,
    pub model_name: Option<String>,
    pub provider_name: Option<String>,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    pub model_lifecycle_status: Option<String>,
    pub response_streaming_supported: Option<bool>,
    pub customizations_supported: Vec<String>,
}

pub struct BedrockCustomModelInfo {
    pub model_arn: Option<String>,
    pub model_name: Option<String>,
    pub creation_time: Option<String>,
    pub base_model_arn: Option<String>,
    pub customization_type: Option<String>,
    pub job_arn: Option<String>,
}

pub struct BedrockGuardrailInfo {
    pub guardrail_id: Option<String>,
    pub guardrail_arn: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub version: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub description: Option<String>,
}

pub struct BedrockS3ConfigInfo {
    pub bucket_name: Option<String>,
    pub key_prefix: Option<String>,
}

pub struct BedrockCloudWatchConfigInfo {
    pub log_group_name: Option<String>,
    pub role_arn: Option<String>,
    pub large_data_delivery_s3_config: Option<BedrockS3ConfigInfo>,
}

pub struct BedrockModelInvocationLoggingConfigInfo {
    pub cloudwatch_config: Option<BedrockCloudWatchConfigInfo>,
    pub s3_config: Option<BedrockS3ConfigInfo>,
}

pub struct BedrockClient {
    inner: aws_sdk_bedrock::Client,
}

impl BedrockClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_bedrock::Client::new(config),
        }
    }

    pub async fn list_foundation_models(
        &self,
        provider: Option<String>,
        by_output_modality: Option<String>,
        by_inference_type: Option<String>,
    ) -> Result<Vec<BedrockFoundationModelInfo>, VaporError> {
        let mut req = self.inner.list_foundation_models();
        if let Some(p) = provider {
            req = req.by_provider(p);
        }
        if let Some(m) = by_output_modality {
            req = req.by_output_modality(aws_sdk_bedrock::types::ModelModality::from(m.as_str()));
        }
        if let Some(t) = by_inference_type {
            req = req.by_inference_type(aws_sdk_bedrock::types::InferenceType::from(t.as_str()));
        }
        let output = req
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output
            .model_summaries()
            .iter()
            .map(|m| BedrockFoundationModelInfo {
                model_id: m.model_id().to_string(),
                model_name: m.model_name().map(|s| s.to_string()),
                provider_name: m.provider_name().map(|s| s.to_string()),
                input_modalities: m
                    .input_modalities()
                    .iter()
                    .map(|mod_| mod_.as_str().to_string())
                    .collect(),
                output_modalities: m
                    .output_modalities()
                    .iter()
                    .map(|mod_| mod_.as_str().to_string())
                    .collect(),
                model_lifecycle_status: m
                    .model_lifecycle()
                    .map(|lc| lc.status().as_str().to_string()),
                response_streaming_supported: m.response_streaming_supported(),
                customizations_supported: m
                    .customizations_supported()
                    .iter()
                    .map(|c| c.as_str().to_string())
                    .collect(),
            })
            .collect())
    }

    pub async fn list_custom_models(&self) -> Result<Vec<BedrockCustomModelInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_custom_models();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for m in output.model_summaries() {
                items.push(BedrockCustomModelInfo {
                    model_arn: Some(m.model_arn().to_string()),
                    model_name: Some(m.model_name().to_string()),
                    creation_time: Some(m.creation_time().to_string()),
                    base_model_arn: Some(m.base_model_arn().to_string()),
                    customization_type: m.customization_type().map(|t| t.as_str().to_string()),
                    job_arn: None,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_guardrails(&self) -> Result<Vec<BedrockGuardrailInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_guardrails();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for g in output.guardrails() {
                items.push(BedrockGuardrailInfo {
                    guardrail_id: Some(g.id().to_string()),
                    guardrail_arn: Some(g.arn().to_string()),
                    name: Some(g.name().to_string()),
                    status: Some(g.status().as_str().to_string()),
                    version: Some(g.version().to_string()),
                    created_at: Some(g.created_at().to_string()),
                    updated_at: Some(g.updated_at().to_string()),
                    description: g.description().map(|s| s.to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_model_invocation_logging_config(
        &self,
    ) -> Result<Option<BedrockModelInvocationLoggingConfigInfo>, VaporError> {
        let output = self
            .inner
            .get_model_invocation_logging_configuration()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.logging_config().map(|lc| {
            BedrockModelInvocationLoggingConfigInfo {
                cloudwatch_config: lc.cloud_watch_config().map(|cw| {
                    BedrockCloudWatchConfigInfo {
                        log_group_name: Some(cw.log_group_name().to_string()),
                        role_arn: Some(cw.role_arn().to_string()),
                        large_data_delivery_s3_config: cw
                            .large_data_delivery_s3_config()
                            .map(|s3| BedrockS3ConfigInfo {
                                bucket_name: Some(s3.bucket_name().to_string()),
                                key_prefix: s3.key_prefix().map(|s| s.to_string()),
                            }),
                    }
                }),
                s3_config: lc.s3_config().map(|s3| BedrockS3ConfigInfo {
                    bucket_name: Some(s3.bucket_name().to_string()),
                    key_prefix: s3.key_prefix().map(|s| s.to_string()),
                }),
            }
        }))
    }
}
