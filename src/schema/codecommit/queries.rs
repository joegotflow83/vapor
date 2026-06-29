use async_graphql::{Context, Object, Result};

use crate::aws::codecommit::CodeCommitClient;
use crate::schema::codecommit::types::{
    CodeCommitBranch, CodeCommitPullRequest, CodeCommitRepository,
};

#[derive(Default)]
pub struct CodeCommitQuery;

#[Object]
impl CodeCommitQuery {
    async fn code_commit_repositories(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<CodeCommitRepository>> {
        let client = ctx.data::<CodeCommitClient>()?;
        let items = client.list_repositories().await?;
        Ok(items.into_iter().map(CodeCommitRepository::from).collect())
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

    async fn code_commit_branches(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
    ) -> Result<Vec<CodeCommitBranch>> {
        let client = ctx.data::<CodeCommitClient>()?;
        let items = client.list_branches(repository_name).await?;
        Ok(items.into_iter().map(CodeCommitBranch::from).collect())
    }

    async fn code_commit_pull_requests(
        &self,
        ctx: &Context<'_>,
        repository_name: String,
        pull_request_status: Option<String>,
    ) -> Result<Vec<CodeCommitPullRequest>> {
        let client = ctx.data::<CodeCommitClient>()?;
        let items = client
            .list_pull_requests(repository_name, pull_request_status)
            .await?;
        Ok(items.into_iter().map(CodeCommitPullRequest::from).collect())
    }
}
