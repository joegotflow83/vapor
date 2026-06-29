use async_graphql::SimpleObject;

use crate::aws::bedrock::{
    BedrockCloudWatchConfigInfo, BedrockCustomModelInfo, BedrockFoundationModelInfo,
    BedrockGuardrailInfo, BedrockModelInvocationLoggingConfigInfo, BedrockS3ConfigInfo,
};

#[derive(SimpleObject, Clone)]
pub struct BedrockFoundationModel {
    pub model_id: String,
    pub model_name: Option<String>,
    pub provider_name: Option<String>,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    pub model_lifecycle_status: Option<String>,
    pub response_streaming_supported: Option<bool>,
    pub customizations_supported: Vec<String>,
}

impl From<BedrockFoundationModelInfo> for BedrockFoundationModel {
    fn from(m: BedrockFoundationModelInfo) -> Self {
        Self {
            model_id: m.model_id,
            model_name: m.model_name,
            provider_name: m.provider_name,
            input_modalities: m.input_modalities,
            output_modalities: m.output_modalities,
            model_lifecycle_status: m.model_lifecycle_status,
            response_streaming_supported: m.response_streaming_supported,
            customizations_supported: m.customizations_supported,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BedrockCustomModel {
    pub model_arn: Option<String>,
    pub model_name: Option<String>,
    pub creation_time: Option<String>,
    pub base_model_arn: Option<String>,
    pub customization_type: Option<String>,
    pub job_arn: Option<String>,
}

impl From<BedrockCustomModelInfo> for BedrockCustomModel {
    fn from(m: BedrockCustomModelInfo) -> Self {
        Self {
            model_arn: m.model_arn,
            model_name: m.model_name,
            creation_time: m.creation_time,
            base_model_arn: m.base_model_arn,
            customization_type: m.customization_type,
            job_arn: m.job_arn,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BedrockGuardrail {
    pub guardrail_id: Option<String>,
    pub guardrail_arn: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub version: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub description: Option<String>,
}

impl From<BedrockGuardrailInfo> for BedrockGuardrail {
    fn from(g: BedrockGuardrailInfo) -> Self {
        Self {
            guardrail_id: g.guardrail_id,
            guardrail_arn: g.guardrail_arn,
            name: g.name,
            status: g.status,
            version: g.version,
            created_at: g.created_at,
            updated_at: g.updated_at,
            description: g.description,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BedrockS3Config {
    pub bucket_name: Option<String>,
    pub key_prefix: Option<String>,
}

impl From<BedrockS3ConfigInfo> for BedrockS3Config {
    fn from(s: BedrockS3ConfigInfo) -> Self {
        Self {
            bucket_name: s.bucket_name,
            key_prefix: s.key_prefix,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BedrockCloudWatchConfig {
    pub log_group_name: Option<String>,
    pub role_arn: Option<String>,
    pub large_data_delivery_s3_config: Option<BedrockS3Config>,
}

impl From<BedrockCloudWatchConfigInfo> for BedrockCloudWatchConfig {
    fn from(cw: BedrockCloudWatchConfigInfo) -> Self {
        Self {
            log_group_name: cw.log_group_name,
            role_arn: cw.role_arn,
            large_data_delivery_s3_config: cw
                .large_data_delivery_s3_config
                .map(BedrockS3Config::from),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BedrockModelInvocationLoggingConfig {
    pub cloudwatch_config: Option<BedrockCloudWatchConfig>,
    pub s3_config: Option<BedrockS3Config>,
}

impl From<BedrockModelInvocationLoggingConfigInfo> for BedrockModelInvocationLoggingConfig {
    fn from(lc: BedrockModelInvocationLoggingConfigInfo) -> Self {
        Self {
            cloudwatch_config: lc.cloudwatch_config.map(BedrockCloudWatchConfig::from),
            s3_config: lc.s3_config.map(BedrockS3Config::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::bedrock::{
        BedrockCloudWatchConfigInfo, BedrockCustomModelInfo, BedrockFoundationModelInfo,
        BedrockGuardrailInfo, BedrockModelInvocationLoggingConfigInfo, BedrockS3ConfigInfo,
    };

    #[test]
    fn test_foundation_model_from_full() {
        let info = BedrockFoundationModelInfo {
            model_id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            model_name: Some("Claude 3 Sonnet".to_string()),
            provider_name: Some("Anthropic".to_string()),
            input_modalities: vec!["TEXT".to_string(), "IMAGE".to_string()],
            output_modalities: vec!["TEXT".to_string()],
            model_lifecycle_status: Some("ACTIVE".to_string()),
            response_streaming_supported: Some(true),
            customizations_supported: vec!["FINE_TUNING".to_string()],
        };
        let result = BedrockFoundationModel::from(info);
        assert_eq!(result.model_id, "anthropic.claude-3-sonnet-20240229-v1:0");
        assert_eq!(result.model_name, Some("Claude 3 Sonnet".to_string()));
        assert_eq!(result.provider_name, Some("Anthropic".to_string()));
        assert_eq!(result.input_modalities.len(), 2);
        assert_eq!(result.output_modalities.len(), 1);
        assert_eq!(result.model_lifecycle_status, Some("ACTIVE".to_string()));
        assert_eq!(result.response_streaming_supported, Some(true));
        assert_eq!(result.customizations_supported.len(), 1);
    }

    #[test]
    fn test_foundation_model_from_minimal() {
        let info = BedrockFoundationModelInfo {
            model_id: "amazon.titan-text-g1-lite-v1".to_string(),
            model_name: None,
            provider_name: None,
            input_modalities: vec![],
            output_modalities: vec![],
            model_lifecycle_status: None,
            response_streaming_supported: None,
            customizations_supported: vec![],
        };
        let result = BedrockFoundationModel::from(info);
        assert_eq!(result.model_id, "amazon.titan-text-g1-lite-v1");
        assert!(result.model_name.is_none());
        assert!(result.provider_name.is_none());
        assert!(result.input_modalities.is_empty());
        assert!(result.response_streaming_supported.is_none());
    }

    #[test]
    fn test_custom_model_from_full() {
        let info = BedrockCustomModelInfo {
            model_arn: Some("arn:aws:bedrock:us-east-1:123456789012:custom-model/my-model".to_string()),
            model_name: Some("my-fine-tuned-model".to_string()),
            creation_time: Some("2024-01-15T10:30:00Z".to_string()),
            base_model_arn: Some("arn:aws:bedrock:us-east-1::foundation-model/amazon.titan-text-lite-v1".to_string()),
            customization_type: Some("FINE_TUNING".to_string()),
            job_arn: Some("arn:aws:bedrock:us-east-1:123456789012:model-customization-job/abc123".to_string()),
        };
        let result = BedrockCustomModel::from(info);
        assert!(result.model_arn.is_some());
        assert_eq!(result.model_name, Some("my-fine-tuned-model".to_string()));
        assert_eq!(result.customization_type, Some("FINE_TUNING".to_string()));
        assert!(result.job_arn.is_some());
    }

    #[test]
    fn test_custom_model_from_minimal() {
        let info = BedrockCustomModelInfo {
            model_arn: None,
            model_name: None,
            creation_time: None,
            base_model_arn: None,
            customization_type: None,
            job_arn: None,
        };
        let result = BedrockCustomModel::from(info);
        assert!(result.model_arn.is_none());
        assert!(result.model_name.is_none());
        assert!(result.creation_time.is_none());
        assert!(result.customization_type.is_none());
    }

    #[test]
    fn test_guardrail_from_full() {
        let info = BedrockGuardrailInfo {
            guardrail_id: Some("abc123def456".to_string()),
            guardrail_arn: Some("arn:aws:bedrock:us-east-1:123456789012:guardrail/abc123def456".to_string()),
            name: Some("my-guardrail".to_string()),
            status: Some("READY".to_string()),
            version: Some("DRAFT".to_string()),
            created_at: Some("2024-01-15T10:30:00Z".to_string()),
            updated_at: Some("2024-01-16T12:00:00Z".to_string()),
            description: Some("Content filtering guardrail".to_string()),
        };
        let result = BedrockGuardrail::from(info);
        assert_eq!(result.guardrail_id, Some("abc123def456".to_string()));
        assert_eq!(result.name, Some("my-guardrail".to_string()));
        assert_eq!(result.status, Some("READY".to_string()));
        assert_eq!(result.description, Some("Content filtering guardrail".to_string()));
    }

    #[test]
    fn test_guardrail_from_no_description() {
        let info = BedrockGuardrailInfo {
            guardrail_id: Some("xyz789".to_string()),
            guardrail_arn: Some("arn:aws:bedrock:us-east-1:123456789012:guardrail/xyz789".to_string()),
            name: Some("basic-guardrail".to_string()),
            status: Some("CREATING".to_string()),
            version: Some("DRAFT".to_string()),
            created_at: Some("2024-01-15T10:30:00Z".to_string()),
            updated_at: Some("2024-01-15T10:30:00Z".to_string()),
            description: None,
        };
        let result = BedrockGuardrail::from(info);
        assert!(result.description.is_none());
        assert_eq!(result.status, Some("CREATING".to_string()));
    }

    #[test]
    fn test_s3_config_from() {
        let info = BedrockS3ConfigInfo {
            bucket_name: Some("my-logging-bucket".to_string()),
            key_prefix: Some("bedrock-logs/".to_string()),
        };
        let result = BedrockS3Config::from(info);
        assert_eq!(result.bucket_name, Some("my-logging-bucket".to_string()));
        assert_eq!(result.key_prefix, Some("bedrock-logs/".to_string()));
    }

    #[test]
    fn test_s3_config_no_prefix() {
        let info = BedrockS3ConfigInfo {
            bucket_name: Some("my-bucket".to_string()),
            key_prefix: None,
        };
        let result = BedrockS3Config::from(info);
        assert_eq!(result.bucket_name, Some("my-bucket".to_string()));
        assert!(result.key_prefix.is_none());
    }

    #[test]
    fn test_cloudwatch_config_from() {
        let info = BedrockCloudWatchConfigInfo {
            log_group_name: Some("/aws/bedrock/invocations".to_string()),
            role_arn: Some("arn:aws:iam::123456789012:role/BedrockLoggingRole".to_string()),
            large_data_delivery_s3_config: Some(BedrockS3ConfigInfo {
                bucket_name: Some("large-data-bucket".to_string()),
                key_prefix: Some("large-data/".to_string()),
            }),
        };
        let result = BedrockCloudWatchConfig::from(info);
        assert_eq!(result.log_group_name, Some("/aws/bedrock/invocations".to_string()));
        assert!(result.role_arn.is_some());
        assert!(result.large_data_delivery_s3_config.is_some());
    }

    #[test]
    fn test_cloudwatch_config_no_s3() {
        let info = BedrockCloudWatchConfigInfo {
            log_group_name: Some("/aws/bedrock/invocations".to_string()),
            role_arn: Some("arn:aws:iam::123456789012:role/BedrockLoggingRole".to_string()),
            large_data_delivery_s3_config: None,
        };
        let result = BedrockCloudWatchConfig::from(info);
        assert!(result.large_data_delivery_s3_config.is_none());
    }

    #[test]
    fn test_logging_config_from_full() {
        let info = BedrockModelInvocationLoggingConfigInfo {
            cloudwatch_config: Some(BedrockCloudWatchConfigInfo {
                log_group_name: Some("/aws/bedrock/invocations".to_string()),
                role_arn: Some("arn:aws:iam::123456789012:role/BedrockLoggingRole".to_string()),
                large_data_delivery_s3_config: None,
            }),
            s3_config: Some(BedrockS3ConfigInfo {
                bucket_name: Some("bedrock-logs".to_string()),
                key_prefix: None,
            }),
        };
        let result = BedrockModelInvocationLoggingConfig::from(info);
        assert!(result.cloudwatch_config.is_some());
        assert!(result.s3_config.is_some());
    }

    #[test]
    fn test_logging_config_from_none() {
        let info = BedrockModelInvocationLoggingConfigInfo {
            cloudwatch_config: None,
            s3_config: None,
        };
        let result = BedrockModelInvocationLoggingConfig::from(info);
        assert!(result.cloudwatch_config.is_none());
        assert!(result.s3_config.is_none());
    }
}
