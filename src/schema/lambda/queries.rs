use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::lambda::LambdaClient;
use crate::schema::lambda::types::{
    LambdaAlias, LambdaEventSourceMapping, LambdaFunction, LambdaLayer,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct LambdaQuery;

#[Object]
impl LambdaQuery {
    /// List all Lambda functions with metadata. Environment variable values are
    /// intentionally omitted — only key names are returned. `limit` caps the
    /// total number of results returned, default unlimited.
    async fn lambda_functions(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LambdaFunction>> {
        let lambda = ctx.data::<LambdaClient>()?;
        let (configs, next_token) = lambda.list_functions(limit, next_token).await?;

        let futures: Vec<_> = configs
            .into_iter()
            .map(|cfg| async move {
                let tags = if let Some(arn) = cfg.function_arn() {
                    lambda.list_tags(arn).await.unwrap_or_default()
                } else {
                    std::collections::HashMap::new()
                };
                LambdaFunction::from_config_and_tags(cfg, tags)
            })
            .collect();

        Ok(Page {
            items: join_all(futures).await,
            next_token,
        })
    }

    /// List aliases for a Lambda function. `limit` caps the total number of
    /// results returned, default unlimited.
    async fn lambda_aliases(
        &self,
        ctx: &Context<'_>,
        function_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LambdaAlias>> {
        let lambda = ctx.data::<LambdaClient>()?;
        let (aliases, next_token) = lambda
            .list_aliases(&function_name, limit, next_token)
            .await?;
        Ok(Page {
            items: aliases.into_iter().map(LambdaAlias::from).collect(),
            next_token,
        })
    }

    /// List event source mappings. If functionName is provided, filter by that
    /// function. `limit` caps the total number of results returned, default
    /// unlimited.
    async fn lambda_event_source_mappings(
        &self,
        ctx: &Context<'_>,
        function_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LambdaEventSourceMapping>> {
        let lambda = ctx.data::<LambdaClient>()?;
        let (mappings, next_token) = lambda
            .list_event_source_mappings(function_name.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: mappings
                .into_iter()
                .map(LambdaEventSourceMapping::from)
                .collect(),
            next_token,
        })
    }

    /// List all Lambda layers with their latest published version metadata.
    /// `limit` caps the total number of results returned, default unlimited.
    async fn lambda_layers(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LambdaLayer>> {
        let lambda = ctx.data::<LambdaClient>()?;
        let (layers, next_token) = lambda.list_layers(limit, next_token).await?;
        Ok(Page {
            items: layers.into_iter().map(LambdaLayer::from).collect(),
            next_token,
        })
    }

    /// Fetch the resource-based policy document for a Lambda function.
    /// Returns the raw JSON policy string, or null if no policy is attached.
    /// Reveals which principals (AWS services, accounts, organizations) have
    /// permission to invoke the function — essential for detecting unintended
    /// cross-account access or public invocability.
    async fn lambda_function_policy(
        &self,
        ctx: &Context<'_>,
        function_name: String,
    ) -> Result<Option<String>> {
        let lambda = ctx.data::<LambdaClient>()?;
        Ok(lambda.get_function_policy(&function_name).await?)
    }
}

// `lambda_functions` has real logic beyond a bare passthrough: a per-item
// `list_tags` fan-out (resolver-local, not inside `LambdaClient`) whose
// errors are silently swallowed via `unwrap_or_default()`, and a branch that
// skips the tag call entirely when `function_arn()` is `None` — both earn
// dedicated tests (acm/cognito fan-out precedent). The other three list
// resolvers plus `lambda_function_policy` are 1:1 passthroughs to
// already-tested `LambdaClient` methods (see `src/aws/lambda.rs`'s own test
// module) and get one light smoke test each (connect/codeartifact
// precedent).
#[cfg(test)]
mod tests {
    use crate::aws::lambda::LambdaClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::LambdaQuery;

    const BASE: &str = "https://lambda.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn lambda_functions_maps_items_and_fans_out_tags() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/2015-03-31/functions?MaxItems=1"), ""),
                json_response(
                    200,
                    r#"{"Functions":[{"FunctionName":"fn-1","FunctionArn":"arn:aws:lambda:us-east-1:111122223333:function:fn-1","Runtime":"python3.12","CodeSize":2048,"State":"Active","Architectures":["arm64"]}],"NextMarker":"cursor-a"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!(
                        "{BASE}/2017-03-31/tags/arn%3Aaws%3Alambda%3Aus-east-1%3A111122223333%3Afunction%3Afn-1"
                    ),
                    "",
                ),
                json_response(200, r#"{"Tags":{"env":"prod"}}"#),
            ),
        ]);
        let schema = build_query_schema(LambdaQuery)
            .data(LambdaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ lambdaFunctions(limit: 1) { items { functionName functionArn runtime codeSize state architecture tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["lambdaFunctions"]["items"];
        assert_eq!(items[0]["functionName"], "fn-1");
        assert_eq!(
            items[0]["functionArn"],
            "arn:aws:lambda:us-east-1:111122223333:function:fn-1"
        );
        assert_eq!(items[0]["runtime"], "python3.12");
        assert_eq!(items[0]["codeSize"], 2048);
        assert_eq!(items[0]["state"], "Active");
        assert_eq!(items[0]["architecture"], "arm64");
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["lambdaFunctions"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lambda_functions_swallows_tag_fetch_errors() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/2015-03-31/functions?MaxItems=1"), ""),
                json_response(
                    200,
                    r#"{"Functions":[{"FunctionName":"fn-1","FunctionArn":"arn:aws:lambda:us-east-1:111122223333:function:fn-1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!(
                        "{BASE}/2017-03-31/tags/arn%3Aaws%3Alambda%3Aus-east-1%3A111122223333%3Afunction%3Afn-1"
                    ),
                    "",
                ),
                json_error_response("ResourceNotFoundException", "function not found"),
            ),
        ]);
        let schema = build_query_schema(LambdaQuery)
            .data(LambdaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ lambdaFunctions(limit: 1) { items { functionName tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["lambdaFunctions"]["items"];
        assert_eq!(items[0]["functionName"], "fn-1");
        assert_eq!(items[0]["tags"].as_array().unwrap().len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lambda_functions_skips_tag_fetch_when_arn_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions?MaxItems=1"), ""),
            json_response(200, r#"{"Functions":[{"FunctionName":"fn-1"}]}"#),
        )]);
        let schema = build_query_schema(LambdaQuery)
            .data(LambdaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ lambdaFunctions(limit: 1) { items { functionName functionArn tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["lambdaFunctions"]["items"];
        assert_eq!(items[0]["functionName"], "fn-1");
        assert!(items[0]["functionArn"].is_null());
        assert_eq!(items[0]["tags"].as_array().unwrap().len(), 0);
        // Only 1 ReplayEvent was queued (list only) — if the resolver called
        // list_tags despite the missing arn, relaxed_requests_match below
        // would fail with "no more test data available".
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lambda_aliases_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2015-03-31/functions/my-fn/aliases?MaxItems=1"),
                "",
            ),
            json_response(
                200,
                r#"{"Aliases":[{"AliasArn":"arn:aws:lambda:us-east-1:111122223333:function:my-fn:live","Name":"live","FunctionVersion":"3","Description":"prod alias"}],"NextMarker":"cursor-b"}"#,
            ),
        )]);
        let schema = build_query_schema(LambdaQuery)
            .data(LambdaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ lambdaAliases(functionName: "my-fn", limit: 1) { items { name aliasArn functionVersion description } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["lambdaAliases"]["items"];
        assert_eq!(items[0]["name"], "live");
        assert_eq!(
            items[0]["aliasArn"],
            "arn:aws:lambda:us-east-1:111122223333:function:my-fn:live"
        );
        assert_eq!(items[0]["functionVersion"], "3");
        assert_eq!(items[0]["description"], "prod alias");
        assert_eq!(json["lambdaAliases"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lambda_event_source_mappings_filters_by_function_name() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2015-03-31/event-source-mappings?FunctionName=my-fn&MaxItems=1"),
                "",
            ),
            json_response(
                200,
                r#"{"EventSourceMappings":[{"UUID":"uuid-1","EventSourceArn":"arn:aws:sqs:us-east-1:111122223333:queue","FunctionArn":"arn:aws:lambda:us-east-1:111122223333:function:my-fn","State":"Enabled","BatchSize":10,"LastModified":1700000000}],"NextMarker":"cursor-c"}"#,
            ),
        )]);
        let schema = build_query_schema(LambdaQuery)
            .data(LambdaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ lambdaEventSourceMappings(functionName: "my-fn", limit: 1) { items { uuid eventSourceArn functionArn state batchSize lastModified } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["lambdaEventSourceMappings"]["items"];
        assert_eq!(items[0]["uuid"], "uuid-1");
        assert_eq!(
            items[0]["eventSourceArn"],
            "arn:aws:sqs:us-east-1:111122223333:queue"
        );
        assert_eq!(items[0]["state"], "Enabled");
        assert_eq!(items[0]["batchSize"], 10);
        assert_eq!(items[0]["lastModified"], "2023-11-14T22:13:20+00:00");
        assert_eq!(json["lambdaEventSourceMappings"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lambda_layers_maps_nested_latest_matching_version() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2018-10-31/layers?MaxItems=1"), ""),
            json_response(
                200,
                r#"{"Layers":[{"LayerName":"my-layer","LayerArn":"arn:aws:lambda:us-east-1:111122223333:layer:my-layer","LatestMatchingVersion":{"LayerVersionArn":"arn:aws:lambda:us-east-1:111122223333:layer:my-layer:2","Version":2,"Description":"v2"}}],"NextMarker":"cursor-d"}"#,
            ),
        )]);
        let schema = build_query_schema(LambdaQuery)
            .data(LambdaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ lambdaLayers(limit: 1) { items { layerName layerArn latestMatchingVersion { version description } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["lambdaLayers"]["items"];
        assert_eq!(items[0]["layerName"], "my-layer");
        assert_eq!(
            items[0]["layerArn"],
            "arn:aws:lambda:us-east-1:111122223333:layer:my-layer"
        );
        assert_eq!(items[0]["latestMatchingVersion"]["version"], 2);
        assert_eq!(items[0]["latestMatchingVersion"]["description"], "v2");
        assert_eq!(json["lambdaLayers"]["nextToken"], "cursor-d");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lambda_function_policy_returns_policy_or_none() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2015-03-31/functions/my-fn/policy"), ""),
            json_response(
                200,
                r#"{"Policy":"{\"Version\":\"2012-10-17\"}","RevisionId":"rev-1"}"#,
            ),
        )]);
        let schema = build_query_schema(LambdaQuery)
            .data(LambdaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ lambdaFunctionPolicy(functionName: "my-fn") }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["lambdaFunctionPolicy"], r#"{"Version":"2012-10-17"}"#);
        http_client.relaxed_requests_match();
    }
}
