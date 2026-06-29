use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct ComprehendEntityRecognizerInfo {
    pub entity_recognizer_arn: Option<String>,
    pub language_code: Option<String>,
    pub status: Option<String>,
    pub submit_time: Option<String>,
    pub end_time: Option<String>,
    pub training_start_time: Option<String>,
    pub training_end_time: Option<String>,
}

pub struct ComprehendDocumentClassifierInfo {
    pub document_classifier_arn: Option<String>,
    pub language_code: Option<String>,
    pub status: Option<String>,
    pub mode: Option<String>,
    pub submit_time: Option<String>,
    pub end_time: Option<String>,
}

pub struct ComprehendEndpointInfo {
    pub endpoint_arn: Option<String>,
    pub model_arn: Option<String>,
    pub status: Option<String>,
    pub current_inference_units: Option<i32>,
    pub creation_time: Option<String>,
    pub last_modified_time: Option<String>,
}

pub struct ComprehendClient {
    inner: aws_sdk_comprehend::Client,
}

impl ComprehendClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_comprehend::Client::new(config),
        }
    }

    pub async fn list_entity_recognizers(
        &self,
        status_filter: Option<String>,
    ) -> Result<Vec<ComprehendEntityRecognizerInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_entity_recognizers();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            if let Some(ref s) = status_filter {
                let filter = aws_sdk_comprehend::types::EntityRecognizerFilter::builder()
                    .status(aws_sdk_comprehend::types::ModelStatus::from(s.as_str()))
                    .build();
                req = req.filter(filter);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for er in output.entity_recognizer_properties_list() {
                items.push(ComprehendEntityRecognizerInfo {
                    entity_recognizer_arn: er.entity_recognizer_arn().map(|s| s.to_string()),
                    language_code: er.language_code().map(|c| c.as_str().to_string()),
                    status: er.status().map(|s| s.as_str().to_string()),
                    submit_time: er.submit_time().map(|t| t.to_string()),
                    end_time: er.end_time().map(|t| t.to_string()),
                    training_start_time: er.training_start_time().map(|t| t.to_string()),
                    training_end_time: er.training_end_time().map(|t| t.to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_document_classifiers(
        &self,
        status_filter: Option<String>,
    ) -> Result<Vec<ComprehendDocumentClassifierInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_document_classifiers();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            if let Some(ref s) = status_filter {
                let filter = aws_sdk_comprehend::types::DocumentClassifierFilter::builder()
                    .status(aws_sdk_comprehend::types::ModelStatus::from(s.as_str()))
                    .build();
                req = req.filter(filter);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for dc in output.document_classifier_properties_list() {
                items.push(ComprehendDocumentClassifierInfo {
                    document_classifier_arn: dc.document_classifier_arn().map(|s| s.to_string()),
                    language_code: dc.language_code().map(|c| c.as_str().to_string()),
                    status: dc.status().map(|s| s.as_str().to_string()),
                    mode: dc.mode().map(|m| m.as_str().to_string()),
                    submit_time: dc.submit_time().map(|t| t.to_string()),
                    end_time: dc.end_time().map(|t| t.to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_endpoints(&self) -> Result<Vec<ComprehendEndpointInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_endpoints();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for ep in output.endpoint_properties_list() {
                items.push(ComprehendEndpointInfo {
                    endpoint_arn: ep.endpoint_arn().map(|s| s.to_string()),
                    model_arn: ep.model_arn().map(|s| s.to_string()),
                    status: ep.status().map(|s| s.as_str().to_string()),
                    current_inference_units: ep.current_inference_units(),
                    creation_time: ep.creation_time().map(|t| t.to_string()),
                    last_modified_time: ep.last_modified_time().map(|t| t.to_string()),
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
