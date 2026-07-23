use aws_config::SdkConfig;
use aws_sdk_codecommit::types::PullRequestStatusEnum;
use aws_smithy_types::DateTime;

use crate::error::VaporError;
use crate::aws::pagination::apply_limit;

#[derive(Debug)]
pub struct CodeCommitRepositoryInfo {
    pub repository_id: Option<String>,
    pub repository_name: Option<String>,
    pub repository_description: Option<String>,
    pub default_branch: Option<String>,
    pub last_modified_date: Option<DateTime>,
    pub creation_date: Option<DateTime>,
    pub clone_url_http: Option<String>,
    pub clone_url_ssh: Option<String>,
    pub arn: Option<String>,
}

#[derive(Debug)]
pub struct CodeCommitBranchInfo {
    pub branch_name: Option<String>,
    pub commit_id: Option<String>,
}

#[derive(Debug)]
pub struct CodeCommitPullRequestTargetInfo {
    pub repository_name: Option<String>,
    pub source_reference: Option<String>,
    pub destination_reference: Option<String>,
    pub merge_base: Option<String>,
}

#[derive(Debug)]
pub struct CodeCommitPullRequestInfo {
    pub pull_request_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub pull_request_status: Option<String>,
    pub author_arn: Option<String>,
    pub creation_date: Option<DateTime>,
    pub last_activity_date: Option<DateTime>,
    pub targets: Vec<CodeCommitPullRequestTargetInfo>,
}

pub struct CodeCommitClient {
    inner: aws_sdk_codecommit::Client,
}

impl CodeCommitClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_codecommit::Client::new(config),
        }
    }

    /// Lists repositories, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListRepositories` has no
    /// `max_results`-equivalent input field (only a bare `next_token`,
    /// verified against pinned `aws-sdk-codecommit` 1.104.0's
    /// `operation/list_repositories/_list_repositories_input.rs`), so
    /// `limit` can only be enforced via client-side `apply_limit` truncation
    /// (xray::get_groups pattern). `batch_get_repositories` fan-out
    /// (25/chunk) covers only the names collected for this page.
    pub async fn list_repositories(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<CodeCommitRepositoryInfo>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_repositories();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for repo in output.repositories.unwrap_or_default() {
                if let Some(name) = repo.repository_name {
                    names.push(name);
                }
            }
            token = output.next_token;

            if apply_limit(&mut names, limit) || token.is_none() {
                break;
            }
        }

        // batch_get_repositories supports up to 25 names at a time
        let mut items = Vec::new();
        for chunk in names.chunks(25) {
            let output = self
                .inner
                .batch_get_repositories()
                .set_repository_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(crate::error::sdk_err)?;

            for repo in output.repositories() {
                items.push(repo_metadata_to_info(repo));
            }
        }

        Ok((items, token))
    }

    pub async fn get_repository(
        &self,
        repository_name: String,
    ) -> Result<Option<CodeCommitRepositoryInfo>, VaporError> {
        let output = self
            .inner
            .get_repository()
            .repository_name(&repository_name)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output.repository_metadata().map(repo_metadata_to_info))
    }

    /// Lists branches, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListBranches` has no
    /// `max_results`-equivalent input field (only a bare `next_token`,
    /// verified against pinned `aws-sdk-codecommit` 1.104.0's
    /// `operation/list_branches/_list_branches_input.rs`), so `limit` can
    /// only be enforced via client-side `apply_limit` truncation
    /// (xray::get_groups pattern). N+1 `get_branch` fan-out covers only the
    /// branch names collected for this page.
    pub async fn list_branches(
        &self,
        repository_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<CodeCommitBranchInfo>, Option<String>), VaporError> {
        let mut branch_names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_branches().repository_name(&repository_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            branch_names.extend(output.branches.unwrap_or_default());
            token = output.next_token;

            if apply_limit(&mut branch_names, limit) || token.is_none() {
                break;
            }
        }

        // N+1 get_branch to retrieve commit_id per branch
        let mut items = Vec::new();
        for name in branch_names {
            let result = self
                .inner
                .get_branch()
                .repository_name(&repository_name)
                .branch_name(&name)
                .send()
                .await;

            let commit_id = result
                .ok()
                .and_then(|o| o.branch().and_then(|b| b.commit_id()).map(|s| s.to_string()));

            items.push(CodeCommitBranchInfo {
                branch_name: Some(name),
                commit_id,
            });
        }

        Ok((items, token))
    }

    /// Lists pull requests, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListPullRequests` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-codecommit` 1.104.0's
    /// `operation/list_pull_requests/_list_pull_requests_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself
    /// (kinesis::list_streams/mq::list_brokers pattern). The request is
    /// rebuilt each loop iteration so `pull_request_status` is reapplied per
    /// page (fsx/transcribe/step_functions precedent). N+1
    /// `get_pull_request` fan-out covers only the ids collected for this
    /// page.
    pub async fn list_pull_requests(
        &self,
        repository_name: String,
        pull_request_status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<CodeCommitPullRequestInfo>, Option<String>), VaporError> {
        let mut pr_ids: Vec<String> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_pull_requests()
                .repository_name(&repository_name);
            if let Some(ref status) = pull_request_status {
                req = req.pull_request_status(PullRequestStatusEnum::from(status.as_str()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - pr_ids.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            pr_ids.extend(output.pull_request_ids);
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if pr_ids.len() as i32 >= l => break,
                _ => continue,
            }
        }

        // N+1 get_pull_request for full details
        let mut items = Vec::new();
        for pr_id in pr_ids {
            let result = self
                .inner
                .get_pull_request()
                .pull_request_id(&pr_id)
                .send()
                .await;

            if let Ok(output) = result {
                if let Some(pr) = output.pull_request() {
                    let targets = pr
                        .pull_request_targets()
                        .iter()
                        .map(|t| CodeCommitPullRequestTargetInfo {
                            repository_name: t.repository_name().map(|s| s.to_string()),
                            source_reference: t.source_reference().map(|s| s.to_string()),
                            destination_reference: t.destination_reference().map(|s| s.to_string()),
                            merge_base: t.merge_base().map(|s| s.to_string()),
                        })
                        .collect();

                    items.push(CodeCommitPullRequestInfo {
                        pull_request_id: pr.pull_request_id().map(|s| s.to_string()),
                        title: pr.title().map(|s| s.to_string()),
                        description: pr.description().map(|s| s.to_string()),
                        pull_request_status: pr
                            .pull_request_status()
                            .map(|s| s.as_str().to_string()),
                        author_arn: pr.author_arn().map(|s| s.to_string()),
                        creation_date: pr.creation_date().cloned(),
                        last_activity_date: pr.last_activity_date().cloned(),
                        targets,
                    });
                }
            }
        }

        Ok((items, token))
    }
}

