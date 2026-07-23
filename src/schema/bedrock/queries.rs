use async_graphql::{Context, Object, Result};

use crate::aws::bedrock::BedrockClient;
use crate::schema::bedrock::types::{
    BedrockCustomModel, BedrockFoundationModel, BedrockGuardrail,
    BedrockModelInvocationLoggingConfig,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct BedrockQuery;

#[Object]
impl BedrockQuery {
    async fn bedrock_foundation_models(
        &self,
        ctx: &Context<'_>,
        provider: Option<String>,
        by_output_modality: Option<String>,
        by_inference_type: Option<String>,
    ) -> Result<Vec<BedrockFoundationModel>> {
        let client = ctx.data::<BedrockClient>()?;
        let models = client
            .list_foundation_models(provider, by_output_modality, by_inference_type)
            .await?;
        Ok(models.into_iter().map(BedrockFoundationModel::from).collect())
    }

    /// List custom Bedrock models. `limit` caps the number of results
    /// returned (default: unlimited); `next_token` resumes from a prior page.
    async fn bedrock_custom_models(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BedrockCustomModel>> {
        let client = ctx.data::<BedrockClient>()?;
        let (items, next_token) = client.list_custom_models(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(BedrockCustomModel::from).collect(),
            next_token,
        })
    }

    /// List Bedrock guardrails. `limit` caps the number of results returned
    /// (default: unlimited); `next_token` resumes from a prior page.
    async fn bedrock_guardrails(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BedrockGuardrail>> {
        let client = ctx.data::<BedrockClient>()?;
        let (items, next_token) = client.list_guardrails(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(BedrockGuardrail::from).collect(),
            next_token,
        })
    }

    async fn bedrock_model_invocation_logging_config(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<BedrockModelInvocationLoggingConfig>> {
        let client = ctx.data::<BedrockClient>()?;
        let config = client.get_model_invocation_logging_config().await?;
        Ok(config.map(BedrockModelInvocationLoggingConfig::from))
    }
}

// All four resolvers are 1:1 passthroughs to a single already-tested
// `BedrockClient` method each (see `src/aws/bedrock.rs`'s own test module for
// the pagination/limit/filter/error-mapping behavior) — only light smoke
// tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::bedrock::BedrockClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::BedrockQuery;

    const FOUNDATION_MODELS: &str = "https://bedrock.us-east-1.amazonaws.com/foundation-models";
    const CUSTOM_MODELS: &str = "https://bedrock.us-east-1.amazonaws.com/custom-models";
    const GUARDRAILS: &str = "https://bedrock.us-east-1.amazonaws.com/guardrails";
    const LOGGING_CONFIG: &str = "https://bedrock.us-east-1.amazonaws.com/logging/modelinvocations";

    #[tokio::test]
    async fn bedrock_foundation_models_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FOUNDATION_MODELS, ""),
            json_response(
                200,
                r#"{"modelSummaries":[{"modelArn":"arn:model1","modelId":"model1","modelName":"Model One","providerName":"Anthropic","inputModalities":["TEXT"],"outputModalities":["TEXT"],"responseStreamingSupported":true,"customizationsSupported":["FINE_TUNING"],"modelLifecycle":{"status":"ACTIVE"}}]}"#,
            ),
        )]);
        let schema = build_query_schema(BedrockQuery)
            .data(BedrockClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ bedrockFoundationModels { modelId modelName providerName inputModalities outputModalities modelLifecycleStatus responseStreamingSupported customizationsSupported } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["bedrockFoundationModels"];
        assert_eq!(items[0]["modelId"], "model1");
        assert_eq!(items[0]["modelName"], "Model One");
        assert_eq!(items[0]["providerName"], "Anthropic");
        assert_eq!(items[0]["modelLifecycleStatus"], "ACTIVE");
        assert_eq!(items[0]["responseStreamingSupported"], true);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn bedrock_custom_models_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{CUSTOM_MODELS}?maxResults=1"), ""),
            json_response(
                200,
                r#"{"modelSummaries":[{"modelArn":"arn:cm1","modelName":"cm-one","creationTime":"2024-01-15T10:30:00Z","baseModelArn":"arn:base1","baseModelName":"base-one","customizationType":"FINE_TUNING"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(BedrockQuery)
            .data(BedrockClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ bedrockCustomModels(limit: 1) { items { modelArn modelName baseModelArn customizationType } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["bedrockCustomModels"]["items"];
        assert_eq!(items[0]["modelArn"], "arn:cm1");
        assert_eq!(items[0]["modelName"], "cm-one");
        assert_eq!(items[0]["customizationType"], "FINE_TUNING");
        assert_eq!(json["bedrockCustomModels"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn bedrock_guardrails_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{GUARDRAILS}?maxResults=1"), ""),
            json_response(
                200,
                r#"{"guardrails":[{"id":"gr1","arn":"arn:gr1","status":"READY","name":"guardrail-one","version":"1","createdAt":"2024-01-15T10:30:00Z","updatedAt":"2024-01-16T10:30:00Z","description":"first guardrail"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(BedrockQuery)
            .data(BedrockClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ bedrockGuardrails(limit: 1) { items { guardrailId guardrailArn name status version description } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["bedrockGuardrails"]["items"];
        assert_eq!(items[0]["guardrailId"], "gr1");
        assert_eq!(items[0]["guardrailArn"], "arn:gr1");
        assert_eq!(items[0]["status"], "READY");
        assert_eq!(items[0]["description"], "first guardrail");
        assert_eq!(json["bedrockGuardrails"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn bedrock_model_invocation_logging_config_maps_nested_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LOGGING_CONFIG, ""),
            json_response(
                200,
                r#"{"loggingConfig":{"cloudWatchConfig":{"logGroupName":"/aws/bedrock","roleArn":"arn:role1","largeDataDeliveryS3Config":{"bucketName":"large-bucket","keyPrefix":"large/"}},"s3Config":{"bucketName":"log-bucket","keyPrefix":"logs/"},"textDataDeliveryEnabled":true}}"#,
            ),
        )]);
        let schema = build_query_schema(BedrockQuery)
            .data(BedrockClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ bedrockModelInvocationLoggingConfig { cloudwatchConfig { logGroupName roleArn largeDataDeliveryS3Config { bucketName keyPrefix } } s3Config { bucketName keyPrefix } } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let config = &json["bedrockModelInvocationLoggingConfig"];
        assert_eq!(config["cloudwatchConfig"]["logGroupName"], "/aws/bedrock");
        assert_eq!(config["cloudwatchConfig"]["roleArn"], "arn:role1");
        assert_eq!(
            config["cloudwatchConfig"]["largeDataDeliveryS3Config"]["bucketName"],
            "large-bucket"
        );
        assert_eq!(config["s3Config"]["bucketName"], "log-bucket");
        assert_eq!(config["s3Config"]["keyPrefix"], "logs/");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn bedrock_model_invocation_logging_config_returns_null_when_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LOGGING_CONFIG, ""),
            json_response(200, r#"{}"#),
        )]);
        let schema = build_query_schema(BedrockQuery)
            .data(BedrockClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ bedrockModelInvocationLoggingConfig { s3Config { bucketName } } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["bedrockModelInvocationLoggingConfig"].is_null());
        http_client.relaxed_requests_match();
    }
}
