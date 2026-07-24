#[cfg(feature = "lambda")]
use aws_config::SdkConfig;
#[cfg(feature = "lambda")]
use aws_sdk_lambda::types::{
    AliasConfiguration, EventSourceMappingConfiguration, FunctionConfiguration, LayersListItem,
};
#[cfg(feature = "lambda")]
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
#[cfg(feature = "lambda")]
use std::collections::HashMap;

#[cfg(feature = "lambda")]
use crate::error::VaporError;

#[cfg(feature = "lambda")]
pub struct LambdaClient {
    inner: aws_sdk_lambda::Client,
}

impl LambdaClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_lambda::Client::new(config),
        }
    }

    /// Lists Lambda functions, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `ListFunctionsInput::max_items` so a capped page boundary lands exactly
    /// on the returned `marker`, matching
    /// `specs/plan-2-schema-v2-pagination-timestamps.md`'s client-layer pattern.
    pub async fn list_functions(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<FunctionConfiguration>, Option<String>), VaporError> {
        let mut functions = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_functions();
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - functions.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for f in output.functions() {
                functions.push(f.clone());
            }
            token = output.next_marker().map(|s| s.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if functions.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((functions, token))
    }

    pub async fn list_aliases(
        &self,
        function_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AliasConfiguration>, Option<String>), VaporError> {
        let mut aliases = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_aliases().function_name(function_name);
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - aliases.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for a in output.aliases() {
                aliases.push(a.clone());
            }
            token = output.next_marker().map(|s| s.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if aliases.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((aliases, token))
    }

    pub async fn list_event_source_mappings(
        &self,
        function_name: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<EventSourceMappingConfiguration>, Option<String>), VaporError> {
        let mut mappings = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_event_source_mappings();
            if let Some(name) = function_name {
                req = req.function_name(name);
            }
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - mappings.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for m in output.event_source_mappings() {
                mappings.push(m.clone());
            }
            token = output.next_marker().map(|s| s.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if mappings.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((mappings, token))
    }

    pub async fn list_layers(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LayersListItem>, Option<String>), VaporError> {
        let mut layers = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_layers();
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - layers.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for l in output.layers() {
                layers.push(l.clone());
            }
            token = output.next_marker().map(|s| s.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if layers.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((layers, token))
    }

    /// Returns tags for a Lambda function (keyed by ARN).
    pub async fn list_tags(
        &self,
        function_arn: &str,
    ) -> Result<HashMap<String, String>, VaporError> {
        let output = self
            .inner
            .list_tags()
            .resource(function_arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.tags().cloned().unwrap_or_default())
    }

    /// Returns the resource-based policy JSON for a Lambda function.
    /// Returns None when no policy is attached (ResourceNotFoundException).
    pub async fn get_function_policy(
        &self,
        function_name: &str,
    ) -> Result<Option<String>, VaporError> {
        match self
            .inner
            .get_policy()
            .function_name(function_name)
            .send()
            .await
        {
            Ok(output) => Ok(output.policy().map(|s| s.to_string())),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_resource_not_found_exception() {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk {
                        code: svc_err.code().map(String::from),
                        message: svc_err
                            .message()
                            .map(String::from)
                            .unwrap_or_else(|| svc_err.to_string()),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::error::VaporError;

    const BASE: &str = "https://lambda.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_functions_lists_a_single_page() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions"), ""),
            json_response(
                200,
                r#"{"Functions":[{"FunctionName":"fn-1","FunctionArn":"arn:aws:lambda:us-east-1:111122223333:function:fn-1","CodeSize":1024,"LastModified":"2024-01-01T00:00:00.000+0000"}]}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (functions, token) = client.list_functions(None, None).await.unwrap();

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].function_name.as_deref(), Some("fn-1"));
        assert_eq!(
            functions[0].function_arn.as_deref(),
            Some("arn:aws:lambda:us-east-1:111122223333:function:fn-1")
        );
        assert_eq!(functions[0].code_size, 1024);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_functions_forwards_limit_to_aws_with_no_client_truncate() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions?MaxItems=2"), ""),
            json_response(
                200,
                r#"{"Functions":[{"FunctionName":"fn-1"},{"FunctionName":"fn-2"}],"NextMarker":"cursor-b"}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (functions, token) = client.list_functions(Some(2), None).await.unwrap();

        assert_eq!(functions.len(), 2);
        assert_eq!(token, Some("cursor-b".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_functions_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions?Marker=cursor-a"), ""),
            json_response(200, r#"{"Functions":[{"FunctionName":"fn-3"}]}"#),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (functions, token) = client
            .list_functions(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(functions.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_functions_propagates_errors() {
        // `InvalidParameterValueException`, not a throttling-classified code
        // (see memory gotcha: those get retried and exhaust the single
        // replay event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions"), ""),
            json_error_response("InvalidParameterValueException", "bad marker"),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let err = client.list_functions(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad marker");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_aliases_lists_and_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions/my-fn/aliases"), ""),
            json_response(
                200,
                r#"{"Aliases":[{"AliasArn":"arn:aws:lambda:us-east-1:111122223333:function:my-fn:live","Name":"live","FunctionVersion":"3","Description":"prod alias"}]}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (aliases, token) = client.list_aliases("my-fn", None, None).await.unwrap();

        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name.as_deref(), Some("live"));
        assert_eq!(aliases[0].function_version.as_deref(), Some("3"));
        assert_eq!(aliases[0].description.as_deref(), Some("prod alias"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_aliases_forwards_limit_to_aws_with_no_client_truncate() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2015-03-31/functions/my-fn/aliases?MaxItems=1"),
                "",
            ),
            json_response(
                200,
                r#"{"Aliases":[{"Name":"live"}],"NextMarker":"cursor-c"}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (aliases, token) = client.list_aliases("my-fn", Some(1), None).await.unwrap();

        assert_eq!(aliases.len(), 1);
        assert_eq!(token, Some("cursor-c".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_aliases_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions/my-fn/aliases"), ""),
            json_error_response("ResourceNotFoundException", "function not found"),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let err = client.list_aliases("my-fn", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "function not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_event_source_mappings_filters_by_function_name_and_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2015-03-31/event-source-mappings?FunctionName=my-fn"),
                "",
            ),
            json_response(
                200,
                r#"{"EventSourceMappings":[{"UUID":"uuid-1","EventSourceArn":"arn:aws:sqs:us-east-1:111122223333:queue","FunctionArn":"arn:aws:lambda:us-east-1:111122223333:function:my-fn","State":"Enabled","BatchSize":10}]}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (mappings, token) = client
            .list_event_source_mappings(Some("my-fn"), None, None)
            .await
            .unwrap();

        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].uuid.as_deref(), Some("uuid-1"));
        assert_eq!(
            mappings[0].event_source_arn.as_deref(),
            Some("arn:aws:sqs:us-east-1:111122223333:queue")
        );
        assert_eq!(mappings[0].state.as_deref(), Some("Enabled"));
        assert_eq!(mappings[0].batch_size, Some(10));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_event_source_mappings_forwards_limit_to_aws_with_no_client_truncate() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2015-03-31/event-source-mappings?MaxItems=1"),
                "",
            ),
            json_response(
                200,
                r#"{"EventSourceMappings":[{"UUID":"uuid-1"}],"NextMarker":"cursor-d"}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (mappings, token) = client
            .list_event_source_mappings(None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(mappings.len(), 1);
        assert_eq!(token, Some("cursor-d".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_event_source_mappings_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/event-source-mappings"), ""),
            json_error_response("InvalidParameterValueException", "bad event source arn"),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_event_source_mappings(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad event source arn");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_layers_lists_and_maps_nested_latest_matching_version() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2018-10-31/layers"), ""),
            json_response(
                200,
                r#"{"Layers":[{"LayerName":"my-layer","LayerArn":"arn:aws:lambda:us-east-1:111122223333:layer:my-layer","LatestMatchingVersion":{"LayerVersionArn":"arn:aws:lambda:us-east-1:111122223333:layer:my-layer:2","Version":2,"Description":"v2"}}]}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (layers, token) = client.list_layers(None, None).await.unwrap();

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].layer_name(), Some("my-layer"));
        let latest = layers[0].latest_matching_version().unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.description.as_deref(), Some("v2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_layers_forwards_limit_to_aws_with_no_client_truncate() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2018-10-31/layers?MaxItems=1"), ""),
            json_response(
                200,
                r#"{"Layers":[{"LayerName":"my-layer"}],"NextMarker":"cursor-e"}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let (layers, token) = client.list_layers(Some(1), None).await.unwrap();

        assert_eq!(layers.len(), 1);
        assert_eq!(token, Some("cursor-e".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_layers_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2018-10-31/layers"), ""),
            json_error_response("ServiceException", "internal error"),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let err = client.list_layers(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ServiceException".to_string()));
                assert_eq!(message, "internal error");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_returns_tag_map() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!(
                    "{BASE}/2017-03-31/tags/arn%3Aaws%3Alambda%3Aus-east-1%3A111122223333%3Afunction%3Amy-fn"
                ),
                "",
            ),
            json_response(200, r#"{"Tags":{"env":"prod","team":"platform"}}"#),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let tags = client
            .list_tags("arn:aws:lambda:us-east-1:111122223333:function:my-fn")
            .await
            .unwrap();

        assert_eq!(tags.get("env"), Some(&"prod".to_string()));
        assert_eq!(tags.get("team"), Some(&"platform".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2017-03-31/tags/my-fn-arn"), ""),
            json_error_response("ResourceNotFoundException", "function not found"),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let err = client.list_tags("my-fn-arn").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "function not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_function_policy_returns_policy() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions/my-fn/policy"), ""),
            json_response(
                200,
                r#"{"Policy":"{\"Version\":\"2012-10-17\"}","RevisionId":"rev-1"}"#,
            ),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let policy = client.get_function_policy("my-fn").await.unwrap();

        assert_eq!(policy.as_deref(), Some(r#"{"Version":"2012-10-17"}"#));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_function_policy_returns_none_when_resource_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions/my-fn/policy"), ""),
            json_error_response("ResourceNotFoundException", "no policy attached"),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let policy = client.get_function_policy("my-fn").await.unwrap();

        assert_eq!(policy, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_function_policy_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions/my-fn/policy"), ""),
            json_error_response("InvalidParameterValueException", "bad function name"),
        )]);
        let client = LambdaClient::new(&sdk_config(http_client.clone()));

        let err = client.get_function_policy("my-fn").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad function name");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