fn repo_metadata_to_info(
    repo: &aws_sdk_codecommit::types::RepositoryMetadata,
) -> CodeCommitRepositoryInfo {
    CodeCommitRepositoryInfo {
        repository_id: repo.repository_id().map(|s| s.to_string()),
        repository_name: repo.repository_name().map(|s| s.to_string()),
        repository_description: repo.repository_description().map(|s| s.to_string()),
        default_branch: repo.default_branch().map(|s| s.to_string()),
        last_modified_date: repo.last_modified_date().cloned(),
        creation_date: repo.creation_date().cloned(),
        clone_url_http: repo.clone_url_http().map(|s| s.to_string()),
        clone_url_ssh: repo.clone_url_ssh().map(|s| s.to_string()),
        arn: repo.arn().map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://codecommit.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_repositories_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-1","repositoryId":"id-1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryNames":["repo-1"]}"#),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-1","repositoryId":"id-1","repositoryDescription":"desc","defaultBranch":"main","creationDate":1700000000,"lastModifiedDate":1700000100,"cloneUrlHttp":"https://git-codecommit.us-east-1.amazonaws.com/v1/repos/repo-1","cloneUrlSsh":"ssh://git-codecommit.us-east-1.amazonaws.com/v1/repos/repo-1"}],"repositoriesNotFound":[]}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client.list_repositories(None, None).await.unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].repository_name, Some("repo-1".to_string()));
        assert_eq!(repos[0].repository_id, Some("id-1".to_string()));
        assert_eq!(repos[0].repository_description, Some("desc".to_string()));
        assert_eq!(repos[0].default_branch, Some("main".to_string()));
        assert!(repos[0].creation_date.is_some());
        assert!(repos[0].last_modified_date.is_some());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"cursor-a"}"#),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-2","repositoryId":"id-2"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryNames":["repo-2"]}"#),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-2","repositoryId":"id-2"}],"repositoriesNotFound":[]}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client
            .list_repositories(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].repository_name, Some("repo-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_stops_at_limit_and_returns_resume_token() {
        // `ListRepositoriesInput` has no `maxResults`-equivalent field, so
        // `limit` truncates the name list client-side (via `apply_limit`)
        // before the `batch_get_repositories` fan-out — only the truncated
        // names should be fanned out, not the full page.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-1"},{"repositoryName":"repo-2"}],"nextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryNames":["repo-1"]}"#),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-1"}],"repositoriesNotFound":[]}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client.list_repositories(Some(1), None).await.unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-1"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"p2"}"#),
                json_response(200, r#"{"repositories":[{"repositoryName":"repo-2"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryNames":["repo-1","repo-2"]}"#),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-1"},{"repositoryName":"repo-2"}],"repositoriesNotFound":[]}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client.list_repositories(Some(10), None).await.unwrap();

        assert_eq!(repos.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidContinuationTokenException", "bad token"),
        )]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let err = client.list_repositories(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidContinuationTokenException".to_string()));
                assert_eq!(message, "bad token");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_chunks_names_over_25_for_batch_get() {
        let names: Vec<String> = (0..30).map(|i| format!("repo-{i}")).collect();
        let list_body = format!(
            r#"{{"repositories":[{}]}}"#,
            names
                .iter()
                .map(|n| format!(r#"{{"repositoryName":"{n}"}}"#))
                .collect::<Vec<_>>()
                .join(",")
        );
        let first_chunk_body = format!(
            r#"{{"repositoryNames":[{}]}}"#,
            names[..25]
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        let second_chunk_body = format!(
            r#"{{"repositoryNames":[{}]}}"#,
            names[25..]
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(request(ENDPOINT, "{}"), json_response(200, list_body)),
            ReplayEvent::new(
                request(ENDPOINT, first_chunk_body),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-0"}],"repositoriesNotFound":[]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, second_chunk_body),
                json_response(
                    200,
                    r#"{"repositories":[{"repositoryName":"repo-25"}],"repositoriesNotFound":[]}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (repos, _token) = client.list_repositories(None, None).await.unwrap();

        assert_eq!(repos.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_skips_batch_get_when_page_is_empty() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"repositories":[]}"#),
        )]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client.list_repositories(None, None).await.unwrap();

        assert_eq!(repos.len(), 0);
        assert_eq!(token, None);
        // Only one replay event registered — if the wrapper called
        // `batch_get_repositories` with an empty name list anyway, this
        // would panic with "no more test data".
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_repository_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"repo-1"}"#),
            json_response(
                200,
                r#"{"repositoryMetadata":{"repositoryName":"repo-1","repositoryId":"id-1","arn":"arn:aws:codecommit:us-east-1:111122223333:repo-1"}}"#,
            ),
        )]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let repo = client
            .get_repository("repo-1".to_string())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(repo.repository_name, Some("repo-1".to_string()));
        assert_eq!(repo.repository_id, Some("id-1".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_repository_propagates_error_for_nonexistent_repository() {
        // Unlike some `describe_*`/`get_*` wrappers elsewhere in this
        // codebase, `get_repository` has no `RepositoryDoesNotExistException`
        // -> `Ok(None)` special case — every SDK error, including not-found,
        // propagates as `VaporError::AwsSdk`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"missing"}"#),
            json_error_response("RepositoryDoesNotExistException", "repo not found"),
        )]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_repository("missing".to_string())
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("RepositoryDoesNotExistException".to_string()));
                assert_eq!(message, "repo not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_branches_lists_all_when_no_limit() {
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
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (branches, token) = client
            .list_branches("repo-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].branch_name, Some("main".to_string()));
        assert_eq!(branches[0].commit_id, Some("c1".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_branches_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"repo-1","nextToken":"cursor-a"}"#,
                ),
                json_response(200, r#"{"branches":["dev"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1","branchName":"dev"}"#),
                json_response(200, r#"{"branch":{"branchName":"dev","commitId":"c2"}}"#),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (branches, token) = client
            .list_branches("repo-1".to_string(), None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].branch_name, Some("dev".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_branches_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1"}"#),
                json_response(200, r#"{"branches":["main","dev"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1","branchName":"main"}"#),
                json_response(200, r#"{"branch":{"branchName":"main","commitId":"c1"}}"#),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (branches, token) = client
            .list_branches("repo-1".to_string(), Some(1), None)
            .await
            .unwrap();

        assert_eq!(branches.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_branches_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1"}"#),
                json_response(200, r#"{"branches":["main"],"nextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"repo-1","nextToken":"p2"}"#,
                ),
                json_response(200, r#"{"branches":["dev"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1","branchName":"main"}"#),
                json_response(200, r#"{"branch":{"branchName":"main","commitId":"c1"}}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1","branchName":"dev"}"#),
                json_response(200, r#"{"branch":{"branchName":"dev","commitId":"c2"}}"#),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (branches, token) = client
            .list_branches("repo-1".to_string(), Some(10), None)
            .await
            .unwrap();

        assert_eq!(branches.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_branches_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"missing"}"#),
            json_error_response("RepositoryDoesNotExistException", "repo not found"),
        )]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_branches("missing".to_string(), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("RepositoryDoesNotExistException".to_string()));
                assert_eq!(message, "repo not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_branches_swallows_get_branch_errors_as_none_commit_id() {
        // `list_branches`'s N+1 `get_branch` fan-out discards per-branch
        // errors via `.ok()` rather than propagating them — a failed
        // `get_branch` call yields `commit_id: None`, not a bubbled error.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1"}"#),
                json_response(200, r#"{"branches":["main"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1","branchName":"main"}"#),
                json_error_response("EncryptionKeyAccessDeniedException", "denied"),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (branches, _token) = client
            .list_branches("repo-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].branch_name, Some("main".to_string()));
        assert_eq!(branches[0].commit_id, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pull_requests_lists_all_when_no_limit() {
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
                    r#"{"pullRequest":{"pullRequestId":"1","title":"Add feature","description":"desc","pullRequestStatus":"OPEN","authorArn":"arn:aws:iam::111122223333:user/dev","creationDate":1700000000,"lastActivityDate":1700000100,"pullRequestTargets":[{"repositoryName":"repo-1","sourceReference":"refs/heads/feature","destinationReference":"refs/heads/main","mergeBase":"abc123"}]}}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (prs, token) = client
            .list_pull_requests("repo-1".to_string(), Some("OPEN".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].pull_request_id, Some("1".to_string()));
        assert_eq!(prs[0].title, Some("Add feature".to_string()));
        assert_eq!(prs[0].pull_request_status, Some("OPEN".to_string()));
        assert_eq!(prs[0].targets.len(), 1);
        assert_eq!(
            prs[0].targets[0].source_reference,
            Some("refs/heads/feature".to_string())
        );
        assert!(prs[0].creation_date.is_some());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pull_requests_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"repo-1","nextToken":"cursor-a"}"#,
                ),
                json_response(200, r#"{"pullRequestIds":["2"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pullRequestId":"2"}"#),
                json_response(
                    200,
                    r#"{"pullRequest":{"pullRequestId":"2","pullRequestStatus":"CLOSED"}}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (prs, token) = client
            .list_pull_requests(
                "repo-1".to_string(),
                None,
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].pull_request_id, Some("2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pull_requests_stops_at_limit_and_returns_resume_token() {
        // `ListPullRequestsInput` has a real `maxResults` field, so `limit`
        // (minus what's already collected) is passed on the request itself
        // rather than truncated client-side after the fact.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"repo-1","maxResults":1}"#,
                ),
                json_response(
                    200,
                    r#"{"pullRequestIds":["1"],"nextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pullRequestId":"1"}"#),
                json_response(
                    200,
                    r#"{"pullRequest":{"pullRequestId":"1","pullRequestStatus":"OPEN"}}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (prs, token) = client
            .list_pull_requests("repo-1".to_string(), None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pull_requests_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"repo-1","maxResults":10}"#,
                ),
                json_response(
                    200,
                    r#"{"pullRequestIds":["1"],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"repositoryName":"repo-1","nextToken":"p2","maxResults":9}"#,
                ),
                json_response(200, r#"{"pullRequestIds":["2"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pullRequestId":"1"}"#),
                json_response(
                    200,
                    r#"{"pullRequest":{"pullRequestId":"1","pullRequestStatus":"OPEN"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pullRequestId":"2"}"#),
                json_response(
                    200,
                    r#"{"pullRequest":{"pullRequestId":"2","pullRequestStatus":"OPEN"}}"#,
                ),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (prs, token) = client
            .list_pull_requests("repo-1".to_string(), None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(prs.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pull_requests_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"repositoryName":"repo-1"}"#),
            json_error_response("InvalidRepositoryNameException", "bad name"),
        )]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_pull_requests("repo-1".to_string(), None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRepositoryNameException".to_string()));
                assert_eq!(message, "bad name");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pull_requests_omits_items_when_get_pull_request_errors() {
        // The N+1 `get_pull_request` fan-out only pushes an item on `Ok`
        // (`if let Ok(output) = result`) — a failed `get_pull_request` call
        // silently drops that id from the results rather than bubbling the
        // error or leaving a placeholder.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"repositoryName":"repo-1"}"#),
                json_response(200, r#"{"pullRequestIds":["1"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pullRequestId":"1"}"#),
                json_error_response("PullRequestDoesNotExistException", "not found"),
            ),
        ]);
        let client = CodeCommitClient::new(&sdk_config(http_client.clone()));

        let (prs, _token) = client
            .list_pull_requests("repo-1".to_string(), None, None, None)
            .await
            .unwrap();

        assert_eq!(prs.len(), 0);
        http_client.relaxed_requests_match();
    }
}

