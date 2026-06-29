use aws_config::SdkConfig;
use aws_sdk_translate::primitives::{DateTime, DateTimeFormat};

use crate::error::VaporError;

pub struct TranslateTerminologyInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub arn: Option<String>,
    pub source_language_code: Option<String>,
    pub target_language_codes: Vec<String>,
    pub term_count: Option<i32>,
    pub created_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub directionality: Option<String>,
    pub format: Option<String>,
}

pub struct TranslateParallelDataInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub source_language_code: Option<String>,
    pub target_language_codes: Vec<String>,
    pub created_at: Option<String>,
    pub last_updated_at: Option<String>,
}

pub struct TranslateTextTranslationJobInfo {
    pub job_id: Option<String>,
    pub job_name: Option<String>,
    pub job_status: Option<String>,
    pub source_language_code: Option<String>,
    pub target_language_codes: Vec<String>,
    pub submitted_time: Option<String>,
    pub end_time: Option<String>,
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

    pub async fn list_terminologies(
        &self,
    ) -> Result<Vec<TranslateTerminologyInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_terminologies();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for term in output.terminology_properties_list() {
                items.push(TranslateTerminologyInfo {
                    name: term.name().map(|s| s.to_string()),
                    description: term.description().map(|s| s.to_string()),
                    arn: term.arn().map(|s| s.to_string()),
                    source_language_code: term.source_language_code().map(|s| s.to_string()),
                    target_language_codes: term
                        .target_language_codes()
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    term_count: term.term_count(),
                    created_at: term.created_at().map(|t| t.to_string()),
                    last_updated_at: term.last_updated_at().map(|t| t.to_string()),
                    directionality: term
                        .directionality()
                        .map(|d| d.as_str().to_string()),
                    format: term.format().map(|f| f.as_str().to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_parallel_data(
        &self,
    ) -> Result<Vec<TranslateParallelDataInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_parallel_data();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for pd in output.parallel_data_properties_list() {
                items.push(TranslateParallelDataInfo {
                    name: pd.name().map(|s| s.to_string()),
                    description: pd.description().map(|s| s.to_string()),
                    arn: pd.arn().map(|s| s.to_string()),
                    status: pd.status().map(|s| s.as_str().to_string()),
                    source_language_code: pd.source_language_code().map(|s| s.to_string()),
                    target_language_codes: pd
                        .target_language_codes()
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    created_at: pd.created_at().map(|t| t.to_string()),
                    last_updated_at: pd.last_updated_at().map(|t| t.to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_text_translation_jobs(
        &self,
        filter: Option<TranslateJobFilter>,
    ) -> Result<Vec<TranslateTextTranslationJobInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_text_translation_jobs();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
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
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for job in output.text_translation_job_properties_list() {
                items.push(TranslateTextTranslationJobInfo {
                    job_id: job.job_id().map(|s| s.to_string()),
                    job_name: job.job_name().map(|s| s.to_string()),
                    job_status: job.job_status().map(|s| s.as_str().to_string()),
                    source_language_code: job.source_language_code().map(|s| s.to_string()),
                    target_language_codes: job
                        .target_language_codes()
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    submitted_time: job.submitted_time().map(|t| t.to_string()),
                    end_time: job.end_time().map(|t| t.to_string()),
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
