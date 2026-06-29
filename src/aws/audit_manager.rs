use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct AuditManagerClient {
    inner: aws_sdk_auditmanager::Client,
}

impl AuditManagerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_auditmanager::Client::new(config),
        }
    }

    pub async fn list_assessments(
        &self,
    ) -> Result<Vec<aws_sdk_auditmanager::types::AssessmentMetadataItem>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_assessments();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            match req.send().await {
                Ok(output) => {
                    items.extend(output.assessment_metadata().to_vec());
                    match output.next_token() {
                        Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                        _ => break,
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("AccessDeniedException") {
                        return Ok(vec![]);
                    }
                    return Err(VaporError::AwsSdk(err_str));
                }
            }
        }

        Ok(items)
    }

    pub async fn list_frameworks(
        &self,
        framework_type: Option<String>,
    ) -> Result<Vec<aws_sdk_auditmanager::types::AssessmentFrameworkMetadata>, VaporError> {
        let types_to_query: Vec<aws_sdk_auditmanager::types::FrameworkType> =
            match framework_type.as_deref() {
                Some("Standard") => vec![aws_sdk_auditmanager::types::FrameworkType::Standard],
                Some("Custom") => vec![aws_sdk_auditmanager::types::FrameworkType::Custom],
                _ => vec![
                    aws_sdk_auditmanager::types::FrameworkType::Standard,
                    aws_sdk_auditmanager::types::FrameworkType::Custom,
                ],
            };

        let mut all_items = Vec::new();
        for ftype in types_to_query {
            let items = self.list_frameworks_by_type(ftype).await?;
            all_items.extend(items);
        }
        Ok(all_items)
    }

    async fn list_frameworks_by_type(
        &self,
        framework_type: aws_sdk_auditmanager::types::FrameworkType,
    ) -> Result<Vec<aws_sdk_auditmanager::types::AssessmentFrameworkMetadata>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_assessment_frameworks()
                .framework_type(framework_type.clone());
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            match req.send().await {
                Ok(output) => {
                    items.extend(output.framework_metadata_list().to_vec());
                    match output.next_token() {
                        Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                        _ => break,
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("AccessDeniedException") {
                        return Ok(vec![]);
                    }
                    return Err(VaporError::AwsSdk(err_str));
                }
            }
        }

        Ok(items)
    }

    pub async fn list_controls(
        &self,
        control_type: Option<String>,
    ) -> Result<Vec<aws_sdk_auditmanager::types::ControlMetadata>, VaporError> {
        let types_to_query: Vec<aws_sdk_auditmanager::types::ControlType> =
            match control_type.as_deref() {
                Some("Standard") => vec![aws_sdk_auditmanager::types::ControlType::Standard],
                Some("Custom") => vec![aws_sdk_auditmanager::types::ControlType::Custom],
                _ => vec![
                    aws_sdk_auditmanager::types::ControlType::Standard,
                    aws_sdk_auditmanager::types::ControlType::Custom,
                ],
            };

        let mut all_items = Vec::new();
        for ctype in types_to_query {
            let items = self.list_controls_by_type(ctype).await?;
            all_items.extend(items);
        }
        Ok(all_items)
    }

    async fn list_controls_by_type(
        &self,
        control_type: aws_sdk_auditmanager::types::ControlType,
    ) -> Result<Vec<aws_sdk_auditmanager::types::ControlMetadata>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_controls()
                .control_type(control_type.clone());
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            match req.send().await {
                Ok(output) => {
                    items.extend(output.control_metadata_list().to_vec());
                    match output.next_token() {
                        Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                        _ => break,
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("AccessDeniedException") {
                        return Ok(vec![]);
                    }
                    return Err(VaporError::AwsSdk(err_str));
                }
            }
        }

        Ok(items)
    }
}
