use async_graphql::{Context, Object, Result};

use crate::aws::audit_manager::AuditManagerClient;
use crate::schema::audit_manager::types::{
    AuditManagerAssessment, AuditManagerControl, AuditManagerFramework,
};

#[derive(Default)]
pub struct AuditManagerQuery;

#[Object]
impl AuditManagerQuery {
    async fn audit_manager_assessments(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<AuditManagerAssessment>> {
        let client = ctx.data::<AuditManagerClient>()?;
        let assessments = client.list_assessments().await?;
        Ok(assessments
            .into_iter()
            .map(AuditManagerAssessment::from)
            .collect())
    }

    async fn audit_manager_frameworks(
        &self,
        ctx: &Context<'_>,
        framework_type: Option<String>,
    ) -> Result<Vec<AuditManagerFramework>> {
        let client = ctx.data::<AuditManagerClient>()?;
        let frameworks = client.list_frameworks(framework_type).await?;
        Ok(frameworks
            .into_iter()
            .map(AuditManagerFramework::from)
            .collect())
    }

    async fn audit_manager_controls(
        &self,
        ctx: &Context<'_>,
        control_type: Option<String>,
    ) -> Result<Vec<AuditManagerControl>> {
        let client = ctx.data::<AuditManagerClient>()?;
        let controls = client.list_controls(control_type).await?;
        Ok(controls
            .into_iter()
            .map(AuditManagerControl::from)
            .collect())
    }
}
