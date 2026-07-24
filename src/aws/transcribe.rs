use aws_config::SdkConfig;

use crate::error::VaporError;

#[derive(Debug)]
pub struct TranscriptionJobInfo {
    pub transcription_job_name: Option<String>,
    pub transcription_job_status: Option<String>,
    pub language_code: Option<String>,
    pub media_sample_rate_hertz: Option<i32>,
    pub media_format: Option<String>,
    pub creation_time: Option<aws_smithy_types::DateTime>,
    pub start_time: Option<aws_smithy_types::DateTime>,
    pub completion_time: Option<aws_smithy_types::DateTime>,
    pub failure_reason: Option<String>,
    pub output_location_type: Option<String>,
}

#[derive(Debug)]
pub struct TranscribeVocabularyInfo {
    pub vocabulary_name: Option<String>,
    pub language_code: Option<String>,
    pub vocabulary_state: Option<String>,
    pub last_modified_time: Option<aws_smithy_types::DateTime>,
    pub failure_reason: Option<String>,
}

#[derive(Debug)]
pub struct TranscribeLanguageModelInfo {
    pub model_name: Option<String>,
    pub language_code: Option<String>,
    pub base_model_name: Option<String>,
    pub model_status: Option<String>,
    pub create_time: Option<aws_smithy_types::DateTime>,
    pub last_modified_time: Option<aws_smithy_types::DateTime>,
}

pub struct TranscribeClient {
    inner: aws_sdk_transcribe::Client,
}

