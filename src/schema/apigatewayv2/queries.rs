use async_graphql::{Context, Object, Result};

use crate::aws::apigatewayv2::ApiGatewayV2Client;
use crate::schema::apigatewayv2::types::{
    ApiV2, ApiV2DomainName, ApiV2Route, ApiV2Stage, ApiV2VpcLink,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct ApiGatewayV2Query;

#[Object]
impl ApiGatewayV2Query {
    /// List all HTTP and WebSocket APIs (v2). `limit` caps the total results
    /// returned per page (default unlimited); `next_token` resumes from a
    /// prior page's `nextToken`.
    async fn api_v2_apis(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ApiV2>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let (apis, next_token) = client.get_apis(limit, next_token).await?;
        Ok(Page {
            items: apis.into_iter().map(ApiV2::from).collect(),
            next_token,
        })
    }

    /// List all stages for the given API (v2). `limit` caps the total
    /// results returned per page (default unlimited); `next_token` resumes
    /// from a prior page's `nextToken`.
    async fn api_v2_stages(
        &self,
        ctx: &Context<'_>,
        api_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ApiV2Stage>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let (stages, next_token) = client.get_stages(&api_id, limit, next_token).await?;
        Ok(Page {
            items: stages.into_iter().map(ApiV2Stage::from).collect(),
            next_token,
        })
    }

    /// List all routes for the given API (v2). `limit` caps the total
    /// results returned per page (default unlimited); `next_token` resumes
    /// from a prior page's `nextToken`.
    async fn api_v2_routes(
        &self,
        ctx: &Context<'_>,
        api_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ApiV2Route>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let (routes, next_token) = client.get_routes(&api_id, limit, next_token).await?;
        Ok(Page {
            items: routes.into_iter().map(ApiV2Route::from).collect(),
            next_token,
        })
    }

    /// List all custom domain names across APIs. `limit` caps the total
    /// results returned per page (default unlimited); `next_token` resumes
    /// from a prior page's `nextToken`.
    async fn api_v2_domain_names(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ApiV2DomainName>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let (domains, next_token) = client.get_domain_names(limit, next_token).await?;
        Ok(Page {
            items: domains.into_iter().map(ApiV2DomainName::from).collect(),
            next_token,
        })
    }

    /// List all VPC links used by private integrations. `limit` caps the
    /// total results returned per page (default unlimited); `next_token`
    /// resumes from a prior page's `nextToken`.
    async fn api_v2_vpc_links(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ApiV2VpcLink>> {
        let client = ctx.data::<ApiGatewayV2Client>()?;
        let (links, next_token) = client.get_vpc_links(limit, next_token).await?;
        Ok(Page {
            items: links.into_iter().map(ApiV2VpcLink::from).collect(),
            next_token,
        })
    }
}

// All five resolvers are 1:1 passthroughs to a single already-tested
// `ApiGatewayV2Client` method each (see `src/aws/apigatewayv2.rs`'s own test
// module for the pagination/limit/error-mapping behavior) — only light
// smoke tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::apigatewayv2::ApiGatewayV2Client;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::ApiGatewayV2Query;

    const BASE: &str = "https://apigateway.us-east-1.amazonaws.com/v2";

    #[tokio::test]
    async fn api_v2_apis_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/apis?maxResults=1"), ""),
            json_response(
                200,
                r#"{"items":[{"apiId":"api1","name":"one"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(ApiGatewayV2Query)
            .data(ApiGatewayV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ apiV2Apis(limit: 1) { items { apiId name } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["apiV2Apis"]["items"];
        assert_eq!(items[0]["apiId"], "api1");
        assert_eq!(items[0]["name"], "one");
        assert_eq!(json["apiV2Apis"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn api_v2_stages_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/apis/api1/stages"), ""),
            json_response(200, r#"{"items":[{"stageName":"prod","deploymentId":"dep1"}]}"#),
        )]);
        let schema = build_query_schema(ApiGatewayV2Query)
            .data(ApiGatewayV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ apiV2Stages(apiId: "api1") { items { stageName deploymentId } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["apiV2Stages"]["items"][0]["stageName"], "prod");
        assert_eq!(json["apiV2Stages"]["items"][0]["deploymentId"], "dep1");
        assert!(json["apiV2Stages"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn api_v2_routes_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/apis/api1/routes"), ""),
            json_response(200, r#"{"items":[{"routeId":"r1","routeKey":"GET /"}]}"#),
        )]);
        let schema = build_query_schema(ApiGatewayV2Query)
            .data(ApiGatewayV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ apiV2Routes(apiId: "api1") { items { routeId routeKey } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["apiV2Routes"]["items"][0]["routeId"], "r1");
        assert_eq!(json["apiV2Routes"]["items"][0]["routeKey"], "GET /");
        assert!(json["apiV2Routes"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn api_v2_domain_names_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/domainnames"), ""),
            json_response(200, r#"{"items":[{"domainName":"example.com"}]}"#),
        )]);
        let schema = build_query_schema(ApiGatewayV2Query)
            .data(ApiGatewayV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ apiV2DomainNames { items { domainName } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["apiV2DomainNames"]["items"][0]["domainName"], "example.com");
        assert!(json["apiV2DomainNames"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn api_v2_vpc_links_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/vpclinks"), ""),
            json_response(200, r#"{"items":[{"vpcLinkId":"vpc1","name":"link1"}]}"#),
        )]);
        let schema = build_query_schema(ApiGatewayV2Query)
            .data(ApiGatewayV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ apiV2VpcLinks { items { vpcLinkId name } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["apiV2VpcLinks"]["items"][0]["vpcLinkId"], "vpc1");
        assert_eq!(json["apiV2VpcLinks"]["items"][0]["name"], "link1");
        assert!(json["apiV2VpcLinks"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
