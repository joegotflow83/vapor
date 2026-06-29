use aws_config::SdkConfig;
use aws_sdk_codecommit::types::PullRequestStatusEnum;

use crate::error::VaporError;

pub struct CodeCommitRepositoryInfo {
    pub repository_id: Option<String>,
    pub repository_name: Option<String>,
    pub repository_description: Option<String>,
    pub default_branch: Option<String>,
    pub last_modified_date: Option<String>,
    pub creation_date: Option<String>,
    pub clone_url_http: Option<String>,
    pub clone_url_ssh: Option<String>,
    pub arn: Option<String>,
}

pub struct CodeCommitBranchInfo {
    pub branch_name: Option<String>,
    pub commit_id: Option<String>,
}

pub struct CodeCommitPullRequestTargetInfo {
    pub repository_name: Option<String>,
    pub source_reference: Option<String>,
    pub destination_reference: Option<String>,
    pub merge_base: Option<String>,
}

pub struct CodeCommitPullRequestInfo {
    pub pull_request_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub pull_request_status: Option<String>,
    pub author_arn: Option<String>,
    pub creation_date: Option<String>,
    pub last_activity_date: Option<String>,
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

    pub async fn list_repositories(&self) -> Result<Vec<CodeCommitRepositoryInfo>, VaporError> {
        let mut names = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_repositories();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for repo in output.repositories() {
                if let Some(name) = repo.repository_name() {
                    names.push(name.to_string());
                }
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
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
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for repo in output.repositories() {
                items.push(repo_metadata_to_info(repo));
            }
        }

        Ok(items)
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.repository_metadata().map(repo_metadata_to_info))
    }

    pub async fn list_branches(
        &self,
        repository_name: String,
    ) -> Result<Vec<CodeCommitBranchInfo>, VaporError> {
        let mut branch_names = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_branches()
                .repository_name(&repository_name);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for name in output.branches() {
                branch_names.push(name.to_string());
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
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

        Ok(items)
    }

    pub async fn list_pull_requests(
        &self,
        repository_name: String,
        pull_request_status: Option<String>,
    ) -> Result<Vec<CodeCommitPullRequestInfo>, VaporError> {
        let mut pr_ids = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_pull_requests()
                .repository_name(&repository_name);
            if let Some(ref status) = pull_request_status {
                req = req
                    .pull_request_status(PullRequestStatusEnum::from(status.as_str()));
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for id in output.pull_request_ids() {
                pr_ids.push(id.to_string());
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
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
                        creation_date: pr.creation_date().map(|t| t.to_string()),
                        last_activity_date: pr.last_activity_date().map(|t| t.to_string()),
                        targets,
                    });
                }
            }
        }

        Ok(items)
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
        last_modified_date: repo.last_modified_date().map(|t| t.to_string()),
        creation_date: repo.creation_date().map(|t| t.to_string()),
        clone_url_http: repo.clone_url_http().map(|s| s.to_string()),
        clone_url_ssh: repo.clone_url_ssh().map(|s| s.to_string()),
        arn: repo.arn().map(|s| s.to_string()),
    }
}
