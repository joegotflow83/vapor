use aws_config::SdkConfig;
use aws_sdk_translate::primitives::{DateTime, DateTimeFormat};

use crate::error::VaporError;

#[derive(Debug)]
pub struct TranslateTerminologyInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub arn: Option<String>,
    pub source_language_code: Option<String>,
    pub target_language_codes: Vec<String>,
    pub term_count: Option<i32>,
    pub created_at: Option<DateTime>,
    pub last_updated_at: Option<DateTime>,
    pub directionality: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug)]
pub struct TranslateParallelDataInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub source_language_code: Option<String>,
    pub target_language_codes: Vec<String>,
    pub created_at: Option<DateTime>,
    pub last_updated_at: Option<DateTime>,
}

#[derive(Debug)]
pub struct TranslateTextTranslationJobInfo {
    pub job_id: Option<String>,
    pub job_name: Option<String>,
    pub job_status: Option<String>,
    pub source_language_code: Option<String>,
    pub target_language_codes: Vec<String>,
    pub submitted_time: Option<DateTime>,
    pub end_time: Option<DateTime>,
}

pub struct TranslateJobFilter {
    pub job_name: Option<String>,
    pub job_status: Option<String>,
    pub submitted_before_time: Option<String>,
    pub submitted_after_time: Option<String>,
}

pub struct TranslateClient {
    inner: aws_sdk_translate::Client,
}

