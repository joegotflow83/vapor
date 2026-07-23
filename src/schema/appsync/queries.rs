use async_graphql::{Context, Object, Result};

use crate::aws::appsync::AppSyncClient;
use crate::schema::appsync::types::{AppSyncApi, AppSyncDataSource};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AppSyncQuery;

#[Object]
impl AppSyncQuery {
    /// Lists AppSync GraphQL APIs, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn appsync_apis(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppSyncApi>> {
        let client = ctx.data::<AppSyncClient>()?;
        let (apis, token) = client.list_graphql_apis(limit, next_token).await?;
        Ok(Page {
            items: apis.into_iter().map(AppSyncApi::from).collect(),
            next_token: token,
        })
    }

    /// Lists AppSync data sources for an API, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn appsync_data_sources(
        &self,
        ctx: &Context<'_>,
        api_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppSyncDataSource>> {
        let client = ctx.data::<AppSyncClient>()?;
        let (sources, token) = client.list_data_sources(&api_id, limit, next_token).await?;
        Ok(Page {
            items: sources.into_iter().map(AppSyncDataSource::from).collect(),
            next_token: token,
        })
    }
}

// Both resolvers are 1:1 passthroughs to a single already-tested
// `AppSyncClient` method (see `src/aws/appsync.rs`'s own test module for the
// pagination/limit/error-mapping behavior) — only light smoke tests are
// needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::appsync::AppSyncClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::AppSyncQuery;

    const APIS: &str = "https://appsync.us-east-1.amazonaws.com/v1/apis";

    #[tokio::test]
    async fn appsync_apis_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}?maxResults=1"), ""),
            json_response(
                200,
                r#"{"graphqlApis":[{"apiId":"api1","name":"one","xrayEnabled":true}],"nextToken":"cursor-b"}"#,
            ),
        )]);
        let schema = build_query_schema(AppSyncQuery)
            .data(AppSyncClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ appsyncApis(limit: 1) { items { apiId name xrayEnabled } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appsyncApis"]["items"];
        assert_eq!(items[0]["apiId"], "api1");
        assert_eq!(items[0]["name"], "one");
        assert_eq!(items[0]["xrayEnabled"], true);
        assert_eq!(json["appsyncApis"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn appsync_data_sources_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}/api1/datasources"), ""),
            json_response(
                200,
                r#"{"dataSources":[{"name":"ds1","type":"AMAZON_DYNAMODB"}]}"#,
            ),
        )]);
        let schema = build_query_schema(AppSyncQuery)
            .data(AppSyncClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ appsyncDataSources(apiId: "api1") { items { name dataSourceType } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appsyncDataSources"]["items"];
        assert_eq!(items[0]["name"], "ds1");
        assert_eq!(items[0]["dataSourceType"], "AMAZON_DYNAMODB");
        assert!(json["appsyncDataSources"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
