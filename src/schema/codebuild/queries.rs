use async_graphql::{Context, Object, Result};

use crate::aws::codebuild::CodeBuildClient;
use crate::schema::codebuild::types::{Build, BuildProject};

#[derive(Default)]
pub struct CodeBuildQuery;

#[Object]
impl CodeBuildQuery {
    async fn build_projects(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<BuildProject>> {
        let client = ctx.data::<CodeBuildClient>()?;
        let project_names = match names {
            Some(n) => n,
            None => client.list_projects().await?,
        };
        if project_names.is_empty() {
            return Ok(Vec::new());
        }
        let projects = client.batch_get_projects(project_names).await?;
        Ok(projects.into_iter().map(BuildProject::from).collect())
    }

    async fn builds(
        &self,
        ctx: &Context<'_>,
        project_name: String,
    ) -> Result<Vec<Build>> {
        let client = ctx.data::<CodeBuildClient>()?;
        let ids = client.list_builds_for_project(&project_name).await?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let builds = client.batch_get_builds(ids).await?;
        Ok(builds.into_iter().map(Build::from).collect())
    }
}
