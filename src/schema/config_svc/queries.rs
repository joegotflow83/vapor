use async_graphql::{Context, Object, Result};

use crate::aws::config_svc::AwsConfigClient;
use crate::schema::config_svc::types::{ComplianceByResource, ComplianceSummary, ConfigRule};

#[derive(Default)]
pub struct AwsConfigQuery;

#[Object]
impl AwsConfigQuery {
    async fn config_rules(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConfigRule>> {
        let client = ctx.data::<AwsConfigClient>()?;
        let rules = client.describe_config_rules(names).await?;
        Ok(rules.into_iter().map(ConfigRule::from).collect())
    }

    async fn compliance_by_rule(
        &self,
        ctx: &Context<'_>,
        rule_names: Option<Vec<String>>,
        compliance_types: Option<Vec<String>>,
    ) -> Result<Vec<ComplianceSummary>> {
        let client = ctx.data::<AwsConfigClient>()?;
        let results = client
            .describe_compliance_by_config_rule(rule_names, compliance_types)
            .await?;
        Ok(results.into_iter().map(ComplianceSummary::from).collect())
    }

    async fn compliance_by_resource(
        &self,
        ctx: &Context<'_>,
        resource_type: Option<String>,
        compliance_types: Option<Vec<String>>,
    ) -> Result<Vec<ComplianceByResource>> {
        let client = ctx.data::<AwsConfigClient>()?;
        let results = client
            .describe_compliance_by_resource(resource_type, compliance_types)
            .await?;
        Ok(results.into_iter().map(ComplianceByResource::from).collect())
    }
}
