use async_graphql::{Context, Object, Result};

use crate::aws::ses::SesClient;
use crate::schema::ses::types::{
    SesAccountDetails, SesConfigurationSet, SesEmailTemplate, SesIdentity,
    SesSuppressedDestination,
};

#[derive(Default)]
pub struct SesQuery;

#[Object]
impl SesQuery {
    async fn ses_identities(
        &self,
        ctx: &Context<'_>,
        page_size: Option<i32>,
    ) -> Result<Vec<SesIdentity>> {
        let client = ctx.data::<SesClient>()?;
        let identities = client.list_email_identities(page_size).await?;
        Ok(identities.into_iter().map(SesIdentity::from).collect())
    }

    async fn ses_identity(
        &self,
        ctx: &Context<'_>,
        identity: String,
    ) -> Result<Option<SesIdentity>> {
        let client = ctx.data::<SesClient>()?;
        let result = client.get_email_identity(identity).await?;
        Ok(result.map(SesIdentity::from))
    }

    async fn ses_configuration_sets(&self, ctx: &Context<'_>) -> Result<Vec<SesConfigurationSet>> {
        let client = ctx.data::<SesClient>()?;
        let sets = client.list_configuration_sets().await?;
        Ok(sets.into_iter().map(SesConfigurationSet::from).collect())
    }

    async fn ses_email_templates(&self, ctx: &Context<'_>) -> Result<Vec<SesEmailTemplate>> {
        let client = ctx.data::<SesClient>()?;
        let templates = client.list_email_templates().await?;
        Ok(templates.into_iter().map(SesEmailTemplate::from).collect())
    }

    async fn ses_suppressed_destinations(
        &self,
        ctx: &Context<'_>,
        reasons: Option<Vec<String>>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<Vec<SesSuppressedDestination>> {
        let client = ctx.data::<SesClient>()?;
        let destinations = client
            .list_suppressed_destinations(reasons, start_date, end_date)
            .await?;
        Ok(destinations
            .into_iter()
            .map(SesSuppressedDestination::from)
            .collect())
    }

    async fn ses_account_details(&self, ctx: &Context<'_>) -> Result<Option<SesAccountDetails>> {
        let client = ctx.data::<SesClient>()?;
        let account = client.get_account().await?;
        Ok(Some(SesAccountDetails::from(account)))
    }
}
