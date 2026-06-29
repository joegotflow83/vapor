use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct PollyVoiceInfo {
    pub voice_id: Option<String>,
    pub language_code: Option<String>,
    pub language_name: Option<String>,
    pub name: Option<String>,
    pub gender: Option<String>,
    pub additional_language_codes: Vec<String>,
    pub supported_engines: Vec<String>,
}

pub struct PollyLexiconInfo {
    pub name: Option<String>,
    pub alphabet: Option<String>,
    pub language_code: Option<String>,
    pub last_modified: Option<String>,
    pub lexeme_count: Option<i32>,
    pub size: Option<i32>,
}

pub struct PollySpeechSynthesisTaskInfo {
    pub task_id: Option<String>,
    pub task_status: Option<String>,
    pub task_status_reason: Option<String>,
    pub output_uri: Option<String>,
    pub creation_time: Option<String>,
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

    pub async fn describe_voices(
        &self,
        language_code: Option<String>,
        engine: Option<String>,
    ) -> Result<Vec<PollyVoiceInfo>, VaporError> {
        let mut req = self.inner.describe_voices();
        if let Some(ref lc) = language_code {
            req = req.language_code(aws_sdk_polly::types::LanguageCode::from(lc.as_str()));
        }
        if let Some(ref e) = engine {
            req = req.engine(aws_sdk_polly::types::Engine::from(e.as_str()));
        }
        let output = req
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        let voices = output
            .voices()
            .iter()
            .map(|v| PollyVoiceInfo {
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
            })
            .collect();

        Ok(voices)
    }

    pub async fn list_lexicons(&self) -> Result<Vec<PollyLexiconInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_lexicons();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for lex in output.lexicons() {
                let attrs = lex.attributes();
                items.push(PollyLexiconInfo {
                    name: lex.name().map(|s| s.to_string()),
                    alphabet: attrs.and_then(|a| a.alphabet()).map(|s| s.to_string()),
                    language_code: attrs
                        .and_then(|a| a.language_code())
                        .map(|lc| lc.as_str().to_string()),
                    last_modified: attrs
                        .and_then(|a| a.last_modified())
                        .map(|t| t.to_string()),
                    lexeme_count: attrs.map(|a| a.lexemes_count()),
                    size: attrs.map(|a| a.size()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_speech_synthesis_tasks(
        &self,
        status: Option<String>,
    ) -> Result<Vec<PollySpeechSynthesisTaskInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_speech_synthesis_tasks();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            if let Some(ref s) = status {
                req = req.status(aws_sdk_polly::types::TaskStatus::from(s.as_str()));
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for task in output.synthesis_tasks() {
                items.push(PollySpeechSynthesisTaskInfo {
                    task_id: task.task_id().map(|s| s.to_string()),
                    task_status: task.task_status().map(|s| s.as_str().to_string()),
                    task_status_reason: task.task_status_reason().map(|s| s.to_string()),
                    output_uri: task.output_uri().map(|s| s.to_string()),
                    creation_time: task.creation_time().map(|t| t.to_string()),
                    text_type: task.text_type().map(|t| t.as_str().to_string()),
                    voice_id: task.voice_id().map(|v| v.as_str().to_string()),
                    output_format: task.output_format().map(|f| f.as_str().to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
