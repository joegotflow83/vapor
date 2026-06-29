use async_graphql::{Context, Object, Result};

use crate::aws::workspaces::WorkspacesClient;
use crate::schema::workspaces::types::{Workspace, WorkspaceBundle, WorkspaceDirectory};

#[derive(Default)]
pub struct WorkspacesQuery;

#[Object]
impl WorkspacesQuery {
    async fn workspaces(
        &self,
        ctx: &Context<'_>,
        directory_id: Option<String>,
        user_name: Option<String>,
        bundle_id: Option<String>,
    ) -> Result<Vec<Workspace>> {
        let client = ctx.data::<WorkspacesClient>()?;
        let items = client.describe_workspaces(directory_id, user_name, bundle_id).await?;
        Ok(items.into_iter().map(Workspace::from).collect())
    }

    async fn workspace_directories(&self, ctx: &Context<'_>) -> Result<Vec<WorkspaceDirectory>> {
        let client = ctx.data::<WorkspacesClient>()?;
        let items = client.describe_workspace_directories().await?;
        Ok(items.into_iter().map(WorkspaceDirectory::from).collect())
    }

    async fn workspace_bundles(
        &self,
        ctx: &Context<'_>,
        owner: Option<String>,
    ) -> Result<Vec<WorkspaceBundle>> {
        let client = ctx.data::<WorkspacesClient>()?;
        let items = client.describe_workspace_bundles(owner).await?;
        Ok(items.into_iter().map(WorkspaceBundle::from).collect())
    }
}
