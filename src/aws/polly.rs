use aws_config::SdkConfig;

use crate::aws::pagination::apply_limit;
use crate::error::VaporError;

#[derive(Debug)]
pub struct PollyVoiceInfo {
    pub voice_id: Option<String>,
    pub language_code: Option<String>,
    pub language_name: Option<String>,
    pub name: Option<String>,
    pub gender: Option<String>,
    pub additional_language_codes: Vec<String>,
    pub supported_engines: Vec<String>,
}

#[derive(Debug)]
pub struct PollyLexiconInfo {
    pub name: Option<String>,
    pub alphabet: Option<String>,
    pub language_code: Option<String>,
    pub last_modified: Option<aws_smithy_types::DateTime>,
    pub lexeme_count: Option<i32>,
    pub size: Option<i32>,
}

#[derive(Debug)]
pub struct PollySpeechSynthesisTaskInfo {
    pub task_id: Option<String>,
    pub task_status: Option<String>,
    pub task_status_reason: Option<String>,
    pub output_uri: Option<String>,
    pub creation_time: Option<aws_smithy_types::DateTime>,
    pub text_type: Option<String>,
    pub voice_id: Option<String>,
    pub output_format: Option<String>,
}

pub struct PollyClient {
    inner: aws_sdk_polly::Client,
}

