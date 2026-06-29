use async_graphql::{Context, Object, Result};

use crate::aws::comprehend::ComprehendClient;
use crate::schema::comprehend::types::{
    ComprehendDocumentClassifier, ComprehendEndpoint, ComprehendEntityRecognizer,
};

#[derive(Default)]
pub struct ComprehendQuery;

#[Object]
impl ComprehendQuery {
    async fn comprehend_entity_recognizers(
        &self,
        ctx: &Context<'_>,
        status_filter: Option<String>,
    ) -> Result<Vec<ComprehendEntityRecognizer>> {
        let client = ctx.data::<ComprehendClient>()?;
        let items = client.list_entity_recognizers(status_filter).await?;
        Ok(items
            .into_iter()
            .map(ComprehendEntityRecognizer::from)
            .collect())
    }

    async fn comprehend_document_classifiers(
        &self,
        ctx: &Context<'_>,
        status_filter: Option<String>,
    ) -> Result<Vec<ComprehendDocumentClassifier>> {
        let client = ctx.data::<ComprehendClient>()?;
        let items = client.list_document_classifiers(status_filter).await?;
        Ok(items
            .into_iter()
            .map(ComprehendDocumentClassifier::from)
            .collect())
    }

    async fn comprehend_endpoints(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<ComprehendEndpoint>> {
        let client = ctx.data::<ComprehendClient>()?;
        let items = client.list_endpoints().await?;
        Ok(items.into_iter().map(ComprehendEndpoint::from).collect())
    }
}
