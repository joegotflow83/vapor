use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::cognito::CognitoClient;
use crate::schema::cognito::types::{UserPool, UserPoolClient};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CognitoQuery;

#[Object]
impl CognitoQuery {
    /// List Cognito user pools. `limit` caps the total number of results
    /// across pages (default: unlimited); `next_token` resumes from a
    /// previous page.
    async fn cognito_user_pools(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<UserPool>> {
        let client = ctx.data::<CognitoClient>()?;
        let (pools, next_token) = client.list_user_pools(limit, next_token).await?;

        let futures: Vec<_> = pools
            .iter()
            .map(|p| async {
                let id = p.id().unwrap_or_default();
                client.describe_user_pool(id).await
            })
            .collect();

        let results = join_all(futures).await;
        let mut items = Vec::new();
        for result in results {
            if let Ok(pool) = result {
                items.push(UserPool::from_sdk(&pool));
            }
        }
        Ok(Page { items, next_token })
    }

    /// List app clients for a user pool. `limit` caps the total number of
    /// results across pages (default: unlimited); `next_token` resumes from
    /// a previous page.
    async fn cognito_user_pool_clients(
        &self,
        ctx: &Context<'_>,
        user_pool_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<UserPoolClient>> {
        let client = ctx.data::<CognitoClient>()?;
        let (descriptions, next_token) = client
            .list_user_pool_clients(&user_pool_id, limit, next_token)
            .await?;

        let futures: Vec<_> = descriptions
            .iter()
            .filter_map(|d| {
                let cid = d.client_id()?;
                Some(client.describe_user_pool_client(&user_pool_id, cid))
            })
            .collect();

        let results = join_all(futures).await;
        let items = results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(UserPoolClient::from_sdk)
            .collect();
        Ok(Page { items, next_token })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::cognito::CognitoClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CognitoQuery;

    const ENDPOINT: &str = "https://cognito-idp.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn cognito_user_pools_maps_fan_out_detail() {
        // Two sequential calls per the resolver's own code: `list_user_pools`
        // (discovery), then per-pool `describe_user_pool` (fan-out) — the
        // mock connector serves responses strictly in send order.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"UserPools":[{"Id":"pool-1"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
                json_response(
                    200,
                    r#"{"UserPool":{"Id":"pool-1","Name":"Pool One","EstimatedNumberOfUsers":5}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CognitoQuery)
            .data(CognitoClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ cognitoUserPools { items { id name estimatedNumberOfUsers } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["cognitoUserPools"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["id"], "pool-1");
        assert_eq!(items[0]["name"], "Pool One");
        assert_eq!(items[0]["estimatedNumberOfUsers"], 5);
        assert!(json["cognitoUserPools"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cognito_user_pools_drops_pool_whose_describe_call_errors() {
        // Unlike acm's fan-out, this resolver's `if let Ok(pool) = result`
        // silently drops any pool whose `describe_user_pool` call errors —
        // no error propagation at all, not even for non-not-found errors.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"UserPools":[{"Id":"pool-err"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"UserPoolId":"pool-err"}"#),
                json_error_response("ResourceNotFoundException", "user pool not found"),
            ),
        ]);
        let schema = build_query_schema(CognitoQuery)
            .data(CognitoClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ cognitoUserPools { items { id } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["cognitoUserPools"]["items"].as_array().unwrap().len(),
            0
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cognito_user_pools_passes_limit_and_next_token_to_discovery_call() {
        // Zero pools returned, so no fan-out calls follow — isolates the
        // discovery-call argument-passthrough behavior.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a","MaxResults":5}"#),
            json_response(200, r#"{"UserPools":[]}"#),
        )]);
        let schema = build_query_schema(CognitoQuery)
            .data(CognitoClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cognitoUserPools(limit: 5, nextToken: "cursor-a") { items { id } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cognito_user_pool_clients_maps_fan_out_detail() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
                json_response(200, r#"{"UserPoolClients":[{"ClientId":"client-1"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"UserPoolId":"pool-1","ClientId":"client-1"}"#),
                json_response(
                    200,
                    r#"{"UserPoolClient":{"UserPoolId":"pool-1","ClientId":"client-1","ClientName":"App One"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CognitoQuery)
            .data(CognitoClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cognitoUserPoolClients(userPoolId: "pool-1") { items { clientId clientName userPoolId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["cognitoUserPoolClients"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["clientId"], "client-1");
        assert_eq!(items[0]["clientName"], "App One");
        assert_eq!(items[0]["userPoolId"], "pool-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cognito_user_pool_clients_skips_description_missing_client_id() {
        // The resolver's `filter_map(|d| { let cid = d.client_id()?; ... })`
        // drops a client-id-less description *before* the fan-out — unlike
        // `cognito_user_pools` (which always calls `describe_user_pool` with
        // an empty-string id via `unwrap_or_default`), no `describe_user_pool_client`
        // call is issued for it at all, so only one fan-out event is needed
        // even though discovery returns two descriptions.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
                json_response(
                    200,
                    r#"{"UserPoolClients":[{"ClientId":"client-1"},{"ClientName":"No Id"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"UserPoolId":"pool-1","ClientId":"client-1"}"#),
                json_response(
                    200,
                    r#"{"UserPoolClient":{"UserPoolId":"pool-1","ClientId":"client-1"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CognitoQuery)
            .data(CognitoClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cognitoUserPoolClients(userPoolId: "pool-1") { items { clientId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["cognitoUserPoolClients"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["clientId"], "client-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cognito_user_pool_clients_drops_client_whose_describe_call_errors() {
        // `.filter_map(|r| r.ok())` silently drops any client whose
        // `describe_user_pool_client` call errors, same no-propagation shape
        // as `cognito_user_pools`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"UserPoolId":"pool-1"}"#),
                json_response(200, r#"{"UserPoolClients":[{"ClientId":"client-err"}]}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"UserPoolId":"pool-1","ClientId":"client-err"}"#,
                ),
                json_error_response("ResourceNotFoundException", "app client not found"),
            ),
        ]);
        let schema = build_query_schema(CognitoQuery)
            .data(CognitoClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cognitoUserPoolClients(userPoolId: "pool-1") { items { clientId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["cognitoUserPoolClients"]["items"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cognito_user_pool_clients_passes_limit_and_next_token_to_discovery_call() {
        // Zero clients returned, so no fan-out calls follow — isolates the
        // discovery-call argument-passthrough behavior.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"UserPoolId":"pool-1","NextToken":"cursor-b","MaxResults":3}"#,
            ),
            json_response(200, r#"{"UserPoolClients":[]}"#),
        )]);
        let schema = build_query_schema(CognitoQuery)
            .data(CognitoClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cognitoUserPoolClients(userPoolId: "pool-1", limit: 3, nextToken: "cursor-b") { items { clientId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        http_client.relaxed_requests_match();
    }
}
