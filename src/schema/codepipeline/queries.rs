use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::codepipeline::CodePipelineClient;
use crate::schema::codepipeline::types::{Pipeline, PipelineExecution, StageState};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CodePipelineQuery;

#[Object]
impl CodePipelineQuery {
    /// Lists pipelines, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`.
    async fn pipelines(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Pipeline>> {
        let client = ctx.data::<CodePipelineClient>()?;
        let (summaries, token) = client.list_pipelines(limit, next_token).await?;
        let pipelines = join_all(summaries.into_iter().map(|s| async {
            let name = s.name().unwrap_or_default().to_string();
            let arn = client.get_pipeline_arn(&name).await.ok().flatten();
            Pipeline::from_summary(s, arn)
        }))
        .await;
        Ok(Page { items: pipelines, next_token: token })
    }

    /// Lists executions for a pipeline, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn pipeline_executions(
        &self,
        ctx: &Context<'_>,
        pipeline_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<PipelineExecution>> {
        let client = ctx.data::<CodePipelineClient>()?;
        let (summaries, token) = client
            .list_pipeline_executions(&pipeline_name, limit, next_token)
            .await?;
        let items = summaries
            .iter()
            .map(|s| PipelineExecution::from_summary(&pipeline_name, s))
            .collect();
        Ok(Page { items, next_token: token })
    }

    async fn pipeline_state(
        &self,
        ctx: &Context<'_>,
        pipeline_name: String,
    ) -> Result<Vec<StageState>> {
        let client = ctx.data::<CodePipelineClient>()?;
        let stages = client.get_pipeline_state(&pipeline_name).await?;
        Ok(stages.iter().map(StageState::from).collect())
    }
}

// `pipeline_executions`/`pipeline_state` are thin passthroughs to a single
// already-tested `CodePipelineClient` method plus a `From`/`from_summary`
// mapping already unit-tested in `types.rs` — light smoke tests only.
// `pipelines` has genuine resolver-local logic worth its own coverage: a
// concurrent per-pipeline `get_pipeline_arn` fan-out via `join_all`, and
// notably `get_pipeline_arn`'s `.ok().flatten()` (see `src/aws/codepipeline.rs`)
// silently swallows *any* arn-lookup error (not just not-found, unlike the
// acm resolver's fan-out which propagates non-not-found errors) — worth
// asserting explicitly since it's a distinct error-handling shape from the
// acm precedent.
#[cfg(test)]
mod tests {
    use crate::aws::codepipeline::CodePipelineClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CodePipelineQuery;

