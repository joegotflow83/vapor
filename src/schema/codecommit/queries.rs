use async_graphql::{Context, Object, Result};

use crate::aws::codecommit::CodeCommitClient;
use crate::schema::pagination::Page;
use crate::schema::codecommit::types::{
    CodeCommitBranch, CodeCommitPullRequest, CodeCommitRepository,
};

#[derive(Default)]
pub struct CodeCommitQuery;

#[Object]
impl CodeCommitQuery {
    /// Lists CodeCommit repositories, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn code_commit_repositories(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CodeCommitRepository>> {
        let client = ctx.data::<CodeCommitClient>()?;
        let (items, token) = client.list_repositories(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(CodeCommitRepository::from).collect(),
            next_token: token,
        })
    }

    async fn code_commit_repository(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
    ) -> Result<Option<CodeCommitRepository>> {
        let client = ctx.data::<CodeCommitClient>()?;
        let item = client.get_repository(repository_name).await?;
        Ok(item.map(CodeCommitRepository::from))
    }

    /// Lists branches, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn code_commit_branches(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CodeCommitBranch>> {
        let client = ctx.data::<CodeCommitClient>()?;
        let (items, token) = client
            .list_branches(repository_name, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(CodeCommitBranch::from).collect(),
            next_token: token,
        })
    }

    /// Lists pull requests, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn code_commit_pull_requests(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
        pull_request_status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CodeCommitPullRequest>> {
        let client = ctx.data::<CodeCommitClient>()?;
        let (items, token) = client
            .list_pull_requests(repository_name, pull_request_status, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(CodeCommitPullRequest::from).collect(),
            next_token: token,
        })
    }
}

// All four resolvers are 1:1 passthroughs to a single already-tested
// `CodeCommitClient` method (see `src/aws/codecommit.rs`'s own test module
// for the fan-out/pagination/error-mapping behavior, e.g. the
// `batch_get_repositories`/`get_branch`/`get_pull_request` N+1 fan-outs and
// the `pull_request_status` filter forwarding) — only light smoke tests are
// needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::codecommit::CodeCommitClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CodeCommitQuery;

    const ENDPOINT: &str = "https://codecommit.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn code_commit_repositories_maps_items_and_next_token() {
        // `ListRepositoriesInput` has no `maxResults`-equivalent field, so
        // the underlying client keeps paging as long as a `nextToken` comes
        // back and no `limit` has tripped `apply_limit` (see
        // `src/aws/codecommit.rs`'s own `list_repositories_stops_at_limit_...`
        // test) — `limit: 1` is required here to stop the loop after one
        // page, otherwise the client issues a second `list_repositories`
        // request instead of moving on to `batch_get_repositories`, and that
        // request consumes this test's second queued event out of order.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-1"}],"nextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryNames":["repo-1"]}"#),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-1","repositoryId":"id-1","arn":"arn:aws:codecommit:us-east-1:111122223333:repo-1"}],"repositoriesNotFound":[]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CodeCommitQuery)
            .data(CodeCommitClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ codeCommitRepositories(limit: 1) { items { repositoryName repositoryId arn } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["codeCommitRepositories"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["repositoryName"], "repo-1");
        assert_eq!(items[0]["repositoryId"], "id-1");
        assert_eq!(json["codeCommitRepositories"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn code_commit_repository_returns_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"repo-1"}"#),
            json_response(
                200,
                r#"{"repositoryMetadata":{"repositoryName":"repo-1","repositoryId":"id-1"}}"#,
            ),
        )]);
        let schema = build_query_schema(CodeCommitQuery)
            .data(CodeCommitClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ codeCommitRepository(repositoryName: "repo-1") { repositoryName repositoryId } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["codeCommitRepository"]["repositoryName"], "repo-1");
        assert_eq!(json["codeCommitRepository"]["repositoryId"], "id-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn code_commit_repository_returns_none_when_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"missing"}"#),
            json_response(200, r#"{}"#),
        )]);
        let schema = build_query_schema(CodeCommitQuery)
            .data(CodeCommitClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ codeCommitRepository(repositoryName: "missing") { repositoryName } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["codeCommitRepository"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn code_commit_repository_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"missing"}"#),
            json_error_response("RepositoryDoesNotExistException", "repo not found"),
        )]);
        let schema = build_query_schema(CodeCommitQuery)
            .data(CodeCommitClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ codeCommitRepository(repositoryName: "missing") { repositoryName } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("repo not found"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn code_commit_branches_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1"}"#),
                json_response(200, r#"{"branches":["main"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1","branchName":"main"}"#),
                json_response(200, r#"{"branch":{"branchName":"main","commitId":"c1"}}"#),
            ),
        ]);
        let schema = build_query_schema(CodeCommitQuery)
            .data(CodeCommitClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ codeCommitBranches(repositoryName: "repo-1") { items { branchName commitId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["codeCommitBranches"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["branchName"], "main");
        assert_eq!(items[0]["commitId"], "c1");
        assert!(json["codeCommitBranches"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn code_commit_pull_requests_forwards_status_filter_and_maps_targets() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"repo-1","pullRequestStatus":"OPEN"}"#,
                ),
                json_response(200, r#"{"pullRequestIds":["1"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pullRequestId":"1"}"#),
                json_response(
                    200,
                    r#"{"pullRequest":{"pullRequestId":"1","title":"Add feature","pullRequestStatus":"OPEN","pullRequestTargets":[{"repositoryName":"repo-1","sourceReference":"refs/heads/feature","destinationReference":"refs/heads/main"}]}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CodeCommitQuery)
            .data(CodeCommitClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ codeCommitPullRequests(repositoryName: "repo-1", pullRequestStatus: "OPEN") { items { pullRequestId title pullRequestStatus targets { sourceReference destinationReference } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["codeCommitPullRequests"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["pullRequestId"], "1");
        assert_eq!(items[0]["pullRequestStatus"], "OPEN");
        assert_eq!(items[0]["targets"][0]["sourceReference"], "refs/heads/feature");
        http_client.relaxed_requests_match();
    }
}
