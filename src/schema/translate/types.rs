use async_graphql::SimpleObject;

use crate::aws::translate::{
    TranslateParallelDataInfo, TranslateTerminologyInfo, TranslateTextTranslationJobInfo,
};

#[derive(SimpleObject, Clone)]
pub struct TranslateTerminology {
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

impl From<TranslateTerminologyInfo> for TranslateTerminology {
    fn from(info: TranslateTerminologyInfo) -> Self {
        Self {
            name: info.name,
            description: info.description,
            arn: info.arn,
            source_language_code: info.source_language_code,
            target_language_codes: info.target_language_codes,
            term_count: info.term_count,
            created_at: info.created_at,
            last_updated_at: info.last_updated_at,
            directionality: info.directionality,
            format: info.format,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TranslateParallelData {
    pub name: Option<String>,
    pub description: Option<String>,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub source_language_code: Option<String>,
    pub target_language_codes: Vec<String>,
    pub created_at: Option<String>,
    pub last_updated_at: Option<String>,
}

impl From<TranslateParallelDataInfo> for TranslateParallelData {
    fn from(info: TranslateParallelDataInfo) -> Self {
        Self {
            name: info.name,
            description: info.description,
            arn: info.arn,
            status: info.status,
            source_language_code: info.source_language_code,
            target_language_codes: info.target_language_codes,
            created_at: info.created_at,
            last_updated_at: info.last_updated_at,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TranslateTextTranslationJob {
    pub job_id: Option<String>,
    pub job_name: Option<String>,
    pub job_status: Option<String>,
    pub source_language_code: Option<String>,
    pub target_language_codes: Vec<String>,
    pub submitted_time: Option<String>,
    pub end_time: Option<String>,
}

impl From<TranslateTextTranslationJobInfo> for TranslateTextTranslationJob {
    fn from(info: TranslateTextTranslationJobInfo) -> Self {
        Self {
            job_id: info.job_id,
            job_name: info.job_name,
            job_status: info.job_status,
            source_language_code: info.source_language_code,
            target_language_codes: info.target_language_codes,
            submitted_time: info.submitted_time,
            end_time: info.end_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::translate::{
        TranslateParallelDataInfo, TranslateTerminologyInfo, TranslateTextTranslationJobInfo,
    };

    #[test]
    fn test_terminology_from_full() {
        let info = TranslateTerminologyInfo {
            name: Some("my-terminology".to_string()),
            description: Some("Custom terms".to_string()),
            arn: Some("arn:aws:translate:us-east-1:123:terminology/my-terminology".to_string()),
            source_language_code: Some("en".to_string()),
            target_language_codes: vec!["fr".to_string(), "de".to_string()],
            term_count: Some(42),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            last_updated_at: Some("2024-06-01T00:00:00Z".to_string()),
            directionality: Some("UNI".to_string()),
            format: Some("CSV".to_string()),
        };
        let result = TranslateTerminology::from(info);
        assert_eq!(result.name, Some("my-terminology".to_string()));
        assert_eq!(result.source_language_code, Some("en".to_string()));
        assert_eq!(result.target_language_codes, vec!["fr", "de"]);
        assert_eq!(result.term_count, Some(42));
        assert_eq!(result.directionality, Some("UNI".to_string()));
        assert_eq!(result.format, Some("CSV".to_string()));
    }

    #[test]
    fn test_terminology_from_minimal() {
        let info = TranslateTerminologyInfo {
            name: None,
            description: None,
            arn: None,
            source_language_code: None,
            target_language_codes: vec![],
            term_count: None,
            created_at: None,
            last_updated_at: None,
            directionality: None,
            format: None,
        };
        let result = TranslateTerminology::from(info);
        assert!(result.name.is_none());
        assert!(result.target_language_codes.is_empty());
        assert!(result.term_count.is_none());
    }

    #[test]
    fn test_parallel_data_from_full() {
        let info = TranslateParallelDataInfo {
            name: Some("my-pd".to_string()),
            description: Some("Parallel corpus".to_string()),
            arn: Some("arn:aws:translate:us-east-1:123:parallel-data/my-pd".to_string()),
            status: Some("ACTIVE".to_string()),
            source_language_code: Some("en".to_string()),
            target_language_codes: vec!["es".to_string()],
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            last_updated_at: Some("2024-06-01T00:00:00Z".to_string()),
        };
        let result = TranslateParallelData::from(info);
        assert_eq!(result.name, Some("my-pd".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.target_language_codes, vec!["es"]);
    }

    #[test]
    fn test_parallel_data_from_minimal() {
        let info = TranslateParallelDataInfo {
            name: None,
            description: None,
            arn: None,
            status: None,
            source_language_code: None,
            target_language_codes: vec![],
            created_at: None,
            last_updated_at: None,
        };
        let result = TranslateParallelData::from(info);
        assert!(result.name.is_none());
        assert!(result.status.is_none());
        assert!(result.target_language_codes.is_empty());
    }

    #[test]
    fn test_text_translation_job_from_full() {
        let info = TranslateTextTranslationJobInfo {
            job_id: Some("job-abc123".to_string()),
            job_name: Some("my-batch-job".to_string()),
            job_status: Some("COMPLETED".to_string()),
            source_language_code: Some("en".to_string()),
            target_language_codes: vec!["fr".to_string(), "es".to_string()],
            submitted_time: Some("2024-01-15T10:00:00Z".to_string()),
            end_time: Some("2024-01-15T10:30:00Z".to_string()),
        };
        let result = TranslateTextTranslationJob::from(info);
        assert_eq!(result.job_id, Some("job-abc123".to_string()));
        assert_eq!(result.job_name, Some("my-batch-job".to_string()));
        assert_eq!(result.job_status, Some("COMPLETED".to_string()));
        assert_eq!(result.source_language_code, Some("en".to_string()));
        assert_eq!(result.target_language_codes, vec!["fr", "es"]);
        assert!(result.submitted_time.is_some());
        assert!(result.end_time.is_some());
    }

    #[test]
    fn test_text_translation_job_from_minimal() {
        let info = TranslateTextTranslationJobInfo {
            job_id: None,
            job_name: None,
            job_status: None,
            source_language_code: None,
            target_language_codes: vec![],
            submitted_time: None,
            end_time: None,
        };
        let result = TranslateTextTranslationJob::from(info);
        assert!(result.job_id.is_none());
        assert!(result.job_status.is_none());
        assert!(result.target_language_codes.is_empty());
    }

    #[test]
    fn test_text_translation_job_failed() {
        let info = TranslateTextTranslationJobInfo {
            job_id: Some("job-fail".to_string()),
            job_name: Some("bad-job".to_string()),
            job_status: Some("FAILED".to_string()),
            source_language_code: Some("en".to_string()),
            target_language_codes: vec!["zh".to_string()],
            submitted_time: Some("2024-01-15T09:00:00Z".to_string()),
            end_time: None,
        };
        let result = TranslateTextTranslationJob::from(info);
        assert_eq!(result.job_status, Some("FAILED".to_string()));
        assert!(result.end_time.is_none());
    }
}