impl TranscribeClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_transcribe::Client::new(config),
        }
    }

    /// Lists transcription jobs, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `ListTranscriptionJobs` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-transcribe` 1.108.0's
    /// `operation/list_transcription_jobs/
    /// _list_transcription_jobs_input.rs`), so `limit` is capped to the
    /// remaining budget on the request itself, matching fsx.rs's pattern.
    pub async fn list_transcription_jobs(
        &self,
        status_equals: Option<String>,
        job_name_contains: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<TranscriptionJobInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_transcription_jobs();
            if let Some(ref s) = status_equals {
                req = req.status(aws_sdk_transcribe::types::TranscriptionJobStatus::from(
                    s.as_str(),
                ));
            }
            if let Some(ref name) = job_name_contains {
                req = req.job_name_contains(name);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for job in output.transcription_job_summaries.unwrap_or_default() {
                items.push(TranscriptionJobInfo {
                    transcription_job_name: job.transcription_job_name,
                    transcription_job_status: job
                        .transcription_job_status
                        .map(|s| s.as_str().to_string()),
                    language_code: job.language_code.map(|c| c.as_str().to_string()),
                    // Not exposed on TranscriptionJobSummary; only the full
                    // TranscriptionJob returned by get_transcription_job has these.
                    media_sample_rate_hertz: None,
                    media_format: None,
                    creation_time: job.creation_time,
                    start_time: job.start_time,
                    completion_time: job.completion_time,
                    failure_reason: job.failure_reason,
                    output_location_type: job.output_location_type.map(|o| o.as_str().to_string()),
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists custom vocabularies, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListVocabularies`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-transcribe` 1.108.0's
    /// `operation/list_vocabularies/_list_vocabularies_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself,
    /// matching fsx.rs's pattern.
    pub async fn list_vocabularies(
        &self,
        state_equals: Option<String>,
        name_contains: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<TranscribeVocabularyInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_vocabularies();
            if let Some(ref s) = state_equals {
                req =
                    req.state_equals(aws_sdk_transcribe::types::VocabularyState::from(s.as_str()));
            }
            if let Some(ref name) = name_contains {
                req = req.name_contains(name);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for vocab in output.vocabularies.unwrap_or_default() {
                items.push(TranscribeVocabularyInfo {
                    vocabulary_name: vocab.vocabulary_name,
                    language_code: vocab.language_code.map(|c| c.as_str().to_string()),
                    vocabulary_state: vocab.vocabulary_state.map(|s| s.as_str().to_string()),
                    last_modified_time: vocab.last_modified_time,
                    // Not exposed on VocabularyInfo summaries; only get_vocabulary returns it.
                    failure_reason: None,
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists custom language models, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `ListLanguageModels` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-transcribe` 1.108.0's
    /// `operation/list_language_models/_list_language_models_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself,
    /// matching fsx.rs's pattern.
    pub async fn list_language_models(
        &self,
        status_equals: Option<String>,
        name_contains: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<TranscribeLanguageModelInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_language_models();
            if let Some(ref s) = status_equals {
                req = req.status_equals(aws_sdk_transcribe::types::ModelStatus::from(s.as_str()));
            }
            if let Some(ref name) = name_contains {
                req = req.name_contains(name);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for model in output.models.unwrap_or_default() {
                items.push(TranscribeLanguageModelInfo {
                    model_name: model.model_name,
                    language_code: model.language_code.map(|c| c.as_str().to_string()),
                    base_model_name: model.base_model_name.map(|b| b.as_str().to_string()),
                    model_status: model.model_status.map(|s| s.as_str().to_string()),
                    create_time: model.create_time,
                    last_modified_time: model.last_modified_time,
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::error::VaporError;

    const ENDPOINT: &str = "https://transcribe.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_transcription_jobs_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"TranscriptionJobSummaries":[{"TranscriptionJobName":"job-1","TranscriptionJobStatus":"COMPLETED","LanguageCode":"en-US","CreationTime":1700000000,"StartTime":1700000010,"CompletionTime":1700000020,"OutputLocationType":"CUSTOMER_BUCKET"},{"TranscriptionJobName":"job-2","TranscriptionJobStatus":"FAILED","FailureReason":"bad audio"}]}"#,
            ),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (jobs, token) = client
            .list_transcription_jobs(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].transcription_job_name, Some("job-1".to_string()));
        assert_eq!(
            jobs[0].transcription_job_status,
            Some("COMPLETED".to_string())
        );
        assert_eq!(jobs[0].language_code, Some("en-US".to_string()));
        assert_eq!(jobs[0].media_sample_rate_hertz, None);
        assert_eq!(jobs[0].media_format, None);
        assert!(jobs[0].creation_time.is_some());
        assert!(jobs[0].start_time.is_some());
        assert!(jobs[0].completion_time.is_some());
        assert_eq!(jobs[0].failure_reason, None);
        assert_eq!(
            jobs[0].output_location_type,
            Some("CUSTOMER_BUCKET".to_string())
        );
        assert_eq!(jobs[1].transcription_job_status, Some("FAILED".to_string()));
        assert_eq!(jobs[1].failure_reason, Some("bad audio".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_transcription_jobs_passes_status_and_job_name_contains_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?Status=COMPLETED&JobNameContains=foo",
                r#"{"Status":"COMPLETED","JobNameContains":"foo"}"#,
            ),
            json_response(
                200,
                r#"{"TranscriptionJobSummaries":[{"TranscriptionJobName":"job-3"}]}"#,
            ),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (jobs, _token) = client
            .list_transcription_jobs(
                Some("COMPLETED".to_string()),
                Some("foo".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].transcription_job_name, Some("job-3".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_transcription_jobs_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?NextToken=cursor-a",
                r#"{"NextToken":"cursor-a"}"#,
            ),
            json_response(
                200,
                r#"{"TranscriptionJobSummaries":[{"TranscriptionJobName":"job-4"}]}"#,
            ),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (jobs, token) = client
            .list_transcription_jobs(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].transcription_job_name, Some("job-4".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_transcription_jobs_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?MaxResults=2",
                r#"{"MaxResults":2}"#,
            ),
            json_response(
                200,
                r#"{"TranscriptionJobSummaries":[{"TranscriptionJobName":"job-a"},{"TranscriptionJobName":"job-b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (jobs, token) = client
            .list_transcription_jobs(None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(jobs.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_transcription_jobs_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    "https://transcribe.us-east-1.amazonaws.com/?MaxResults=10",
                    r#"{"MaxResults":10}"#,
                ),
                json_response(
                    200,
                    r#"{"TranscriptionJobSummaries":[{"TranscriptionJobName":"job-a"},{"TranscriptionJobName":"job-b"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    "https://transcribe.us-east-1.amazonaws.com/?NextToken=p2&MaxResults=8",
                    r#"{"NextToken":"p2","MaxResults":8}"#,
                ),
                json_response(
                    200,
                    r#"{"TranscriptionJobSummaries":[{"TranscriptionJobName":"job-c"}]}"#,
                ),
            ),
        ]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (jobs, token) = client
            .list_transcription_jobs(None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs[2].transcription_job_name, Some("job-c".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_transcription_jobs_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("BadRequestException", "invalid status filter"),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_transcription_jobs(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("BadRequestException"));
                assert_eq!(message, "invalid status filter");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_vocabularies_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Vocabularies":[{"VocabularyName":"vocab-1","LanguageCode":"en-US","VocabularyState":"READY","LastModifiedTime":1700000000},{"VocabularyName":"vocab-2","VocabularyState":"PENDING"}]}"#,
            ),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (vocabs, token) = client
            .list_vocabularies(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(vocabs.len(), 2);
        assert_eq!(vocabs[0].vocabulary_name, Some("vocab-1".to_string()));
        assert_eq!(vocabs[0].language_code, Some("en-US".to_string()));
        assert_eq!(vocabs[0].vocabulary_state, Some("READY".to_string()));
        assert!(vocabs[0].last_modified_time.is_some());
        assert_eq!(vocabs[0].failure_reason, None);
        assert_eq!(vocabs[1].vocabulary_state, Some("PENDING".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_vocabularies_passes_state_equals_and_name_contains_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?StateEquals=READY&NameContains=foo",
                r#"{"StateEquals":"READY","NameContains":"foo"}"#,
            ),
            json_response(200, r#"{"Vocabularies":[{"VocabularyName":"vocab-3"}]}"#),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (vocabs, _token) = client
            .list_vocabularies(
                Some("READY".to_string()),
                Some("foo".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(vocabs.len(), 1);
        assert_eq!(vocabs[0].vocabulary_name, Some("vocab-3".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_vocabularies_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?NextToken=cursor-a",
                r#"{"NextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"Vocabularies":[{"VocabularyName":"vocab-4"}]}"#),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (vocabs, token) = client
            .list_vocabularies(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(vocabs.len(), 1);
        assert_eq!(vocabs[0].vocabulary_name, Some("vocab-4".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_vocabularies_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?MaxResults=2",
                r#"{"MaxResults":2}"#,
            ),
            json_response(
                200,
                r#"{"Vocabularies":[{"VocabularyName":"vocab-a"},{"VocabularyName":"vocab-b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (vocabs, token) = client
            .list_vocabularies(None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(vocabs.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_vocabularies_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("BadRequestException", "invalid state filter"),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_vocabularies(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("BadRequestException"));
                assert_eq!(message, "invalid state filter");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_language_models_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Models":[{"ModelName":"model-1","LanguageCode":"en-US","BaseModelName":"NarrowBand","ModelStatus":"COMPLETED","CreateTime":1700000000,"LastModifiedTime":1700000010},{"ModelName":"model-2","ModelStatus":"IN_PROGRESS"}]}"#,
            ),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (models, token) = client
            .list_language_models(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model_name, Some("model-1".to_string()));
        assert_eq!(models[0].language_code, Some("en-US".to_string()));
        assert_eq!(models[0].base_model_name, Some("NarrowBand".to_string()));
        assert_eq!(models[0].model_status, Some("COMPLETED".to_string()));
        assert!(models[0].create_time.is_some());
        assert!(models[0].last_modified_time.is_some());
        assert_eq!(models[1].model_status, Some("IN_PROGRESS".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_language_models_passes_name_contains_filter() {
        // Only `name_contains` here, deliberately not `status_equals` — see
        // `list_language_models_status_equals_filter_panics_on_pinned_sdk_bug`
        // below for why.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?NameContains=foo",
                r#"{"NameContains":"foo"}"#,
            ),
            json_response(200, r#"{"Models":[{"ModelName":"model-3"}]}"#),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (models, _token) = client
            .list_language_models(None, Some("foo".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_name, Some("model-3".to_string()));
        http_client.relaxed_requests_match();
    }

    // Pinned `aws-sdk-transcribe` 1.108.0's generated `uri_query` fn for
    // `ListLanguageModels` (operation/list_language_models.rs) writes the
    // query key for `status_equals` as the literal string
    // `"         StatusEquals"` (leading spaces baked into the codegen —
    // confirmed by reading the crate source directly, not a copy/paste
    // artifact). `aws_smithy_http::query::Writer::push_kv` doesn't encode
    // keys, only values, so the raw spaces land in the URI and
    // `http::Uri`'s parser rejects it, panicking at the SDK's own
    // `.expect("valid request")` in `make_operation`/`request_builder`
    // before any network I/O happens. This means calling vapor's
    // `list_language_models` with `status_equals: Some(_)` unconditionally
    // panics today — there's no way to work around it from the wrapper
    // since the panic happens inside the pinned SDK's request-building
    // code, not something vapor's own request construction touches. See
    // IMPLEMENTATION_PLAN.md for tracking (only `list_language_models` is
    // affected — `list_transcription_jobs`'s `Status` and
    // `list_vocabularies`'s `StateEquals` query keys are unaffected,
    // confirmed by reading both files' `uri_query` fns).
    #[tokio::test]
    #[should_panic(expected = "valid request")]
    async fn list_language_models_status_equals_filter_panics_on_pinned_sdk_bug() {
        let http_client = StaticReplayClient::new(vec![]);
        let client = TranscribeClient::new(&sdk_config(http_client));

        let _ = client
            .list_language_models(Some("COMPLETED".to_string()), None, None, None)
            .await;
    }

    #[tokio::test]
    async fn list_language_models_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?NextToken=cursor-a",
                r#"{"NextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"Models":[{"ModelName":"model-4"}]}"#),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (models, token) = client
            .list_language_models(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_name, Some("model-4".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_language_models_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://transcribe.us-east-1.amazonaws.com/?MaxResults=2",
                r#"{"MaxResults":2}"#,
            ),
            json_response(
                200,
                r#"{"Models":[{"ModelName":"model-a"},{"ModelName":"model-b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let (models, token) = client
            .list_language_models(None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_language_models_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("BadRequestException", "invalid status filter"),
        )]);
        let client = TranscribeClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_language_models(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("BadRequestException"));
                assert_eq!(message, "invalid status filter");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
