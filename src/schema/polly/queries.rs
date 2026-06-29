use async_graphql::{Context, Object, Result};

use crate::aws::polly::PollyClient;
use crate::schema::polly::types::{PollyLexicon, PollySpeechSynthesisTask, PollyVoice};

#[derive(Default)]
pub struct PollyQuery;

#[Object]
impl PollyQuery {
    async fn polly_voices(
        &self,
        ctx: &Context<'_>,
        language_code: Option<String>,
        engine: Option<String>,
    ) -> Result<Vec<PollyVoice>> {
        let client = ctx.data::<PollyClient>()?;
        let items = client.describe_voices(language_code, engine).await?;
        Ok(items.into_iter().map(PollyVoice::from).collect())
    }

    async fn polly_lexicons(&self, ctx: &Context<'_>) -> Result<Vec<PollyLexicon>> {
        let client = ctx.data::<PollyClient>()?;
        let items = client.list_lexicons().await?;
        Ok(items.into_iter().map(PollyLexicon::from).collect())
    }

    async fn polly_speech_synthesis_tasks(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
    ) -> Result<Vec<PollySpeechSynthesisTask>> {
        let client = ctx.data::<PollyClient>()?;
        let items = client.list_speech_synthesis_tasks(status).await?;
        Ok(items
            .into_iter()
            .map(PollySpeechSynthesisTask::from)
            .collect())
    }
}
