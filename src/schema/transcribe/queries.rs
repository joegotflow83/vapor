use async_graphql::{Context, Object, Result};

use crate::aws::transcribe::TranscribeClient;
use crate::schema::transcribe::types::{
    TranscribeLanguageModel, TranscribeVocabulary, TranscriptionJob,
};

#[derive(Default)]
pub struct TranscribeQuery;

#[Object]
impl TranscribeQuery {
    async fn transcribe_jobs(
        &self,
        ctx: &Context<'_>,
        status_equals: Option<String>,
        job_name_contains: Option<String>,
    ) -> Result<Vec<TranscriptionJob>> {
        let client = ctx.data::<TranscribeClient>()?;
        let items = client
            .list_transcription_jobs(status_equals, job_name_contains)
            .await?;
        Ok(items.into_iter().map(TranscriptionJob::from).collect())
    }

    async fn transcribe_vocabularies(
        &self,
        ctx: &Context<'_>,
        state_equals: Option<String>,
    ) -> Result<Vec<TranscribeVocabulary>> {
        let client = ctx.data::<TranscribeClient>()?;
        let items = client.list_vocabularies(state_equals).await?;
        Ok(items.into_iter().map(TranscribeVocabulary::from).collect())
    }

    async fn transcribe_language_models(
        &self,
        ctx: &Context<'_>,
        status_equals: Option<String>,
    ) -> Result<Vec<TranscribeLanguageModel>> {
        let client = ctx.data::<TranscribeClient>()?;
        let items = client.list_language_models(status_equals).await?;
        Ok(items
            .into_iter()
            .map(TranscribeLanguageModel::from)
            .collect())
    }
}