impl TranslateClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_translate::Client::new(config),
        }
    }

    /// Lists custom terminologies, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListTerminologies`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-translate` 1.102.0's
    /// `operation/list_terminologies/_list_terminologies_input.rs`), so
    /// `limit` is capped on the request itself; `.into_paginator()` dropped
    /// since it hides the token (kinesis pattern).
    pub async fn list_terminologies(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<TranslateTerminologyInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_terminologies();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for term in output.terminology_properties_list.unwrap_or_default() {
                items.push(TranslateTerminologyInfo {
                    name: term.name,
                    description: term.description,
                    arn: term.arn,
                    source_language_code: term.source_language_code,
                    target_language_codes: term.target_language_codes.unwrap_or_default(),
                    term_count: term.term_count,
                    created_at: term.created_at,
                    last_updated_at: term.last_updated_at,
                    directionality: term.directionality.map(|d| d.as_str().to_string()),
                    format: term.format.map(|f| f.as_str().to_string()),
                });
            }

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists parallel data resources, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListParallelData`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-translate` 1.102.0's
    /// `operation/list_parallel_data/_list_parallel_data_input.rs`), same
    /// pattern as `list_terminologies` above.
    pub async fn list_parallel_data(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<TranslateParallelDataInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_parallel_data();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for pd in output.parallel_data_properties_list.unwrap_or_default() {
                items.push(TranslateParallelDataInfo {
                    name: pd.name,
                    description: pd.description,
                    arn: pd.arn,
                    status: pd.status.map(|s| s.as_str().to_string()),
                    source_language_code: pd.source_language_code,
                    target_language_codes: pd.target_language_codes.unwrap_or_default(),
                    created_at: pd.created_at,
                    last_updated_at: pd.last_updated_at,
                });
            }

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists text translation jobs, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `ListTextTranslationJobs` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-translate` 1.102.0's
    /// `operation/list_text_translation_jobs/
    /// _list_text_translation_jobs_input.rs`); the filter is rebuilt each
    /// loop iteration so it's reapplied per page (fsx/transcribe/
    /// step_functions precedent).
    pub async fn list_text_translation_jobs(
        &self,
        filter: Option<TranslateJobFilter>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<TranslateTextTranslationJobInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_text_translation_jobs();
            if let Some(ref f) = filter {
                let mut filter_builder =
                    aws_sdk_translate::types::TextTranslationJobFilter::builder();
                if let Some(ref name) = f.job_name {
                    filter_builder = filter_builder.job_name(name);
                }
                if let Some(ref status) = f.job_status {
                    filter_builder = filter_builder.job_status(
                        aws_sdk_translate::types::JobStatus::from(status.as_str()),
                    );
                }
                if let Some(ref before) = f.submitted_before_time {
                    if let Ok(dt) =
                        DateTime::from_str(before.as_str(), DateTimeFormat::DateTimeWithOffset)
                            .or_else(|_| DateTime::from_str(before.as_str(), DateTimeFormat::DateTime))
                    {
                        filter_builder = filter_builder.submitted_before_time(dt);
                    }
                }
                if let Some(ref after) = f.submitted_after_time {
                    if let Ok(dt) =
                        DateTime::from_str(after.as_str(), DateTimeFormat::DateTimeWithOffset)
                            .or_else(|_| DateTime::from_str(after.as_str(), DateTimeFormat::DateTime))
                    {
                        filter_builder = filter_builder.submitted_after_time(dt);
                    }
                }
                req = req.filter(filter_builder.build());
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for job in output.text_translation_job_properties_list.unwrap_or_default() {
                items.push(TranslateTextTranslationJobInfo {
                    job_id: job.job_id,
                    job_name: job.job_name,
                    job_status: job.job_status.map(|s| s.as_str().to_string()),
                    source_language_code: job.source_language_code,
                    target_language_codes: job.target_language_codes.unwrap_or_default(),
                    submitted_time: job.submitted_time,
                    end_time: job.end_time,
                });
            }

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

    const BASE: &str = "https://translate.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_terminologies_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"TerminologyPropertiesList":[{"Name":"t1","Description":"desc1","Arn":"arn:t1","SourceLanguageCode":"en","TargetLanguageCodes":["es","fr"],"TermCount":5,"CreatedAt":1705314600,"LastUpdatedAt":1705315600,"Directionality":"UNI","Format":"CSV"},{"Name":"t2","Arn":"arn:t2"}]}"#,
            ),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_terminologies(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let t1 = &items[0];
        assert_eq!(t1.name.as_deref(), Some("t1"));
        assert_eq!(t1.description.as_deref(), Some("desc1"));
        assert_eq!(t1.arn.as_deref(), Some("arn:t1"));
        assert_eq!(t1.source_language_code.as_deref(), Some("en"));
        assert_eq!(t1.target_language_codes, vec!["es".to_string(), "fr".to_string()]);
        assert_eq!(t1.term_count, Some(5));
        assert!(t1.created_at.is_some());
        assert!(t1.last_updated_at.is_some());
        assert_eq!(t1.directionality.as_deref(), Some("UNI"));
        assert_eq!(t1.format.as_deref(), Some("CSV"));

        let t2 = &items[1];
        assert_eq!(t2.name.as_deref(), Some("t2"));
        assert_eq!(t2.description, None);
        assert_eq!(t2.directionality, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_terminologies_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"TerminologyPropertiesList":[{"Name":"t3","Arn":"arn:t3"}]}"#),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_terminologies(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_terminologies_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"TerminologyPropertiesList":[{"Name":"t1","Arn":"arn:t1"},{"Name":"t2","Arn":"arn:t2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_terminologies(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_terminologies_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"TerminologyPropertiesList":[{"Name":"t1","Arn":"arn:t1"},{"Name":"t2","Arn":"arn:t2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":8,"NextToken":"p2"}"#),
                json_response(200, r#"{"TerminologyPropertiesList":[{"Name":"t3","Arn":"arn:t3"}]}"#),
            ),
        ]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_terminologies(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_terminologies_propagates_errors() {
        // `InvalidParameterValueException`, not a throttling-classified code
        // (see memory gotcha 1: those get retried and exhaust the single
        // replay event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InvalidParameterValueException", "bad parameter"),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let err = client.list_terminologies(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_parallel_data_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"ParallelDataPropertiesList":[{"Name":"pd1","Arn":"arn:pd1","Description":"desc1","Status":"ACTIVE","SourceLanguageCode":"en","TargetLanguageCodes":["de"],"CreatedAt":1705314600,"LastUpdatedAt":1705315600},{"Name":"pd2","Arn":"arn:pd2"}]}"#,
            ),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_parallel_data(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let pd1 = &items[0];
        assert_eq!(pd1.name.as_deref(), Some("pd1"));
        assert_eq!(pd1.arn.as_deref(), Some("arn:pd1"));
        assert_eq!(pd1.description.as_deref(), Some("desc1"));
        assert_eq!(pd1.status.as_deref(), Some("ACTIVE"));
        assert_eq!(pd1.source_language_code.as_deref(), Some("en"));
        assert_eq!(pd1.target_language_codes, vec!["de".to_string()]);
        assert!(pd1.created_at.is_some());
        assert!(pd1.last_updated_at.is_some());

        let pd2 = &items[1];
        assert_eq!(pd2.name.as_deref(), Some("pd2"));
        assert_eq!(pd2.status, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_parallel_data_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"ParallelDataPropertiesList":[{"Name":"pd3","Arn":"arn:pd3"}]}"#),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_parallel_data(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_parallel_data_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"ParallelDataPropertiesList":[{"Name":"pd1","Arn":"arn:pd1"},{"Name":"pd2","Arn":"arn:pd2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_parallel_data(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_parallel_data_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"ParallelDataPropertiesList":[{"Name":"pd1","Arn":"arn:pd1"},{"Name":"pd2","Arn":"arn:pd2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":8,"NextToken":"p2"}"#),
                json_response(200, r#"{"ParallelDataPropertiesList":[{"Name":"pd3","Arn":"arn:pd3"}]}"#),
            ),
        ]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_parallel_data(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_parallel_data_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InvalidParameterValueException", "bad parameter"),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let err = client.list_parallel_data(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_text_translation_jobs_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"TextTranslationJobPropertiesList":[{"JobId":"j1","JobName":"job-one","JobStatus":"COMPLETED","SourceLanguageCode":"en","TargetLanguageCodes":["es"],"SubmittedTime":1705314600,"EndTime":1705315600},{"JobId":"j2"}]}"#,
            ),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_text_translation_jobs(None, None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let j1 = &items[0];
        assert_eq!(j1.job_id.as_deref(), Some("j1"));
        assert_eq!(j1.job_name.as_deref(), Some("job-one"));
        assert_eq!(j1.job_status.as_deref(), Some("COMPLETED"));
        assert_eq!(j1.source_language_code.as_deref(), Some("en"));
        assert_eq!(j1.target_language_codes, vec!["es".to_string()]);
        assert!(j1.submitted_time.is_some());
        assert!(j1.end_time.is_some());

        let j2 = &items[1];
        assert_eq!(j2.job_id.as_deref(), Some("j2"));
        assert_eq!(j2.job_status, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_text_translation_jobs_passes_filter_fields() {
        // Timestamps are serialized as epoch seconds (verified against pinned
        // `aws-sdk-translate` 1.102.0's `ser_text_translation_job_filter`),
        // matching the response side's `Format::EpochSeconds` too.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Filter":{"JobName":"job-one","JobStatus":"COMPLETED","SubmittedBeforeTime":1705314600,"SubmittedAfterTime":1705315600}}"#,
            ),
            json_response(200, r#"{"TextTranslationJobPropertiesList":[]}"#),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let filter = TranslateJobFilter {
            job_name: Some("job-one".to_string()),
            job_status: Some("COMPLETED".to_string()),
            submitted_before_time: Some("2024-01-15T10:30:00Z".to_string()),
            submitted_after_time: Some("2024-01-15T10:46:40Z".to_string()),
        };

        let (items, token) = client
            .list_text_translation_jobs(Some(filter), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_text_translation_jobs_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"TextTranslationJobPropertiesList":[{"JobId":"j3"}]}"#),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_text_translation_jobs(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_text_translation_jobs_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"TextTranslationJobPropertiesList":[{"JobId":"j1"},{"JobId":"j2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_text_translation_jobs(None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_text_translation_jobs_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"TextTranslationJobPropertiesList":[{"JobId":"j1"},{"JobId":"j2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":8,"NextToken":"p2"}"#),
                json_response(200, r#"{"TextTranslationJobPropertiesList":[{"JobId":"j3"}]}"#),
            ),
        ]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_text_translation_jobs(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_text_translation_jobs_propagates_errors() {
        // `InvalidRequestException`, not a throttling-classified code.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InvalidRequestException", "malformed request"),
        )]);
        let client = TranslateClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_text_translation_jobs(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "malformed request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

