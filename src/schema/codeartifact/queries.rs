use async_graphql::{Context, Object, Result};

use crate::aws::codeartifact::CodeArtifactClient;
use crate::schema::codeartifact::types::{
    CodeArtifactDomain, CodeArtifactPackage, CodeArtifactRepository,
};

#[derive(Default)]
pub struct CodeArtifactQuery;

#[Object]
impl CodeArtifactQuery {
    async fn code_artifact_domains(&self, ctx: &Context<'_>) -> Result<Vec<CodeArtifactDomain>> {
        let client = ctx.data::<CodeArtifactClient>()?;
        let items = client.list_domains().await?;
        Ok(items.into_iter().map(CodeArtifactDomain::from).collect())
    }

    async fn code_artifact_repositories(
        &self,
        ctx: &Context<'_>,
        domain: String,
        domain_owner: Option<String>,
    ) -> Result<Vec<CodeArtifactRepository>> {
        let client = ctx.data::<CodeArtifactClient>()?;
        let items = client.list_repositories(domain, domain_owner).await?;
        Ok(items
            .into_iter()
            .map(CodeArtifactRepository::from)
            .collect())
    }

    async fn code_artifact_packages(
        &self,
        ctx: &Context<'_>,
        domain: String,
        repository: String,
        format: Option<String>,
        namespace: Option<String>,
    ) -> Result<Vec<CodeArtifactPackage>> {
        let client = ctx.data::<CodeArtifactClient>()?;
        let items = client
            .list_packages(domain, repository, format, namespace)
            .await?;
        Ok(items.into_iter().map(CodeArtifactPackage::from).collect())
    }
}
