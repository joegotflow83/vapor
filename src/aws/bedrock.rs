use aws_config::SdkConfig;
use chrono::{DateTime, Utc};

use crate::error::VaporError;
use crate::schema::time::to_utc;

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
    pub creation_time: Option<DateTime<Utc>>,
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
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
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
            .map_err(crate::error::sdk_err)?;

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

    /// Lists custom Bedrock models. `limit` caps the total number of results
    /// across pages (default: unlimited); `next_token` resumes from a prior page.
    pub async fn list_custom_models(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<BedrockCustomModelInfo>, Option<String>), VaporError> {
        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_custom_models();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            summaries.extend(output.model_summaries.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let items = summaries
            .into_iter()
            .map(|m| BedrockCustomModelInfo {
                model_arn: Some(m.model_arn),
                model_name: Some(m.model_name),
                creation_time: to_utc(Some(&m.creation_time)),
                base_model_arn: Some(m.base_model_arn),
                customization_type: m.customization_type.map(|t| t.as_str().to_string()),
                job_arn: None,
            })
            .collect();

        Ok((items, token))
    }

    /// Lists Bedrock guardrails. `limit` caps the total number of results
    /// across pages (default: unlimited); `next_token` resumes from a prior page.
    pub async fn list_guardrails(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<BedrockGuardrailInfo>, Option<String>), VaporError> {
        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_guardrails();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            summaries.extend(output.guardrails);

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let items = summaries
            .into_iter()
            .map(|g| BedrockGuardrailInfo {
                guardrail_id: Some(g.id),
                guardrail_arn: Some(g.arn),
                name: Some(g.name),
                status: Some(g.status.as_str().to_string()),
                version: Some(g.version),
                created_at: to_utc(Some(&g.created_at)),
                updated_at: to_utc(Some(&g.updated_at)),
                description: g.description,
            })
            .collect();

        Ok((items, token))
    }

    pub async fn get_model_invocation_logging_config(
        &self,
    ) -> Result<Option<BedrockModelInvocationLoggingConfigInfo>, VaporError> {
        let output = self
            .inner
            .get_model_invocation_logging_configuration()
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const FOUNDATION_MODELS: &str = "https://bedrock.us-east-1.amazonaws.com/foundation-models";
    const CUSTOM_MODELS: &str = "https://bedrock.us-east-1.amazonaws.com/custom-models";
    const GUARDRAILS: &str = "https://bedrock.us-east-1.amazonaws.com/guardrails";
    const LOGGING_CONFIG: &str = "https://bedrock.us-east-1.amazonaws.com/logging/modelinvocations";

    #[tokio::test]
    async fn list_foundation_models_returns_all_with_no_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FOUNDATION_MODELS, ""),
            json_response(
                200,
                r#"{"modelSummaries":[{"modelArn":"arn:model1","modelId":"model1","modelName":"Model One","providerName":"Anthropic","inputModalities":["TEXT"],"outputModalities":["TEXT"],"responseStreamingSupported":true,"customizationsSupported":["FINE_TUNING"],"modelLifecycle":{"status":"ACTIVE"}},{"modelArn":"arn:model2","modelId":"model2"}]}"#,
            ),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let models = client.list_foundation_models(None, None, None).await.unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model_id, "model1");
        assert_eq!(models[0].model_name, Some("Model One".to_string()));
        assert_eq!(models[0].provider_name, Some("Anthropic".to_string()));
        assert_eq!(models[0].input_modalities, vec!["TEXT".to_string()]);
        assert_eq!(models[0].output_modalities, vec!["TEXT".to_string()]);
        assert_eq!(models[0].model_lifecycle_status, Some("ACTIVE".to_string()));
        assert_eq!(models[0].response_streaming_supported, Some(true));
        assert_eq!(models[0].customizations_supported, vec!["FINE_TUNING".to_string()]);
        assert_eq!(models[1].model_id, "model2");
        assert_eq!(models[1].model_lifecycle_status, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_foundation_models_passes_filters_in_codegen_order() {
        // Query order fixed by `uri_query` codegen: byProvider, byOutputModality,
        // byInferenceType (byCustomizationType is skipped — this wrapper never sets it).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{FOUNDATION_MODELS}?byProvider=Anthropic&byOutputModality=TEXT&byInferenceType=ON_DEMAND"),
                "",
            ),
            json_response(200, r#"{"modelSummaries":[]}"#),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let models = client
            .list_foundation_models(
                Some("Anthropic".to_string()),
                Some("TEXT".to_string()),
                Some("ON_DEMAND".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(models.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_foundation_models_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FOUNDATION_MODELS, ""),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        match client.list_foundation_models(None, None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            Ok(_) => panic!("expected VaporError::AwsSdk, got Ok"),
            Err(other) => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_custom_models_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(CUSTOM_MODELS, ""),
            json_response(
                200,
                r#"{"modelSummaries":[{"modelArn":"arn:cm1","modelName":"cm-one","creationTime":"2024-01-15T10:30:00Z","baseModelArn":"arn:base1","baseModelName":"base-one","customizationType":"FINE_TUNING"}]}"#,
            ),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let (models, token) = client.list_custom_models(None, None).await.unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_arn, Some("arn:cm1".to_string()));
        assert_eq!(models[0].model_name, Some("cm-one".to_string()));
        assert_eq!(
            models[0].creation_time,
            Some(DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::seconds(1705314600))
        );
        assert_eq!(models[0].base_model_arn, Some("arn:base1".to_string()));
        assert_eq!(models[0].customization_type, Some("FINE_TUNING".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_custom_models_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{CUSTOM_MODELS}?nextToken=cursor-a"), ""),
            json_response(
                200,
                r#"{"modelSummaries":[{"modelArn":"arn:cm2","modelName":"cm-two","creationTime":"2024-01-15T10:30:00Z","baseModelArn":"arn:base2","baseModelName":"base-two"}]}"#,
            ),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let (models, token) = client
            .list_custom_models(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_custom_models_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{CUSTOM_MODELS}?maxResults=2"), ""),
            json_response(
                200,
                r#"{"modelSummaries":[{"modelArn":"arn:cm1","modelName":"cm-one","creationTime":"2024-01-15T10:30:00Z","baseModelArn":"arn:base1","baseModelName":"base-one"},{"modelArn":"arn:cm2","modelName":"cm-two","creationTime":"2024-01-15T10:30:00Z","baseModelArn":"arn:base2","baseModelName":"base-two"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let (models, token) = client.list_custom_models(Some(2), None).await.unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_custom_models_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{CUSTOM_MODELS}?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"modelSummaries":[{"modelArn":"arn:cm1","modelName":"cm-one","creationTime":"2024-01-15T10:30:00Z","baseModelArn":"arn:base1","baseModelName":"base-one"},{"modelArn":"arn:cm2","modelName":"cm-two","creationTime":"2024-01-15T10:30:00Z","baseModelArn":"arn:base2","baseModelName":"base-two"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{CUSTOM_MODELS}?maxResults=8&nextToken=p2"), ""),
                json_response(
                    200,
                    r#"{"modelSummaries":[{"modelArn":"arn:cm3","modelName":"cm-three","creationTime":"2024-01-15T10:30:00Z","baseModelArn":"arn:base3","baseModelName":"base-three"}]}"#,
                ),
            ),
        ]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let (models, token) = client.list_custom_models(Some(10), None).await.unwrap();

        assert_eq!(models.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_custom_models_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(CUSTOM_MODELS, ""),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        match client.list_custom_models(None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            Ok(_) => panic!("expected VaporError::AwsSdk, got Ok"),
            Err(other) => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_guardrails_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(GUARDRAILS, ""),
            json_response(
                200,
                r#"{"guardrails":[{"id":"gr1","arn":"arn:gr1","status":"READY","name":"guardrail-one","version":"1","createdAt":"2024-01-15T10:30:00Z","updatedAt":"2024-01-16T10:30:00Z","description":"first guardrail"}]}"#,
            ),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let (guardrails, token) = client.list_guardrails(None, None).await.unwrap();

        assert_eq!(guardrails.len(), 1);
        assert_eq!(guardrails[0].guardrail_id, Some("gr1".to_string()));
        assert_eq!(guardrails[0].guardrail_arn, Some("arn:gr1".to_string()));
        assert_eq!(guardrails[0].name, Some("guardrail-one".to_string()));
        assert_eq!(guardrails[0].status, Some("READY".to_string()));
        assert_eq!(guardrails[0].version, Some("1".to_string()));
        assert_eq!(
            guardrails[0].created_at,
            Some(DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::seconds(1705314600))
        );
        assert_eq!(
            guardrails[0].updated_at,
            Some(DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::seconds(1705401000))
        );
        assert_eq!(guardrails[0].description, Some("first guardrail".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_guardrails_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{GUARDRAILS}?maxResults=1"), ""),
            json_response(
                200,
                r#"{"guardrails":[{"id":"gr1","arn":"arn:gr1","status":"READY","name":"guardrail-one","version":"1","createdAt":"2024-01-15T10:30:00Z","updatedAt":"2024-01-16T10:30:00Z"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let (guardrails, token) = client.list_guardrails(Some(1), None).await.unwrap();

        assert_eq!(guardrails.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_model_invocation_logging_config_returns_full_config() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LOGGING_CONFIG, ""),
            json_response(
                200,
                r#"{"loggingConfig":{"cloudWatchConfig":{"logGroupName":"/aws/bedrock","roleArn":"arn:role1","largeDataDeliveryS3Config":{"bucketName":"large-bucket","keyPrefix":"large/"}},"s3Config":{"bucketName":"log-bucket","keyPrefix":"logs/"},"textDataDeliveryEnabled":true}}"#,
            ),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let config = client.get_model_invocation_logging_config().await.unwrap().unwrap();

        let cw = config.cloudwatch_config.unwrap();
        assert_eq!(cw.log_group_name, Some("/aws/bedrock".to_string()));
        assert_eq!(cw.role_arn, Some("arn:role1".to_string()));
        let large_s3 = cw.large_data_delivery_s3_config.unwrap();
        assert_eq!(large_s3.bucket_name, Some("large-bucket".to_string()));
        assert_eq!(large_s3.key_prefix, Some("large/".to_string()));
        let s3 = config.s3_config.unwrap();
        assert_eq!(s3.bucket_name, Some("log-bucket".to_string()));
        assert_eq!(s3.key_prefix, Some("logs/".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_model_invocation_logging_config_returns_none_when_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LOGGING_CONFIG, ""),
            json_response(200, r#"{}"#),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        let config = client.get_model_invocation_logging_config().await.unwrap();

        assert!(config.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_model_invocation_logging_config_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LOGGING_CONFIG, ""),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = BedrockClient::new(&sdk_config(http_client.clone()));

        match client.get_model_invocation_logging_config().await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            Ok(_) => panic!("expected VaporError::AwsSdk, got Ok"),
            Err(other) => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

