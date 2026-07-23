use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

use crate::schema::time::to_utc;

#[derive(SimpleObject, Clone)]
pub struct ComprehendEntityRecognizer {
    pub entity_recognizer_arn: Option<String>,
    pub language_code: Option<String>,
    pub status: Option<String>,
    pub submit_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub training_start_time: Option<DateTime<Utc>>,
    pub training_end_time: Option<DateTime<Utc>>,
}

impl From<aws_sdk_comprehend::types::EntityRecognizerProperties> for ComprehendEntityRecognizer {
    fn from(er: aws_sdk_comprehend::types::EntityRecognizerProperties) -> Self {
        Self {
            entity_recognizer_arn: er.entity_recognizer_arn,
            language_code: er.language_code.map(|c| c.as_str().to_string()),
            status: er.status.map(|s| s.as_str().to_string()),
            submit_time: to_utc(er.submit_time.as_ref()),
            end_time: to_utc(er.end_time.as_ref()),
            training_start_time: to_utc(er.training_start_time.as_ref()),
            training_end_time: to_utc(er.training_end_time.as_ref()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ComprehendDocumentClassifier {
    pub document_classifier_arn: Option<String>,
    pub language_code: Option<String>,
    pub status: Option<String>,
    pub mode: Option<String>,
    pub submit_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

impl From<aws_sdk_comprehend::types::DocumentClassifierProperties>
    for ComprehendDocumentClassifier
{
    fn from(dc: aws_sdk_comprehend::types::DocumentClassifierProperties) -> Self {
        Self {
            document_classifier_arn: dc.document_classifier_arn,
            language_code: dc.language_code.map(|c| c.as_str().to_string()),
            status: dc.status.map(|s| s.as_str().to_string()),
            mode: dc.mode.map(|m| m.as_str().to_string()),
            submit_time: to_utc(dc.submit_time.as_ref()),
            end_time: to_utc(dc.end_time.as_ref()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ComprehendEndpoint {
    pub endpoint_arn: Option<String>,
    pub model_arn: Option<String>,
    pub status: Option<String>,
    pub current_inference_units: Option<i32>,
    pub creation_time: Option<DateTime<Utc>>,
    pub last_modified_time: Option<DateTime<Utc>>,
}

impl From<aws_sdk_comprehend::types::EndpointProperties> for ComprehendEndpoint {
    fn from(ep: aws_sdk_comprehend::types::EndpointProperties) -> Self {
        Self {
            endpoint_arn: ep.endpoint_arn,
            model_arn: ep.model_arn,
            status: ep.status.map(|s| s.as_str().to_string()),
            current_inference_units: ep.current_inference_units,
            creation_time: to_utc(ep.creation_time.as_ref()),
            last_modified_time: to_utc(ep.last_modified_time.as_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_smithy_types::DateTime as SmithyDateTime;

    #[test]
    fn test_entity_recognizer_from_full() {
        let info = aws_sdk_comprehend::types::EntityRecognizerProperties::builder()
            .entity_recognizer_arn(
                "arn:aws:comprehend:us-east-1:123456789012:entity-recognizer/my-recognizer",
            )
            .language_code(aws_sdk_comprehend::types::LanguageCode::En)
            .status(aws_sdk_comprehend::types::ModelStatus::Trained)
            .submit_time(SmithyDateTime::from_secs(1_705_314_600))
            .end_time(SmithyDateTime::from_secs(1_705_318_200))
            .training_start_time(SmithyDateTime::from_secs(1_705_314_900))
            .training_end_time(SmithyDateTime::from_secs(1_705_317_900))
            .build();
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
        let info = aws_sdk_comprehend::types::EntityRecognizerProperties::builder().build();
        let result = ComprehendEntityRecognizer::from(info);
        assert!(result.entity_recognizer_arn.is_none());
        assert!(result.language_code.is_none());
        assert!(result.status.is_none());
        assert!(result.training_start_time.is_none());
    }

    #[test]
    fn test_document_classifier_from_full() {
        let info = aws_sdk_comprehend::types::DocumentClassifierProperties::builder()
            .document_classifier_arn(
                "arn:aws:comprehend:us-east-1:123456789012:document-classifier/my-classifier",
            )
            .language_code(aws_sdk_comprehend::types::LanguageCode::En)
            .status(aws_sdk_comprehend::types::ModelStatus::Trained)
            .mode(aws_sdk_comprehend::types::DocumentClassifierMode::MultiClass)
            .submit_time(SmithyDateTime::from_secs(1_705_314_600))
            .end_time(SmithyDateTime::from_secs(1_705_318_200))
            .build();
        let result = ComprehendDocumentClassifier::from(info);
        assert!(result.document_classifier_arn.is_some());
        assert_eq!(result.language_code, Some("en".to_string()));
        assert_eq!(result.status, Some("TRAINED".to_string()));
        assert_eq!(result.mode, Some("MULTI_CLASS".to_string()));
        assert!(result.submit_time.is_some());
    }

    #[test]
    fn test_document_classifier_from_multi_label() {
        let info = aws_sdk_comprehend::types::DocumentClassifierProperties::builder()
            .document_classifier_arn("arn:aws:comprehend:us-east-1:123:dc/ml")
            .language_code(aws_sdk_comprehend::types::LanguageCode::Es)
            .status(aws_sdk_comprehend::types::ModelStatus::InError)
            .mode(aws_sdk_comprehend::types::DocumentClassifierMode::MultiLabel)
            .build();
        let result = ComprehendDocumentClassifier::from(info);
        assert_eq!(result.mode, Some("MULTI_LABEL".to_string()));
        assert_eq!(result.status, Some("IN_ERROR".to_string()));
        assert!(result.submit_time.is_none());
    }

    #[test]
    fn test_endpoint_from_full() {
        let info = aws_sdk_comprehend::types::EndpointProperties::builder()
            .endpoint_arn(
                "arn:aws:comprehend:us-east-1:123456789012:document-classifier-endpoint/my-ep",
            )
            .model_arn(
                "arn:aws:comprehend:us-east-1:123456789012:document-classifier/my-classifier",
            )
            .status(aws_sdk_comprehend::types::EndpointStatus::InService)
            .current_inference_units(1)
            .creation_time(SmithyDateTime::from_secs(1_705_314_600))
            .last_modified_time(SmithyDateTime::from_secs(1_705_406_400))
            .build();
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
        let info = aws_sdk_comprehend::types::EndpointProperties::builder().build();
        let result = ComprehendEndpoint::from(info);
        assert!(result.endpoint_arn.is_none());
        assert!(result.model_arn.is_none());
        assert!(result.current_inference_units.is_none());
    }
}
