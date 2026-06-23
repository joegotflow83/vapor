use async_graphql::{Context, Object, Result};

use crate::aws::inspector::InspectorClient;
use crate::schema::inspector::types::{InspectorCoverage, InspectorFinding};

#[derive(Default)]
pub struct InspectorQuery;

#[Object]
impl InspectorQuery {
    async fn inspector_findings(
        &self,
        ctx: &Context<'_>,
        severity: Option<String>,
        resource_type: Option<String>,
    ) -> Result<Vec<InspectorFinding>> {
        let client = ctx.data::<InspectorClient>()?;
        let findings = client.list_findings(severity, resource_type).await?;
        Ok(findings.into_iter().map(InspectorFinding::from).collect())
    }

    async fn inspector_coverage(&self, ctx: &Context<'_>) -> Result<Vec<InspectorCoverage>> {
        let client = ctx.data::<InspectorClient>()?;
        let resources = client.list_coverage().await?;
        Ok(resources.into_iter().map(InspectorCoverage::from).collect())
    }
}
