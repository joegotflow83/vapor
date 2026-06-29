use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct TranscriptionJobInfo {
    pub transcription_job_name: Option<String>,
    pub transcription_job_status: Option<String>,
    pub language_code: Option<String>,
    pub media_sample_rate_hertz: Option<i32>,
    pub media_format: Option<String>,
    pub creation_time: Option<String>,
    pub start_time: Option<String>,
    pub completion_time: Option<String>,
    pub failure_reason: Option<String>,
    pub output_location_type: Option<String>,
}

pub struct TranscribeVocabularyInfo {
    pub vocabulary_name: Option<String>,
    pub language_code: Option<String>,
    pub vocabulary_state: Option<String>,
    pub last_modified_time: Option<String>,
    pub failure_reason: Option<String>,
}

pub struct TranscribeLanguageModelInfo {
    pub model_name: Option<String>,
    pub language_code: Option<String>,
    pub base_model_name: Option<String>,
    pub model_status: Option<String>,
    pub create_time: Option<String>,
    pub last_modified_time: Option<String>,
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

    pub async fn list_transcription_jobs(
        &self,
        status_equals: Option<String>,
        job_name_contains: Option<String>,
    ) -> Result<Vec<TranscriptionJobInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_transcription_jobs();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            if let Some(ref s) = status_equals {
                req = req.status(aws_sdk_transcribe::types::TranscriptionJobStatus::from(
                    s.as_str(),
                ));
            }
            if let Some(ref name) = job_name_contains {
                req = req.job_name_contains(name);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for job in output.transcription_job_summaries() {
                items.push(TranscriptionJobInfo {
                    transcription_job_name: job.transcription_job_name().map(|s| s.to_string()),
                    transcription_job_status: job
                        .transcription_job_status()
                        .map(|s| s.as_str().to_string()),
                    language_code: job.language_code().map(|c| c.as_str().to_string()),
                    media_sample_rate_hertz: None,
                    media_format: None,
                    creation_time: job.creation_time().map(|t| t.to_string()),
                    start_time: job.start_time().map(|t| t.to_string()),
                    completion_time: job.completion_time().map(|t| t.to_string()),
                    failure_reason: job.failure_reason().map(|s| s.to_string()),
                    output_location_type: job
                        .output_location_type()
                        .map(|o| o.as_str().to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_vocabularies(
        &self,
        state_equals: Option<String>,
    ) -> Result<Vec<TranscribeVocabularyInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_vocabularies();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            if let Some(ref s) = state_equals {
                req = req.state_equals(aws_sdk_transcribe::types::VocabularyState::from(
                    s.as_str(),
                ));
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for vocab in output.vocabularies() {
                items.push(TranscribeVocabularyInfo {
                    vocabulary_name: vocab.vocabulary_name().map(|s| s.to_string()),
                    language_code: vocab.language_code().map(|c| c.as_str().to_string()),
                    vocabulary_state: vocab.vocabulary_state().map(|s| s.as_str().to_string()),
                    last_modified_time: vocab.last_modified_time().map(|t| t.to_string()),
                    // Not exposed on VocabularyInfo summaries; only get_vocabulary returns it.
                    failure_reason: None,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_language_models(
        &self,
        status_equals: Option<String>,
    ) -> Result<Vec<TranscribeLanguageModelInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_language_models();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            if let Some(ref s) = status_equals {
                req = req.status_equals(aws_sdk_transcribe::types::ModelStatus::from(s.as_str()));
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for model in output.models() {
                items.push(TranscribeLanguageModelInfo {
                    model_name: model.model_name().map(|s| s.to_string()),
                    language_code: model.language_code().map(|c| c.as_str().to_string()),
                    base_model_name: model.base_model_name().map(|b| b.as_str().to_string()),
                    model_status: model.model_status().map(|s| s.as_str().to_string()),
                    create_time: model.create_time().map(|t| t.to_string()),
                    last_modified_time: model.last_modified_time().map(|t| t.to_string()),
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
