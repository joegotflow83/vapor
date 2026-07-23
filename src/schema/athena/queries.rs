use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::athena::AthenaClient;
use crate::schema::athena::types::{AthenaNamedQuery, AthenaQueryExecution, AthenaWorkgroup};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AthenaQuery;

#[Object]
impl AthenaQuery {
    /// Lists Athena workgroups, capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn athena_workgroups(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AthenaWorkgroup>> {
        let client = ctx.data::<AthenaClient>()?;
        let (summaries, next_token) = client.list_work_groups(limit, next_token).await?;

        let futures: Vec<_> = summaries
            .iter()
            .map(|s| async {
                let name = s.name().unwrap_or_default();
                client.get_work_group(name).await
            })
            .collect();

        let results = join_all(futures).await;
        Ok(Page {
            items: results
                .into_iter()
                .filter_map(|r| r.ok())
                .map(|wg| AthenaWorkgroup::from_sdk(&wg))
                .collect(),
            next_token,
        })
    }

    /// Lists named queries, optionally filtered by workgroup, capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    async fn athena_named_queries(
        &self,
        ctx: &Context<'_>,
        workgroup: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AthenaNamedQuery>> {
        let client = ctx.data::<AthenaClient>()?;
        let (ids, next_token) = client
            .list_named_queries(workgroup.as_deref(), limit, next_token)
            .await?;
        if ids.is_empty() {
            return Ok(Page { items: Vec::new(), next_token });
        }
        let queries = client.batch_get_named_query(ids).await?;
        Ok(Page {
            items: queries.into_iter().map(AthenaNamedQuery::from).collect(),
            next_token,
        })
    }

    /// Lists query executions, optionally filtered by workgroup, capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    async fn athena_query_executions(
        &self,
        ctx: &Context<'_>,
        workgroup: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AthenaQueryExecution>> {
        let client = ctx.data::<AthenaClient>()?;
        let (ids, next_token) = client
            .list_query_executions(workgroup.as_deref(), limit, next_token)
            .await?;
        if ids.is_empty() {
            return Ok(Page { items: Vec::new(), next_token });
        }
        let executions = client.batch_get_query_execution(ids).await?;
        Ok(Page {
            items: executions.into_iter().map(AthenaQueryExecution::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::athena::AthenaClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::AthenaQuery;

    const ENDPOINT: &str = "https://athena.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn athena_workgroups_maps_discovery_and_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"WorkGroups":[{"Name":"wg1"},{"Name":"wg2"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"WorkGroup":"wg1"}"#),
                json_response(200, r#"{"WorkGroup":{"Name":"wg1","State":"ENABLED"}}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"WorkGroup":"wg2"}"#),
                json_response(200, r#"{"WorkGroup":{"Name":"wg2","State":"DISABLED"}}"#),
            ),
        ]);
        let schema = build_query_schema(AthenaQuery)
            .data(AthenaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ athenaWorkgroups { items { name state } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["athenaWorkgroups"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["name"], "wg1");
        assert_eq!(items[0]["state"], "ENABLED");
        assert_eq!(items[1]["name"], "wg2");
        assert_eq!(items[1]["state"], "DISABLED");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn athena_workgroups_drops_workgroup_whose_fan_out_call_errors() {
        // `get_work_group` is called once per discovered name via
        // `join_all`+`filter_map(Result::ok)` — an error for one name simply
        // drops it from `items` (unlike acm's per-item concurrent calls,
        // there's only one call per item here, so no "sibling call still
        // fires" nuance to account for).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"WorkGroups":[{"Name":"wg1"},{"Name":"wg-denied"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"WorkGroup":"wg1"}"#),
                json_response(200, r#"{"WorkGroup":{"Name":"wg1","State":"ENABLED"}}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"WorkGroup":"wg-denied"}"#),
                json_error_response("AccessDeniedException", "not authorized"),
            ),
        ]);
        let schema = build_query_schema(AthenaQuery)
            .data(AthenaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute("{ athenaWorkgroups { items { name } } }").await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["athenaWorkgroups"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], "wg1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn athena_workgroups_passes_limit_and_next_token_to_discovery_call() {
        // No workgroups returned, so no fan-out calls follow — isolates the
        // discovery-call argument-passthrough behavior.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a","MaxResults":5}"#),
            json_response(200, r#"{"WorkGroups":[]}"#),
        )]);
        let schema = build_query_schema(AthenaQuery)
            .data(AthenaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ athenaWorkgroups(limit: 5, nextToken: "cursor-a") { items { name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn athena_named_queries_maps_discovery_and_batch_get() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"WorkGroup":"wg1"}"#),
                json_response(200, r#"{"NamedQueryIds":["q1","q2"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NamedQueryIds":["q1","q2"]}"#),
                json_response(
                    200,
                    r#"{"NamedQueries":[{"Name":"nq1","Database":"db1","QueryString":"SELECT 1"},{"Name":"nq2","Database":"db1","QueryString":"SELECT 2"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(AthenaQuery)
            .data(AthenaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ athenaNamedQueries(workgroup: "wg1") { items { name database queryString } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["athenaNamedQueries"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["name"], "nq1");
        assert_eq!(items[1]["name"], "nq2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn athena_named_queries_skips_batch_get_when_discovery_returns_no_ids() {
        // The resolver's own `if ids.is_empty() { return ... }` short-circuits
        // before `batch_get_named_query` is ever called — a single
        // `ReplayEvent` is enough to prove no second request is sent.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"NamedQueryIds":[]}"#),
        )]);
        let schema = build_query_schema(AthenaQuery)
            .data(AthenaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ athenaNamedQueries { items { name } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["athenaNamedQueries"]["items"].as_array().unwrap().len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn athena_query_executions_maps_discovery_and_batch_get() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"WorkGroup":"wg1"}"#),
                json_response(200, r#"{"QueryExecutionIds":["e1"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"QueryExecutionIds":["e1"]}"#),
                json_response(
                    200,
                    r#"{"QueryExecutions":[{"QueryExecutionId":"e1","Query":"SELECT 1"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(AthenaQuery)
            .data(AthenaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ athenaQueryExecutions(workgroup: "wg1") { items { id query } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["athenaQueryExecutions"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], "e1");
        assert_eq!(items[0]["query"], "SELECT 1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn athena_query_executions_skips_batch_get_when_discovery_returns_no_ids() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"QueryExecutionIds":[]}"#),
        )]);
        let schema = build_query_schema(AthenaQuery)
            .data(AthenaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ athenaQueryExecutions { items { id } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["athenaQueryExecutions"]["items"].as_array().unwrap().len(), 0);
        http_client.relaxed_requests_match();
    }
}
