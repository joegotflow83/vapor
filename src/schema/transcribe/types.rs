use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

use crate::aws::transcribe::{
    TranscribeLanguageModelInfo, TranscribeVocabularyInfo, TranscriptionJobInfo,
};
use crate::schema::time::to_utc;

#[derive(SimpleObject, Clone)]
pub struct TranscriptionJob {
    pub transcription_job_name: Option<String>,
    pub transcription_job_status: Option<String>,
    pub language_code: Option<String>,
    pub media_sample_rate_hertz: Option<i32>,
    pub media_format: Option<String>,
    pub creation_time: Option<DateTime<Utc>>,
    pub start_time: Option<DateTime<Utc>>,
    pub completion_time: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub output_location_type: Option<String>,
}

impl From<TranscriptionJobInfo> for TranscriptionJob {
    fn from(job: TranscriptionJobInfo) -> Self {
        Self {
            transcription_job_name: job.transcription_job_name,
            transcription_job_status: job.transcription_job_status,
            language_code: job.language_code,
            media_sample_rate_hertz: job.media_sample_rate_hertz,
            media_format: job.media_format,
            creation_time: to_utc(job.creation_time.as_ref()),
            start_time: to_utc(job.start_time.as_ref()),
            completion_time: to_utc(job.completion_time.as_ref()),
            failure_reason: job.failure_reason,
            output_location_type: job.output_location_type,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TranscribeVocabulary {
    pub vocabulary_name: Option<String>,
    pub language_code: Option<String>,
    pub vocabulary_state: Option<String>,
    pub last_modified_time: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
}

impl From<TranscribeVocabularyInfo> for TranscribeVocabulary {
    fn from(vocab: TranscribeVocabularyInfo) -> Self {
        Self {
            vocabulary_name: vocab.vocabulary_name,
            language_code: vocab.language_code,
            vocabulary_state: vocab.vocabulary_state,
            last_modified_time: to_utc(vocab.last_modified_time.as_ref()),
            failure_reason: vocab.failure_reason,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TranscribeLanguageModel {
    pub model_name: Option<String>,
    pub language_code: Option<String>,
    pub base_model_name: Option<String>,
    pub model_status: Option<String>,
    pub create_time: Option<DateTime<Utc>>,
    pub last_modified_time: Option<DateTime<Utc>>,
}

impl From<TranscribeLanguageModelInfo> for TranscribeLanguageModel {
    fn from(model: TranscribeLanguageModelInfo) -> Self {
        Self {
            model_name: model.model_name,
            language_code: model.language_code,
            base_model_name: model.base_model_name,
            model_status: model.model_status,
            create_time: to_utc(model.create_time.as_ref()),
            last_modified_time: to_utc(model.last_modified_time.as_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::transcribe::{
        TranscribeLanguageModelInfo, TranscribeVocabularyInfo, TranscriptionJobInfo,
    };
    use aws_smithy_types::DateTime as SmithyDateTime;

    #[test]
    fn test_transcription_job_from_full() {
        let info = TranscriptionJobInfo {
            transcription_job_name: Some("my-job".to_string()),
            transcription_job_status: Some("COMPLETED".to_string()),
            language_code: Some("en-US".to_string()),
            media_sample_rate_hertz: Some(16000),
            media_format: Some("wav".to_string()),
            creation_time: Some(SmithyDateTime::from_secs(1_705_312_800)),
            start_time: Some(SmithyDateTime::from_secs(1_705_312_860)),
            completion_time: Some(SmithyDateTime::from_secs(1_705_313_100)),
            failure_reason: None,
            output_location_type: Some("SERVICE_BUCKET".to_string()),
        };
        let result = TranscriptionJob::from(info);
        assert_eq!(result.transcription_job_name, Some("my-job".to_string()));
        assert_eq!(
            result.transcription_job_status,
            Some("COMPLETED".to_string())
        );
        assert_eq!(result.language_code, Some("en-US".to_string()));
        assert_eq!(result.media_sample_rate_hertz, Some(16000));
        assert_eq!(result.media_format, Some("wav".to_string()));
        assert_eq!(
            result.creation_time.map(|d| d.to_rfc3339()),
            Some("2024-01-15T10:00:00+00:00".to_string())
        );
        assert_eq!(
            result.completion_time.map(|d| d.to_rfc3339()),
            Some("2024-01-15T10:05:00+00:00".to_string())
        );
        assert!(result.failure_reason.is_none());
        assert_eq!(
            result.output_location_type,
            Some("SERVICE_BUCKET".to_string())
        );
    }

    #[test]
    fn test_transcription_job_from_minimal() {
        let info = TranscriptionJobInfo {
            transcription_job_name: None,
            transcription_job_status: None,
            language_code: None,
            media_sample_rate_hertz: None,
            media_format: None,
            creation_time: None,
            start_time: None,
            completion_time: None,
            failure_reason: None,
            output_location_type: None,
        };
        let result = TranscriptionJob::from(info);
        assert!(result.transcription_job_name.is_none());
        assert!(result.transcription_job_status.is_none());
        assert!(result.media_sample_rate_hertz.is_none());
        assert!(result.creation_time.is_none());
    }

    #[test]
    fn test_transcription_job_failed() {
        let info = TranscriptionJobInfo {
            transcription_job_name: Some("failed-job".to_string()),
            transcription_job_status: Some("FAILED".to_string()),
            language_code: Some("en-US".to_string()),
            media_sample_rate_hertz: None,
            media_format: None,
            creation_time: Some(SmithyDateTime::from_secs(1_705_312_800)),
            start_time: None,
            completion_time: None,
            failure_reason: Some("Audio file too short".to_string()),
            output_location_type: None,
        };
        let result = TranscriptionJob::from(info);
        assert_eq!(result.transcription_job_status, Some("FAILED".to_string()));
        assert_eq!(
            result.failure_reason,
            Some("Audio file too short".to_string())
        );
        assert!(result.completion_time.is_none());
    }

    #[test]
    fn test_vocabulary_from_full() {
        let info = TranscribeVocabularyInfo {
            vocabulary_name: Some("my-vocab".to_string()),
            language_code: Some("en-US".to_string()),
            vocabulary_state: Some("READY".to_string()),
            last_modified_time: Some(SmithyDateTime::from_secs(1_705_312_800)),
            failure_reason: None,
        };
        let result = TranscribeVocabulary::from(info);
        assert_eq!(result.vocabulary_name, Some("my-vocab".to_string()));
        assert_eq!(result.language_code, Some("en-US".to_string()));
        assert_eq!(result.vocabulary_state, Some("READY".to_string()));
        assert_eq!(
            result.last_modified_time.map(|d| d.to_rfc3339()),
            Some("2024-01-15T10:00:00+00:00".to_string())
        );
        assert!(result.failure_reason.is_none());
    }

    #[test]
    fn test_vocabulary_from_failed() {
        let info = TranscribeVocabularyInfo {
            vocabulary_name: Some("bad-vocab".to_string()),
            language_code: Some("fr-FR".to_string()),
            vocabulary_state: Some("FAILED".to_string()),
            last_modified_time: None,
            failure_reason: Some("Invalid file format".to_string()),
        };
        let result = TranscribeVocabulary::from(info);
        assert_eq!(result.vocabulary_state, Some("FAILED".to_string()));
        assert_eq!(
            result.failure_reason,
            Some("Invalid file format".to_string())
        );
        assert!(result.last_modified_time.is_none());
    }

    #[test]
    fn test_language_model_from_full() {
        let info = TranscribeLanguageModelInfo {
            model_name: Some("my-model".to_string()),
            language_code: Some("en-US".to_string()),
            base_model_name: Some("WideBand".to_string()),
            model_status: Some("COMPLETED".to_string()),
            create_time: Some(SmithyDateTime::from_secs(1_705_312_800)),
            last_modified_time: Some(SmithyDateTime::from_secs(1_705_399_200)),
        };
        let result = TranscribeLanguageModel::from(info);
        assert_eq!(result.model_name, Some("my-model".to_string()));
        assert_eq!(result.language_code, Some("en-US".to_string()));
        assert_eq!(result.base_model_name, Some("WideBand".to_string()));
        assert_eq!(result.model_status, Some("COMPLETED".to_string()));
        assert_eq!(
            result.create_time.map(|d| d.to_rfc3339()),
            Some("2024-01-15T10:00:00+00:00".to_string())
        );
        assert_eq!(
            result.last_modified_time.map(|d| d.to_rfc3339()),
            Some("2024-01-16T10:00:00+00:00".to_string())
        );
    }

    #[test]
    fn test_language_model_from_minimal() {
        let info = TranscribeLanguageModelInfo {
            model_name: None,
            language_code: None,
            base_model_name: None,
            model_status: None,
            create_time: None,
            last_modified_time: None,
        };
        let result = TranscribeLanguageModel::from(info);
        assert!(result.model_name.is_none());
        assert!(result.base_model_name.is_none());
        assert!(result.model_status.is_none());
        assert!(result.create_time.is_none());
    }
}
