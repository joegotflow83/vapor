use async_graphql::{Context, Object, Result};

use crate::aws::security_hub::SecurityHubClient;
use crate::schema::security_hub::types::SecurityHubFinding;

#[derive(Default)]
pub struct SecurityHubQuery;

#[Object]
impl SecurityHubQuery {
    async fn security_hub_findings(
        &self,
        ctx: &Context<'_>,
        severity_label: Option<String>,
        workflow_status: Option<String>,
        record_state: Option<String>,
        max_results: Option<i32>,
    ) -> Result<Vec<SecurityHubFinding>> {
        let client = ctx.data::<SecurityHubClient>()?;
        let findings = client
            .get_findings(severity_label, workflow_status, record_state, max_results)
            .await?;
        Ok(findings.into_iter().map(SecurityHubFinding::from).collect())
    }
}
