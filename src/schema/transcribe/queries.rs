use async_graphql::{Context, Object, Result};

use crate::aws::transcribe::TranscribeClient;
use crate::schema::pagination::Page;
use crate::schema::transcribe::types::{
    TranscribeLanguageModel, TranscribeVocabulary, TranscriptionJob,
};

#[derive(Default)]
pub struct TranscribeQuery;

#[Object]
impl TranscribeQuery {
    /// Lists transcription jobs, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn transcribe_jobs(
        &self,
        ctx: &Context<'_>,
        status_equals: Option<String>,
        job_name_contains: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TranscriptionJob>> {
        let client = ctx.data::<TranscribeClient>()?;
        let (items, next_token) = client
            .list_transcription_jobs(status_equals, job_name_contains, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(TranscriptionJob::from).collect(),
            next_token,
        })
    }

    /// Lists custom vocabularies, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn transcribe_vocabularies(
        &self,
        ctx: &Context<'_>,
        state_equals: Option<String>,
        name_contains: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TranscribeVocabulary>> {
        let client = ctx.data::<TranscribeClient>()?;
        let (items, next_token) = client
            .list_vocabularies(state_equals, name_contains, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(TranscribeVocabulary::from).collect(),
            next_token,
        })
    }

    /// Lists custom language models, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn transcribe_language_models(
        &self,
        ctx: &Context<'_>,
        status_equals: Option<String>,
        name_contains: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TranscribeLanguageModel>> {
        let client = ctx.data::<TranscribeClient>()?;
        let (items, next_token) = client
            .list_language_models(status_equals, name_contains, limit, next_token)
            .await?;
        Ok(Page {
            items: items
                .into_iter()
                .map(TranscribeLanguageModel::from)
                .collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::aws::transcribe::TranscribeClient;
    use crate::schema::test_util::build_query_schema;

    use super::TranscribeQuery;

    #[tokio::test]
    async fn transcribe_jobs_forwards_filters_and_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?Status=COMPLETED&JobNameContains=foo&MaxResults=1",
                r#"{"Status":"COMPLETED","JobNameContains":"foo","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"TranscriptionJobSummaries":[{"TranscriptionJobName":"job-1","TranscriptionJobStatus":"COMPLETED","LanguageCode":"en-US","CreationTime":1700000000,"OutputLocationType":"CUSTOMER_BUCKET"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(TranscribeQuery)
            .data(TranscribeClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ transcribeJobs(statusEquals: "COMPLETED", jobNameContains: "foo", limit: 1) { items { transcriptionJobName transcriptionJobStatus languageCode outputLocationType } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["transcribeJobs"]["items"][0]["transcriptionJobName"],
            "job-1"
        );
        assert_eq!(
            json["transcribeJobs"]["items"][0]["transcriptionJobStatus"],
            "COMPLETED"
        );
        assert_eq!(json["transcribeJobs"]["items"][0]["languageCode"], "en-US");
        assert_eq!(
            json["transcribeJobs"]["items"][0]["outputLocationType"],
            "CUSTOMER_BUCKET"
        );
        assert_eq!(json["transcribeJobs"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn transcribe_vocabularies_forwards_filters_and_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?MaxResults=1&StateEquals=READY&NameContains=foo",
                r#"{"StateEquals":"READY","NameContains":"foo","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Vocabularies":[{"VocabularyName":"vocab-1","LanguageCode":"en-US","VocabularyState":"READY","LastModifiedTime":1700000000}]}"#,
            ),
        )]);
        let schema = build_query_schema(TranscribeQuery)
            .data(TranscribeClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ transcribeVocabularies(stateEquals: "READY", nameContains: "foo", limit: 1) { items { vocabularyName languageCode vocabularyState } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["transcribeVocabularies"]["items"][0]["vocabularyName"],
            "vocab-1"
        );
        assert_eq!(
            json["transcribeVocabularies"]["items"][0]["languageCode"],
            "en-US"
        );
        assert_eq!(
            json["transcribeVocabularies"]["items"][0]["vocabularyState"],
            "READY"
        );
        assert!(json["transcribeVocabularies"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn transcribe_language_models_lists_and_maps_fields() {
        // `name_contains` only (no `status_equals`) — the pinned SDK's
        // `ListLanguageModels` `uri_query` codegen emits a malformed query
        // key for `StatusEquals` that panics `http::Uri`'s parser (see
        // `src/aws/transcribe.rs`'s
        // `list_language_models_status_equals_filter_panics_on_pinned_sdk_bug`),
        // so this resolver-layer test avoids that arg entirely.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?NameContains=foo&MaxResults=1",
                r#"{"NameContains":"foo","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Models":[{"ModelName":"model-1","LanguageCode":"en-US","BaseModelName":"NarrowBand","ModelStatus":"COMPLETED","CreateTime":1700000000,"LastModifiedTime":1700000010}]}"#,
            ),
        )]);
        let schema = build_query_schema(TranscribeQuery)
            .data(TranscribeClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ transcribeLanguageModels(nameContains: "foo", limit: 1) { items { modelName languageCode baseModelName modelStatus } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["transcribeLanguageModels"]["items"][0]["modelName"],
            "model-1"
        );
        assert_eq!(
            json["transcribeLanguageModels"]["items"][0]["baseModelName"],
            "NarrowBand"
        );
        assert_eq!(
            json["transcribeLanguageModels"]["items"][0]["modelStatus"],
            "COMPLETED"
        );
        assert!(json["transcribeLanguageModels"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