impl PollyClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_polly::Client::new(config),
        }
    }

    /// Lists voices, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `DescribeVoices` returns `next_token`
    /// only if the response is truncated (confirmed in the SDK docs for
    /// `DescribeVoicesOutput`); in practice Polly's full voice catalog fits
    /// in one page, but the field existing means it's a real (if rarely hit)
    /// truncation risk. `DescribeVoicesInput` has no `max_results`-equivalent
    /// field at all (verified: `_describe_voices_input.rs` only exposes
    /// `next_token`, no size hint) — same caveat class as
    /// `cost_explorer.rs::get_cost_and_usage`: `limit` can only be enforced
    /// via client-side `apply_limit` truncation, so when that trips mid-page
    /// the returned `next_token` is still AWS's *next*-page token, permanently
    /// skipping whatever was truncated off the current page.
    pub async fn describe_voices(
        &self,
        language_code: Option<String>,
        engine: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PollyVoiceInfo>, Option<String>), VaporError> {
        let mut voices = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_voices();
            if let Some(ref lc) = language_code {
                req = req.language_code(aws_sdk_polly::types::LanguageCode::from(lc.as_str()));
            }
            if let Some(ref e) = engine {
                req = req.engine(aws_sdk_polly::types::Engine::from(e.as_str()));
            }
            if let Some(ref tok) = token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            voices.extend(output.voices().iter().map(|v| {
                PollyVoiceInfo {
                    voice_id: v.id().map(|id| id.as_str().to_string()),
                    language_code: v.language_code().map(|lc| lc.as_str().to_string()),
                    language_name: v.language_name().map(|s| s.to_string()),
                    name: v.name().map(|s| s.to_string()),
                    gender: v.gender().map(|g| g.as_str().to_string()),
                    additional_language_codes: v
                        .additional_language_codes()
                        .iter()
                        .map(|lc| lc.as_str().to_string())
                        .collect(),
                    supported_engines: v
                        .supported_engines()
                        .iter()
                        .map(|e| e.as_str().to_string())
                        .collect(),
                }
            }));

            token = match output.next_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            if apply_limit(&mut voices, limit) {
                break;
            }
            if token.is_none() {
                break;
            }
        }

        Ok((voices, token))
    }

    /// Lists lexicons, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListLexiconsInput` has no
    /// `max_results`-equivalent field at all (verified:
    /// `_list_lexicons_input.rs` only exposes `next_token`) — same caveat
    /// class as `describe_voices` above and `cost_explorer.rs`'s
    /// `get_cost_and_usage`.
    pub async fn list_lexicons(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PollyLexiconInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_lexicons();
            if let Some(ref tok) = token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for lex in output.lexicons() {
                let attrs = lex.attributes();
                items.push(PollyLexiconInfo {
                    name: lex.name().map(|s| s.to_string()),
                    alphabet: attrs.and_then(|a| a.alphabet()).map(|s| s.to_string()),
                    language_code: attrs
                        .and_then(|a| a.language_code())
                        .map(|lc| lc.as_str().to_string()),
                    last_modified: attrs.and_then(|a| a.last_modified()).cloned(),
                    lexeme_count: attrs.map(|a| a.lexemes_count()),
                    size: attrs.map(|a| a.size()),
                });
            }

            token = match output.next_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            if apply_limit(&mut items, limit) {
                break;
            }
            if token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    /// Lists speech synthesis tasks, optionally scoped to `status` and capped
    /// at `limit` results (default unlimited) and resumed from `next_token`.
    /// `ListSpeechSynthesisTasks` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-polly` 1.110.0's
    /// `operation/list_speech_synthesis_tasks/
    /// _list_speech_synthesis_tasks_input.rs`), so `limit` is capped to the
    /// remaining budget on the request itself, matching `kinesis.rs`'s
    /// `list_streams` pattern — dropped `into_paginator()` since it hides the
    /// token needed to resume from the GraphQL layer.
    pub async fn list_speech_synthesis_tasks(
        &self,
        status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PollySpeechSynthesisTaskInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_speech_synthesis_tasks();
            if let Some(ref s) = status {
                req = req.status(aws_sdk_polly::types::TaskStatus::from(s.as_str()));
            }
            if let Some(ref tok) = token {
                req = req.next_token(tok);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for task in output.synthesis_tasks.unwrap_or_default() {
                items.push(PollySpeechSynthesisTaskInfo {
                    task_id: task.task_id,
                    task_status: task.task_status.map(|s| s.as_str().to_string()),
                    task_status_reason: task.task_status_reason,
                    output_uri: task.output_uri,
                    creation_time: task.creation_time,
                    text_type: task.text_type.map(|t| t.as_str().to_string()),
                    voice_id: task.voice_id.map(|v| v.as_str().to_string()),
                    output_format: task.output_format.map(|f| f.as_str().to_string()),
                });
            }
            token = output.next_token.filter(|t| !t.is_empty());

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
    use aws_smithy_types::DateTime;

    const BASE: &str = "https://polly.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn describe_voices_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/voices"), ""),
            json_response(
                200,
                r#"{"Voices":[{"Id":"Joanna","LanguageCode":"en-US","LanguageName":"US English","Name":"Joanna","Gender":"Female","AdditionalLanguageCodes":["en-GB"],"SupportedEngines":["standard","neural"]},{"Id":"Matthew"}]}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (voices, token) = client
            .describe_voices(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(voices.len(), 2);
        let v1 = &voices[0];
        assert_eq!(v1.voice_id, Some("Joanna".to_string()));
        assert_eq!(v1.language_code, Some("en-US".to_string()));
        assert_eq!(v1.language_name, Some("US English".to_string()));
        assert_eq!(v1.name, Some("Joanna".to_string()));
        assert_eq!(v1.gender, Some("Female".to_string()));
        assert_eq!(v1.additional_language_codes, vec!["en-GB".to_string()]);
        assert_eq!(
            v1.supported_engines,
            vec!["standard".to_string(), "neural".to_string()]
        );

        let v2 = &voices[1];
        assert_eq!(v2.voice_id, Some("Matthew".to_string()));
        assert_eq!(v2.language_code, None);
        assert_eq!(v2.additional_language_codes, Vec::<String>::new());
        assert_eq!(v2.supported_engines, Vec::<String>::new());

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_voices_filters_by_language_code_and_engine() {
        // Query-param order (`Engine` before `LanguageCode`) read straight
        // from the pinned SDK's `describe_voices`'s `uri_query` fn.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v1/voices?Engine=neural&LanguageCode=en-US"),
                "",
            ),
            json_response(
                200,
                r#"{"Voices":[{"Id":"Joanna","LanguageCode":"en-US"}]}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (voices, token) = client
            .describe_voices(
                Some("en-US".to_string()),
                Some("neural".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].voice_id, Some("Joanna".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_voices_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/voices?NextToken=cursor-a"), ""),
            json_response(200, r#"{"Voices":[{"Id":"Amy"}]}"#),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (voices, token) = client
            .describe_voices(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(voices.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_voices_stops_at_limit_and_returns_upstream_next_token() {
        // `DescribeVoicesInput` has no `max_results`-equivalent field, so
        // `limit` is enforced by client-side `apply_limit` truncation on a
        // single already-fetched page. The returned token is still AWS's
        // actual next-page token (not recomputed after truncation) — this
        // is the caveat documented on `describe_voices` itself: it
        // permanently skips whatever got truncated off this page.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/voices"), ""),
            json_response(
                200,
                r#"{"Voices":[{"Id":"v1"},{"Id":"v2"},{"Id":"v3"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (voices, token) = client
            .describe_voices(None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(voices.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_voices_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/voices"), ""),
                json_response(
                    200,
                    r#"{"Voices":[{"Id":"v1"},{"Id":"v2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/voices?NextToken=p2"), ""),
                json_response(200, r#"{"Voices":[{"Id":"v3"}]}"#),
            ),
        ]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (voices, token) = client
            .describe_voices(None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(voices.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_voices_propagates_errors() {
        // `ServiceFailureException`, not a throttling-classified code (see
        // memory gotcha 1: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/voices"), ""),
            json_error_response("ServiceFailureException", "internal error"),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_voices(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ServiceFailureException".to_string()));
                assert_eq!(message, "internal error");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_lexicons_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/lexicons"), ""),
            json_response(
                200,
                r#"{"Lexicons":[{"Name":"lex1","Attributes":{"Alphabet":"ipa","LanguageCode":"en-US","LastModified":1700000000,"LexiconArn":"arn:aws:polly:us-east-1:123456789012:lexicon/lex1","LexemesCount":5,"Size":120}},{"Name":"lex2"}]}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_lexicons(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let l1 = &items[0];
        assert_eq!(l1.name, Some("lex1".to_string()));
        assert_eq!(l1.alphabet, Some("ipa".to_string()));
        assert_eq!(l1.language_code, Some("en-US".to_string()));
        assert_eq!(l1.last_modified, Some(DateTime::from_secs(1_700_000_000)));
        assert_eq!(l1.lexeme_count, Some(5));
        assert_eq!(l1.size, Some(120));

        // `LexiconAttributes::lexemes_count()`/`size()` return bare `i32`
        // (not `Option<i32>`) on this pinned SDK version even with no
        // `*_correct_errors` post-processing involved — a missing
        // `Attributes` object entirely means `attrs` itself is `None`, so
        // the wrapper's `attrs.map(...)` yields `None` for every attribute
        // field, not a defaulted `Some(0)`.
        let l2 = &items[1];
        assert_eq!(l2.name, Some("lex2".to_string()));
        assert_eq!(l2.alphabet, None);
        assert_eq!(l2.lexeme_count, None);
        assert_eq!(l2.size, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_lexicons_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/lexicons?NextToken=cursor-a"), ""),
            json_response(200, r#"{"Lexicons":[{"Name":"lex3"}]}"#),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_lexicons(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_lexicons_stops_at_limit_and_returns_upstream_next_token() {
        // Same no-server-side-limit-field shape as `describe_voices` above:
        // `ListLexiconsInput` only has `next_token`, so `limit` truncates
        // client-side and the returned token is AWS's actual next-page
        // token, not recomputed post-truncation.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/lexicons"), ""),
            json_response(
                200,
                r#"{"Lexicons":[{"Name":"lex1"},{"Name":"lex2"},{"Name":"lex3"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_lexicons(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_lexicons_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/lexicons"), ""),
                json_response(
                    200,
                    r#"{"Lexicons":[{"Name":"lex1"},{"Name":"lex2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/lexicons?NextToken=p2"), ""),
                json_response(200, r#"{"Lexicons":[{"Name":"lex3"}]}"#),
            ),
        ]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_lexicons(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_lexicons_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/lexicons"), ""),
            json_error_response("InvalidNextTokenException", "invalid token"),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let err = client.list_lexicons(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidNextTokenException".to_string()));
                assert_eq!(message, "invalid token");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_speech_synthesis_tasks_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/synthesisTasks"), ""),
            json_response(
                200,
                r#"{"SynthesisTasks":[{"TaskId":"task-1","TaskStatus":"completed","OutputUri":"https://s3.amazonaws.com/bucket/task-1.mp3","CreationTime":1700000000,"TextType":"text","VoiceId":"Joanna","OutputFormat":"mp3"},{"TaskId":"task-2"}]}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_speech_synthesis_tasks(None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        let t1 = &items[0];
        assert_eq!(t1.task_id, Some("task-1".to_string()));
        assert_eq!(t1.task_status, Some("completed".to_string()));
        assert_eq!(
            t1.output_uri,
            Some("https://s3.amazonaws.com/bucket/task-1.mp3".to_string())
        );
        assert_eq!(t1.creation_time, Some(DateTime::from_secs(1_700_000_000)));
        assert_eq!(t1.text_type, Some("text".to_string()));
        assert_eq!(t1.voice_id, Some("Joanna".to_string()));
        assert_eq!(t1.output_format, Some("mp3".to_string()));

        let t2 = &items[1];
        assert_eq!(t2.task_id, Some("task-2".to_string()));
        assert_eq!(t2.task_status, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_speech_synthesis_tasks_filters_by_status() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/synthesisTasks?Status=inProgress"), ""),
            json_response(
                200,
                r#"{"SynthesisTasks":[{"TaskId":"task-5","TaskStatus":"inProgress"}]}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_speech_synthesis_tasks(Some("inProgress".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].task_status, Some("inProgress".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_speech_synthesis_tasks_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/synthesisTasks?NextToken=cursor-a"), ""),
            json_response(200, r#"{"SynthesisTasks":[{"TaskId":"task-9"}]}"#),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_speech_synthesis_tasks(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_speech_synthesis_tasks_stops_at_limit_and_returns_resume_token() {
        // Unlike `describe_voices`/`list_lexicons`, `ListSpeechSynthesisTasksInput`
        // has a real `max_results` field and the wrapper forwards `limit`
        // straight to it with no client-side truncate — so the canned
        // response must return exactly `limit` items (AWS-side enforcement),
        // not more.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/synthesisTasks?MaxResults=2"), ""),
            json_response(
                200,
                r#"{"SynthesisTasks":[{"TaskId":"task-1"},{"TaskId":"task-2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_speech_synthesis_tasks(None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_speech_synthesis_tasks_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/synthesisTasks?MaxResults=10"), ""),
                json_response(
                    200,
                    r#"{"SynthesisTasks":[{"TaskId":"task-1"},{"TaskId":"task-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v1/synthesisTasks?MaxResults=8&NextToken=p2"),
                    "",
                ),
                json_response(200, r#"{"SynthesisTasks":[{"TaskId":"task-3"}]}"#),
            ),
        ]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_speech_synthesis_tasks(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_speech_synthesis_tasks_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/synthesisTasks"), ""),
            json_error_response("ServiceFailureException", "internal error"),
        )]);
        let client = PollyClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_speech_synthesis_tasks(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ServiceFailureException".to_string()));
                assert_eq!(message, "internal error");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
