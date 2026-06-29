use async_graphql::SimpleObject;

use crate::aws::comprehend::{
    ComprehendDocumentClassifierInfo, ComprehendEndpointInfo, ComprehendEntityRecognizerInfo,
};

#[derive(SimpleObject, Clone)]
pub struct ComprehendEntityRecognizer {
    pub entity_recognizer_arn: Option<String>,
    pub language_code: Option<String>,
    pub status: Option<String>,
    pub submit_time: Option<String>,
    pub end_time: Option<String>,
    pub training_start_time: Option<String>,
    pub training_end_time: Option<String>,
}

impl From<ComprehendEntityRecognizerInfo> for ComprehendEntityRecognizer {
    fn from(er: ComprehendEntityRecognizerInfo) -> Self {
        Self {
            entity_recognizer_arn: er.entity_recognizer_arn,
            language_code: er.language_code,
            status: er.status,
            submit_time: er.submit_time,
            end_time: er.end_time,
            training_start_time: er.training_start_time,
            training_end_time: er.training_end_time,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ComprehendDocumentClassifier {
    pub document_classifier_arn: Option<String>,
    pub language_code: Option<String>,
    pub status: Option<String>,
    pub mode: Option<String>,
    pub submit_time: Option<String>,
    pub end_time: Option<String>,
}

impl From<ComprehendDocumentClassifierInfo> for ComprehendDocumentClassifier {
    fn from(dc: ComprehendDocumentClassifierInfo) -> Self {
        Self {
            document_classifier_arn: dc.document_classifier_arn,
            language_code: dc.language_code,
            status: dc.status,
            mode: dc.mode,
            submit_time: dc.submit_time,
            end_time: dc.end_time,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ComprehendEndpoint {
    pub endpoint_arn: Option<String>,
    pub model_arn: Option<String>,
    pub status: Option<String>,
    pub current_inference_units: Option<i32>,
    pub creation_time: Option<String>,
    pub last_modified_time: Option<String>,
}

impl From<ComprehendEndpointInfo> for ComprehendEndpoint {
    fn from(ep: ComprehendEndpointInfo) -> Self {
        Self {
            endpoint_arn: ep.endpoint_arn,
            model_arn: ep.model_arn,
            status: ep.status,
            current_inference_units: ep.current_inference_units,
            creation_time: ep.creation_time,
            last_modified_time: ep.last_modified_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::comprehend::{
        ComprehendDocumentClassifierInfo, ComprehendEndpointInfo, ComprehendEntityRecognizerInfo,
    };

    #[test]
    fn test_entity_recognizer_from_full() {
        let info = ComprehendEntityRecognizerInfo {
            entity_recognizer_arn: Some(
                "arn:aws:comprehend:us-east-1:123456789012:entity-recognizer/my-recognizer"
                    .to_string(),
            ),
            language_code: Some("en".to_string()),
            status: Some("TRAINED".to_string()),
            submit_time: Some("2024-01-15T10:30:00Z".to_string()),
            end_time: Some("2024-01-15T11:30:00Z".to_string()),
            training_start_time: Some("2024-01-15T10:35:00Z".to_string()),
            training_end_time: Some("2024-01-15T11:25:00Z".to_string()),
        };
        let result = ComprehendEntityRecognizer::from(info);
        assert!(result.entity_recognizer_arn.is_some());
        assert_eq!(result.language_code, Some("en".to_string()));
        assert_eq!(result.status, Some("TRAINED".to_string()));
        assert!(result.submit_time.is_some());
        assert!(result.training_start_time.is_some());
        assert!(result.training_end_time.is_some());
    }

    #[test]
    fn test_entity_recognizer_from_minimal() {
        let info = ComprehendEntityRecognizerInfo {
            entity_recognizer_arn: None,
            language_code: None,
            status: None,
            submit_time: None,
            end_time: None,
            training_start_time: None,
            training_end_time: None,
        };
        let result = ComprehendEntityRecognizer::from(info);
        assert!(result.entity_recognizer_arn.is_none());
        assert!(result.language_code.is_none());
        assert!(result.status.is_none());
        assert!(result.training_start_time.is_none());
    }

    #[test]
    fn test_document_classifier_from_full() {
        let info = ComprehendDocumentClassifierInfo {
            document_classifier_arn: Some(
                "arn:aws:comprehend:us-east-1:123456789012:document-classifier/my-classifier"
                    .to_string(),
            ),
            language_code: Some("en".to_string()),
            status: Some("TRAINED".to_string()),
            mode: Some("MULTI_CLASS".to_string()),
            submit_time: Some("2024-01-15T10:30:00Z".to_string()),
            end_time: Some("2024-01-15T11:30:00Z".to_string()),
        };
        let result = ComprehendDocumentClassifier::from(info);
        assert!(result.document_classifier_arn.is_some());
        assert_eq!(result.language_code, Some("en".to_string()));
        assert_eq!(result.status, Some("TRAINED".to_string()));
        assert_eq!(result.mode, Some("MULTI_CLASS".to_string()));
        assert!(result.submit_time.is_some());
    }

    #[test]
    fn test_document_classifier_from_multi_label() {
        let info = ComprehendDocumentClassifierInfo {
            document_classifier_arn: Some("arn:aws:comprehend:us-east-1:123:dc/ml".to_string()),
            language_code: Some("es".to_string()),
            status: Some("IN_ERROR".to_string()),
            mode: Some("MULTI_LABEL".to_string()),
            submit_time: None,
            end_time: None,
        };
        let result = ComprehendDocumentClassifier::from(info);
        assert_eq!(result.mode, Some("MULTI_LABEL".to_string()));
        assert_eq!(result.status, Some("IN_ERROR".to_string()));
        assert!(result.submit_time.is_none());
    }

    #[test]
    fn test_endpoint_from_full() {
        let info = ComprehendEndpointInfo {
            endpoint_arn: Some(
                "arn:aws:comprehend:us-east-1:123456789012:document-classifier-endpoint/my-ep"
                    .to_string(),
            ),
            model_arn: Some(
                "arn:aws:comprehend:us-east-1:123456789012:document-classifier/my-classifier"
                    .to_string(),
            ),
            status: Some("IN_SERVICE".to_string()),
            current_inference_units: Some(1),
            creation_time: Some("2024-01-15T10:30:00Z".to_string()),
            last_modified_time: Some("2024-01-16T12:00:00Z".to_string()),
        };
        let result = ComprehendEndpoint::from(info);
        assert!(result.endpoint_arn.is_some());
        assert!(result.model_arn.is_some());
        assert_eq!(result.status, Some("IN_SERVICE".to_string()));
        assert_eq!(result.current_inference_units, Some(1));
        assert!(result.creation_time.is_some());
        assert!(result.last_modified_time.is_some());
    }

    #[test]
    fn test_endpoint_from_minimal() {
        let info = ComprehendEndpointInfo {
            endpoint_arn: None,
            model_arn: None,
            status: None,
            current_inference_units: None,
            creation_time: None,
            last_modified_time: None,
        };
        let result = ComprehendEndpoint::from(info);
        assert!(result.endpoint_arn.is_none());
        assert!(result.model_arn.is_none());
        assert!(result.current_inference_units.is_none());
    }
}
