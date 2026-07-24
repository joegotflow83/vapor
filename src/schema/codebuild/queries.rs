use async_graphql::{Context, Object, Result};

use crate::aws::codebuild::CodeBuildClient;
use crate::schema::codebuild::types::{Build, BuildProject};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CodeBuildQuery;

#[Object]
impl CodeBuildQuery {
    /// Lists build projects, optionally filtered to explicit `names` (which
    /// bypasses pagination entirely — the full set is fetched in one
    /// `batch_get_projects` call), otherwise capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn build_projects(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BuildProject>> {
        let client = ctx.data::<CodeBuildClient>()?;
        let (project_names, next_token) = match names {
            Some(n) => (n, None),
            None => client.list_projects(limit, next_token).await?,
        };
        if project_names.is_empty() {
            return Ok(Page {
                items: Vec::new(),
                next_token,
            });
        }
        let projects = client.batch_get_projects(project_names).await?;
        Ok(Page {
            items: projects.into_iter().map(BuildProject::from).collect(),
            next_token,
        })
    }

    /// Lists builds for a project, capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn builds(
        &self,
        ctx: &Context<'_>,
        project_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Build>> {
        let client = ctx.data::<CodeBuildClient>()?;
        let (ids, next_token) = client
            .list_builds_for_project(&project_name, limit, next_token)
            .await?;
        if ids.is_empty() {
            return Ok(Page {
                items: Vec::new(),
                next_token,
            });
        }
        let builds = client.batch_get_builds(ids).await?;
        Ok(Page {
            items: builds.into_iter().map(Build::from).collect(),
            next_token,
        })
    }
}

// Both resolvers have real logic beyond a bare passthrough: `build_projects`
// branches on whether `names` was given (bypasses `list_projects` discovery
// entirely and goes straight to `batch_get_projects`, always returning
// `next_token: None`) vs. discovery (`list_projects` then `batch_get_projects`,
// forwarding the discovery call's own `next_token`), plus an early-return
// when discovery finds nothing (skips `batch_get_projects` altogether);
// `builds` has the same list-then-batch_get shape with the same
// empty-early-return branch. Per the resolver-layer sweep's stated scope
// this gets bespoke coverage rather than a single light smoke test.
#[cfg(test)]
mod tests {
    use crate::aws::codebuild::CodeBuildClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CodeBuildQuery;

    const ENDPOINT: &str = "https://codebuild.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn build_projects_with_names_bypasses_discovery_and_batch_gets_directly() {
        // Only one queued event: if the resolver didn't skip `list_projects`
        // when `names` was given, `StaticReplayClient` would fail with "no
        // more test data available".
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"names":["proj-1"]}"#),
            json_response(
                200,
                r#"{"projects":[{"name":"proj-1","arn":"arn:aws:codebuild:us-east-1:111122223333:project/proj-1"}]}"#,
            ),
        )]);
        let schema = build_query_schema(CodeBuildQuery)
            .data(CodeBuildClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ buildProjects(names: ["proj-1"]) { items { name arn } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["buildProjects"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["name"], "proj-1");
        // The names path is a targeted lookup, not a single AWS-side stream
        // — never resumable regardless of what `batch_get_projects` returns.
        assert!(json["buildProjects"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn build_projects_without_names_discovers_then_batch_gets_and_forwards_next_token() {
        // `list_projects`'s own internal pagination loop (already covered by
        // `src/aws/codebuild.rs`'s own tests) keeps paging as long as a
        // `nextToken` comes back and no `limit` has tripped `apply_limit` —
        // so a `limit: 1` arg is required here to stop the loop after one
        // page, otherwise the client issues a second `list_projects` request
        // instead of moving on to `batch_get_projects`, and that request
        // consumes this test's second queued event out of order.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"projects":["proj-1"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"names":["proj-1"]}"#),
                json_response(200, r#"{"projects":[{"name":"proj-1"}]}"#),
            ),
        ]);
        let schema = build_query_schema(CodeBuildQuery)
            .data(CodeBuildClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ buildProjects(limit: 1) { items { name } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["buildProjects"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["name"], "proj-1");
        assert_eq!(json["buildProjects"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn build_projects_returns_empty_and_skips_batch_get_when_discovery_finds_nothing() {
        // Only one queued event: if the resolver didn't early-return on an
        // empty discovery result, it would call `batch_get_projects` anyway
        // and `StaticReplayClient` would fail with "no more test data
        // available".
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"projects":[]}"#),
        )]);
        let schema = build_query_schema(CodeBuildQuery)
            .data(CodeBuildClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ buildProjects { items { name } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["buildProjects"]["items"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(json["buildProjects"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn build_projects_propagates_batch_get_errors() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"projects":["proj-1"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"names":["proj-1"]}"#),
                json_error_response("InvalidInputException", "bad names"),
            ),
        ]);
        let schema = build_query_schema(CodeBuildQuery)
            .data(CodeBuildClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ buildProjects { items { name } } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("bad names"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn builds_happy_path_lists_then_batch_gets_and_forwards_next_token() {
        // Same `limit`-required-to-stop-the-internal-pagination-loop caveat
        // as `build_projects_without_names_...` above — `list_builds_for_project`
        // would otherwise page a second time on this mocked `nextToken`
        // instead of moving on to `batch_get_builds`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"projectName":"my-project"}"#),
                json_response(200, r#"{"ids":["my-project:build-1"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ids":["my-project:build-1"]}"#),
                json_response(
                    200,
                    r#"{"builds":[{"id":"my-project:build-1","projectName":"my-project","buildStatus":"SUCCEEDED"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CodeBuildQuery)
            .data(CodeBuildClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ builds(projectName: "my-project", limit: 1) { items { id projectName buildStatus } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["builds"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["id"], "my-project:build-1");
        assert_eq!(items[0]["buildStatus"], "SUCCEEDED");
        assert_eq!(json["builds"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn builds_returns_empty_and_skips_batch_get_when_no_ids_found() {
        // Only one queued event: if the resolver didn't early-return on an
        // empty id list, it would call `batch_get_builds` anyway and
        // `StaticReplayClient` would fail with "no more test data available".
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"projectName":"my-project"}"#),
            json_response(200, r#"{"ids":[]}"#),
        )]);
        let schema = build_query_schema(CodeBuildQuery)
            .data(CodeBuildClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ builds(projectName: "my-project") { items { id } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["builds"]["items"].as_array().unwrap().is_empty());
        assert!(json["builds"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