    const ENDPOINT: &str = "https://codepipeline.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn pipelines_maps_items_with_arn_fanout_and_next_token() {
        // `ListPipelinesInput.max_results` is genuine, but `list_pipelines`'s
        // own loop only stops early on `items.len() >= limit` when `limit`
        // is `Some` (see `src/aws/codepipeline.rs`) — with no `limit` and a
        // `nextToken` in the mocked page-1 response, it unconditionally
        // fetches a second page, consuming this test's second queued event
        // (intended for the `get_pipeline_arn` fan-out call) as a bogus page
        // 2 instead — same gotcha-29 class as codebuild/codecommit/
        // codedeploy/cloudtrail. `limit: 1` trips the `items.len() >= limit`
        // break after page one so the fan-out call gets its own event.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"maxResults":1}"#),
                json_response(
                    200,
                    r#"{"pipelines":[{"name":"pipeline-1","version":1}],"nextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"name":"pipeline-1"}"#),
                json_response(
                    200,
                    r#"{"metadata":{"pipelineArn":"arn:aws:codepipeline:us-east-1:111122223333:pipeline-1"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CodePipelineQuery)
            .data(CodePipelineClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ pipelines(limit: 1) { items { name arn version } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["pipelines"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["name"], "pipeline-1");
        assert_eq!(items[0]["version"], 1);
        assert_eq!(
            items[0]["arn"],
            "arn:aws:codepipeline:us-east-1:111122223333:pipeline-1"
        );
        assert_eq!(json["pipelines"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pipelines_silently_omits_arn_when_get_pipeline_arn_errors() {
        // `get_pipeline_arn`'s `.ok().flatten()` swallows the error entirely —
        // the pipeline still appears in `items` with `arn: null`, and no
        // GraphQL error is surfaced (contrast with acm's fan-out, which
        // propagates non-not-found describe errors).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"pipelines":[{"name":"pipeline-1"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"name":"pipeline-1"}"#),
                json_error_response("AccessDeniedException", "not authorized"),
            ),
        ]);
        let schema = build_query_schema(CodePipelineQuery)
            .data(CodePipelineClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ pipelines { items { name arn } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["pipelines"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["name"], "pipeline-1");
        assert!(items[0]["arn"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pipelines_forwards_limit_and_next_token_with_no_fanout_when_empty() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"nextToken":"cursor-a","maxResults":5}"#),
            json_response(200, r#"{"pipelines":[]}"#),
        )]);
        let schema = build_query_schema(CodePipelineQuery)
            .data(CodePipelineClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ pipelines(limit: 5, nextToken: "cursor-a") { items { name } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["pipelines"]["items"].as_array().unwrap().len(), 0);
        // Only one queued event (list_pipelines) — a wrongful get_pipeline_arn
        // call would fail `relaxed_requests_match` below.
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pipelines_propagates_list_pipelines_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("ValidationException", "bad request"),
        )]);
        let schema = build_query_schema(CodePipelineQuery)
            .data(CodePipelineClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute("{ pipelines { items { name } } }").await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("bad request"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pipeline_executions_forwards_args_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"pipelineName":"my-pipeline","nextToken":"cursor-a","maxResults":10}"#,
            ),
            json_response(
                200,
                r#"{"pipelineExecutionSummaries":[{"pipelineExecutionId":"exec-1","status":"Succeeded","trigger":{"triggerType":"Webhook"},"startTime":1700000000,"lastUpdateTime":1700000100}]}"#,
            ),
        )]);
        let schema = build_query_schema(CodePipelineQuery)
            .data(CodePipelineClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ pipelineExecutions(pipelineName: "my-pipeline", limit: 10, nextToken: "cursor-a") { items { pipelineName executionId status trigger startedAt lastUpdatedAt } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["pipelineExecutions"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["pipelineName"], "my-pipeline");
        assert_eq!(items[0]["executionId"], "exec-1");
        assert_eq!(items[0]["status"], "Succeeded");
        assert_eq!(items[0]["trigger"], "Webhook");
        assert_eq!(items[0]["startedAt"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["lastUpdatedAt"], "2023-11-14T22:15:00+00:00");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pipeline_executions_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"pipelineName":"my-pipeline"}"#),
            json_error_response("PipelineNotFoundException", "pipeline not found"),
        )]);
        let schema = build_query_schema(CodePipelineQuery)
            .data(CodePipelineClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ pipelineExecutions(pipelineName: "my-pipeline") { items { executionId } } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("pipeline not found"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pipeline_state_maps_stages_and_action_states() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"name":"my-pipeline"}"#),
            json_response(
                200,
                r#"{"pipelineName":"my-pipeline","stageStates":[{"stageName":"Deploy","latestExecution":{"pipelineExecutionId":"exec-1","status":"Failed"},"actionStates":[{"actionName":"Deploy","latestExecution":{"status":"Failed","lastStatusChange":1700000000,"externalExecutionUrl":"https://example.com/build/1","errorDetails":{"code":"JobFailed","message":"boom"}}}]}]}"#,
            ),
        )]);
        let schema = build_query_schema(CodePipelineQuery)
            .data(CodePipelineClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ pipelineState(pipelineName: "my-pipeline") { stageName status actionStates { actionName status lastStatusChange errorDetails externalExecutionUrl } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let stages = json["pipelineState"].as_array().unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0]["stageName"], "Deploy");
        assert_eq!(stages[0]["status"], "Failed");
        let actions = stages[0]["actionStates"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["actionName"], "Deploy");
        assert_eq!(actions[0]["status"], "Failed");
        assert_eq!(actions[0]["lastStatusChange"], "2023-11-14T22:13:20+00:00");
        assert_eq!(actions[0]["errorDetails"], "boom");
        assert_eq!(actions[0]["externalExecutionUrl"], "https://example.com/build/1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pipeline_state_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"name":"my-pipeline"}"#),
            json_error_response("PipelineNotFoundException", "pipeline not found"),
        )]);
        let schema = build_query_schema(CodePipelineQuery)
            .data(CodePipelineClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ pipelineState(pipelineName: "my-pipeline") { stageName } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("pipeline not found"));
        http_client.relaxed_requests_match();
    }
}
