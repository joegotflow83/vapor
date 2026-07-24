use async_graphql::{Context, Object, Result};

use crate::aws::polly::PollyClient;
use crate::schema::pagination::Page;
use crate::schema::polly::types::{PollyLexicon, PollySpeechSynthesisTask, PollyVoice};

#[derive(Default)]
pub struct PollyQuery;

#[Object]
impl PollyQuery {
    /// Lists voices, optionally filtered by `language_code`/`engine`, capped
    /// at `limit` results (default unlimited) and resumed from `next_token`.
    async fn polly_voices(
        &self,
        ctx: &Context<'_>,
        language_code: Option<String>,
        engine: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<PollyVoice>> {
        let client = ctx.data::<PollyClient>()?;
        let (items, next_token) = client
            .describe_voices(language_code, engine, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(PollyVoice::from).collect(),
            next_token,
        })
    }

    /// Lists Polly lexicons, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn polly_lexicons(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<PollyLexicon>> {
        let client = ctx.data::<PollyClient>()?;
        let (items, next_token) = client.list_lexicons(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(PollyLexicon::from).collect(),
            next_token,
        })
    }

    /// Lists Polly speech synthesis tasks, optionally scoped to `status`,
    /// capped at `limit` results (default unlimited) and resumed from
    /// `next_token`.
    async fn polly_speech_synthesis_tasks(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<PollySpeechSynthesisTask>> {
        let client = ctx.data::<PollyClient>()?;
        let (items, next_token) = client
            .list_speech_synthesis_tasks(status, limit, next_token)
            .await?;
        Ok(Page {
            items: items
                .into_iter()
                .map(PollySpeechSynthesisTask::from)
                .collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::polly::PollyClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::PollyQuery;

    const BASE: &str = "https://polly.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn polly_voices_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/voices"), ""),
            json_response(
                200,
                r#"{"Voices":[{"Id":"Joanna","LanguageCode":"en-US","Name":"Joanna","Gender":"Female"}],"NextToken":"cursor-a"}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(PollyQuery).data(client).finish();

        let res = schema
            .execute(r#"{ pollyVoices(limit: 1) { items { voiceId languageCode name gender } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["pollyVoices"]["items"][0]["voiceId"], "Joanna");
        assert_eq!(data["pollyVoices"]["items"][0]["languageCode"], "en-US");
        assert_eq!(data["pollyVoices"]["items"][0]["name"], "Joanna");
        assert_eq!(data["pollyVoices"]["items"][0]["gender"], "Female");
        assert_eq!(data["pollyVoices"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn polly_lexicons_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/lexicons"), ""),
            json_response(
                200,
                r#"{"Lexicons":[{"Name":"lex1","Attributes":{"Alphabet":"ipa","LanguageCode":"en-US","LexemesCount":5,"Size":120}}],"NextToken":"cursor-b"}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(PollyQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ pollyLexicons(limit: 1) { items { name attributes { alphabet languageCode lexemeCount size } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["pollyLexicons"]["items"][0]["name"], "lex1");
        assert_eq!(
            data["pollyLexicons"]["items"][0]["attributes"]["alphabet"],
            "ipa"
        );
        assert_eq!(
            data["pollyLexicons"]["items"][0]["attributes"]["lexemeCount"],
            5
        );
        assert_eq!(data["pollyLexicons"]["items"][0]["attributes"]["size"], 120);
        assert_eq!(data["pollyLexicons"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn polly_speech_synthesis_tasks_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/synthesisTasks?MaxResults=1"), ""),
            json_response(
                200,
                r#"{"SynthesisTasks":[{"TaskId":"task-1","TaskStatus":"completed","VoiceId":"Joanna","OutputFormat":"mp3"}],"NextToken":"cursor-c"}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(PollyQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ pollySpeechSynthesisTasks(limit: 1) { items { taskId taskStatus voiceId outputFormat } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data["pollySpeechSynthesisTasks"]["items"][0]["taskId"],
            "task-1"
        );
        assert_eq!(
            data["pollySpeechSynthesisTasks"]["items"][0]["taskStatus"],
            "completed"
        );
        assert_eq!(
            data["pollySpeechSynthesisTasks"]["items"][0]["voiceId"],
            "Joanna"
        );
        assert_eq!(
            data["pollySpeechSynthesisTasks"]["items"][0]["outputFormat"],
            "mp3"
        );
        assert_eq!(data["pollySpeechSynthesisTasks"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }
}
