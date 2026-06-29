use async_graphql::{Context, InputObject, Object, Result};

use crate::aws::translate::{TranslateClient, TranslateJobFilter};
use crate::schema::translate::types::{
    TranslateParallelData, TranslateTerminology, TranslateTextTranslationJob,
};

#[derive(InputObject)]
pub struct TranslateJobFilterInput {
    pub job_name: Option<String>,
    pub job_status: Option<String>,
    pub submitted_before_time: Option<String>,
    pub submitted_after_time: Option<String>,
}

#[derive(Default)]
pub struct TranslateQuery;

#[Object]
impl TranslateQuery {
    async fn translate_terminologies(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<TranslateTerminology>> {
        let client = ctx.data::<TranslateClient>()?;
        let items = client.list_terminologies().await?;
        Ok(items.into_iter().map(TranslateTerminology::from).collect())
    }

    async fn translate_parallel_data(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<TranslateParallelData>> {
        let client = ctx.data::<TranslateClient>()?;
        let items = client.list_parallel_data().await?;
        Ok(items.into_iter().map(TranslateParallelData::from).collect())
    }

    async fn translate_text_translation_jobs(
        &self,
        ctx: &Context<'_>,
        filter: Option<TranslateJobFilterInput>,
    ) -> Result<Vec<TranslateTextTranslationJob>> {
        let client = ctx.data::<TranslateClient>()?;
        let filter = filter.map(|f| TranslateJobFilter {
            job_name: f.job_name,
            job_status: f.job_status,
            submitted_before_time: f.submitted_before_time,
            submitted_after_time: f.submitted_after_time,
        });
        let items = client.list_text_translation_jobs(filter).await?;
        Ok(items
            .into_iter()
            .map(TranslateTextTranslationJob::from)
            .collect())
    }
}
