use async_graphql::SimpleObject;

use crate::aws::polly::{PollyLexiconInfo, PollySpeechSynthesisTaskInfo, PollyVoiceInfo};

#[derive(SimpleObject, Clone)]
pub struct PollyVoice {
    pub voice_id: Option<String>,
    pub language_code: Option<String>,
    pub language_name: Option<String>,
    pub name: Option<String>,
    pub gender: Option<String>,
    pub additional_language_codes: Vec<String>,
    pub supported_engines: Vec<String>,
}

impl From<PollyVoiceInfo> for PollyVoice {
    fn from(v: PollyVoiceInfo) -> Self {
        Self {
            voice_id: v.voice_id,
            language_code: v.language_code,
            language_name: v.language_name,
            name: v.name,
            gender: v.gender,
            additional_language_codes: v.additional_language_codes,
            supported_engines: v.supported_engines,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct PollyLexiconAttributes {
    pub alphabet: Option<String>,
    pub language_code: Option<String>,
    pub last_modified: Option<String>,
    pub lexeme_count: Option<i32>,
    pub size: Option<i32>,
}

#[derive(SimpleObject, Clone)]
pub struct PollyLexicon {
    pub name: Option<String>,
    pub attributes: Option<PollyLexiconAttributes>,
}

impl From<PollyLexiconInfo> for PollyLexicon {
    fn from(lex: PollyLexiconInfo) -> Self {
        let has_attrs = lex.alphabet.is_some()
            || lex.language_code.is_some()
            || lex.last_modified.is_some()
            || lex.lexeme_count.is_some()
            || lex.size.is_some();
        let attributes = if has_attrs {
            Some(PollyLexiconAttributes {
                alphabet: lex.alphabet,
                language_code: lex.language_code,
                last_modified: lex.last_modified,
                lexeme_count: lex.lexeme_count,
                size: lex.size,
            })
        } else {
            None
        };
        Self {
            name: lex.name,
            attributes,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct PollySpeechSynthesisTask {
    pub task_id: Option<String>,
    pub task_status: Option<String>,
    pub task_status_reason: Option<String>,
    pub output_uri: Option<String>,
    pub creation_time: Option<String>,
    pub text_type: Option<String>,
    pub voice_id: Option<String>,
    pub output_format: Option<String>,
}

impl From<PollySpeechSynthesisTaskInfo> for PollySpeechSynthesisTask {
    fn from(t: PollySpeechSynthesisTaskInfo) -> Self {
        Self {
            task_id: t.task_id,
            task_status: t.task_status,
            task_status_reason: t.task_status_reason,
            output_uri: t.output_uri,
            creation_time: t.creation_time,
            text_type: t.text_type,
            voice_id: t.voice_id,
            output_format: t.output_format,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::polly::{PollyLexiconInfo, PollySpeechSynthesisTaskInfo, PollyVoiceInfo};

    #[test]
    fn test_polly_voice_from_full() {
        let info = PollyVoiceInfo {
            voice_id: Some("Joanna".to_string()),
            language_code: Some("en-US".to_string()),
            language_name: Some("US English".to_string()),
            name: Some("Joanna".to_string()),
            gender: Some("Female".to_string()),
            additional_language_codes: vec!["es-US".to_string()],
            supported_engines: vec!["standard".to_string(), "neural".to_string()],
        };
        let result = PollyVoice::from(info);
        assert_eq!(result.voice_id, Some("Joanna".to_string()));
        assert_eq!(result.language_code, Some("en-US".to_string()));
        assert_eq!(result.gender, Some("Female".to_string()));
        assert_eq!(result.additional_language_codes.len(), 1);
        assert_eq!(result.supported_engines.len(), 2);
    }

    #[test]
    fn test_polly_voice_from_minimal() {
        let info = PollyVoiceInfo {
            voice_id: None,
            language_code: None,
            language_name: None,
            name: None,
            gender: None,
            additional_language_codes: vec![],
            supported_engines: vec![],
        };
        let result = PollyVoice::from(info);
        assert!(result.voice_id.is_none());
        assert!(result.additional_language_codes.is_empty());
        assert!(result.supported_engines.is_empty());
    }

    #[test]
    fn test_polly_lexicon_from_full() {
        let info = PollyLexiconInfo {
            name: Some("my-lexicon".to_string()),
            alphabet: Some("ipa".to_string()),
            language_code: Some("en-US".to_string()),
            last_modified: Some("2024-01-15T10:00:00Z".to_string()),
            lexeme_count: Some(5),
            size: Some(1024),
        };
        let result = PollyLexicon::from(info);
        assert_eq!(result.name, Some("my-lexicon".to_string()));
        assert!(result.attributes.is_some());
        let attrs = result.attributes.unwrap();
        assert_eq!(attrs.alphabet, Some("ipa".to_string()));
        assert_eq!(attrs.lexeme_count, Some(5));
        assert_eq!(attrs.size, Some(1024));
    }

    #[test]
    fn test_polly_lexicon_from_no_attrs() {
        let info = PollyLexiconInfo {
            name: Some("empty-lexicon".to_string()),
            alphabet: None,
            language_code: None,
            last_modified: None,
            lexeme_count: None,
            size: None,
        };
        let result = PollyLexicon::from(info);
        assert_eq!(result.name, Some("empty-lexicon".to_string()));
        assert!(result.attributes.is_none());
    }

    #[test]
    fn test_polly_speech_synthesis_task_from_full() {
        let info = PollySpeechSynthesisTaskInfo {
            task_id: Some("abc-123".to_string()),
            task_status: Some("completed".to_string()),
            task_status_reason: None,
            output_uri: Some("s3://bucket/output.mp3".to_string()),
            creation_time: Some("2024-01-15T10:00:00Z".to_string()),
            text_type: Some("text".to_string()),
            voice_id: Some("Joanna".to_string()),
            output_format: Some("mp3".to_string()),
        };
        let result = PollySpeechSynthesisTask::from(info);
        assert_eq!(result.task_id, Some("abc-123".to_string()));
        assert_eq!(result.task_status, Some("completed".to_string()));
        assert_eq!(
            result.output_uri,
            Some("s3://bucket/output.mp3".to_string())
        );
        assert_eq!(result.voice_id, Some("Joanna".to_string()));
        assert!(result.task_status_reason.is_none());
    }

    #[test]
    fn test_polly_speech_synthesis_task_failed() {
        let info = PollySpeechSynthesisTaskInfo {
            task_id: Some("failed-task".to_string()),
            task_status: Some("failed".to_string()),
            task_status_reason: Some("Invalid text".to_string()),
            output_uri: None,
            creation_time: None,
            text_type: None,
            voice_id: None,
            output_format: None,
        };
        let result = PollySpeechSynthesisTask::from(info);
        assert_eq!(result.task_status, Some("failed".to_string()));
        assert_eq!(
            result.task_status_reason,
            Some("Invalid text".to_string())
        );
        assert!(result.output_uri.is_none());
    }
}
