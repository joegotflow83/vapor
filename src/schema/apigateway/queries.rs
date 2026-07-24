use async_graphql::{Context, Object, Result};

use crate::aws::apigateway::ApiGatewayClient;
use crate::schema::apigateway::types::{
    ApigwDeployment, ApigwResource, ApigwRestApi, ApigwRestStage,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct ApiGatewayQuery;

#[Object]
impl ApiGatewayQuery {
    /// List all REST APIs (v1) in the region, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn apigw_rest_apis(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ApigwRestApi>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let (apis, next_token) = client.list_rest_apis(limit, next_token).await?;
        Ok(Page {
            items: apis.into_iter().map(ApigwRestApi::from).collect(),
            next_token,
        })
    }

    /// List all stages for the given REST API (v1). `GetStages` has no
    /// pagination at all, so this always returns every stage in one call.
    async fn apigw_rest_stages(
        &self,
        ctx: &Context<'_>,
        api_id: String,
    ) -> Result<Vec<ApigwRestStage>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let stages = client.list_rest_stages(&api_id).await?;
        Ok(stages.into_iter().map(ApigwRestStage::from).collect())
    }

    /// List all resources (path nodes) for the given REST API (v1),
    /// optionally capped at `limit` results (default unlimited) and resumed
    /// from `next_token`.
    async fn apigw_rest_resources(
        &self,
        ctx: &Context<'_>,
        api_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ApigwResource>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let (resources, next_token) = client
            .list_rest_resources(&api_id, limit, next_token)
            .await?;
        Ok(Page {
            items: resources.into_iter().map(ApigwResource::from).collect(),
            next_token,
        })
    }

    /// List all deployments for the given REST API (v1), optionally capped
    /// at `limit` results (default unlimited) and resumed from
    /// `next_token`.
    async fn apigw_rest_deployments(
        &self,
        ctx: &Context<'_>,
        api_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ApigwDeployment>> {
        let client = ctx.data::<ApiGatewayClient>()?;
        let (deployments, next_token) = client
            .list_rest_deployments(&api_id, limit, next_token)
            .await?;
        Ok(Page {
            items: deployments.into_iter().map(ApigwDeployment::from).collect(),
            next_token,
        })
    }
}

// All four resolvers are 1:1 passthroughs to a single already-tested
// `ApiGatewayClient` method each (see `src/aws/apigateway.rs`'s own test
// module for the pagination/limit/error-mapping behavior) — only light
// smoke tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::apigateway::ApiGatewayClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::ApiGatewayQuery;

    const APIS: &str = "https://apigateway.us-east-1.amazonaws.com/restapis";

    #[tokio::test]
    async fn apigw_rest_apis_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}?limit=1"), ""),
            json_response(
                200,
                r#"{"item":[{"id":"api1","name":"one"}],"position":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(ApiGatewayQuery)
            .data(ApiGatewayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ apigwRestApis(limit: 1) { items { id name } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["apigwRestApis"]["items"];
        assert_eq!(items[0]["id"], "api1");
        assert_eq!(items[0]["name"], "one");
        assert_eq!(json["apigwRestApis"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn apigw_rest_stages_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}/api1/stages"), ""),
            json_response(
                200,
                r#"{"item":[{"stageName":"prod","deploymentId":"dep1"}]}"#,
            ),
        )]);
        let schema = build_query_schema(ApiGatewayQuery)
            .data(ApiGatewayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ apigwRestStages(apiId: "api1") { stageName deploymentId } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["apigwRestStages"][0]["stageName"], "prod");
        assert_eq!(json["apigwRestStages"][0]["deploymentId"], "dep1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn apigw_rest_resources_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}/api1/resources"), ""),
            json_response(200, r#"{"item":[{"id":"res1","pathPart":"users"}]}"#),
        )]);
        let schema = build_query_schema(ApiGatewayQuery)
            .data(ApiGatewayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ apigwRestResources(apiId: "api1") { items { id pathPart } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["apigwRestResources"]["items"][0]["id"], "res1");
        assert_eq!(json["apigwRestResources"]["items"][0]["pathPart"], "users");
        assert!(json["apigwRestResources"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn apigw_rest_deployments_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}/api1/deployments"), ""),
            json_response(200, r#"{"item":[{"id":"dep1","description":"initial"}]}"#),
        )]);
        let schema = build_query_schema(ApiGatewayQuery)
            .data(ApiGatewayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ apigwRestDeployments(apiId: "api1") { items { id description } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["apigwRestDeployments"]["items"][0]["id"], "dep1");
        assert_eq!(
            json["apigwRestDeployments"]["items"][0]["description"],
            "initial"
        );
        assert!(json["apigwRestDeployments"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
