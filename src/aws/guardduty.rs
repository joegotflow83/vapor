use aws_config::SdkConfig;
use aws_sdk_guardduty::types::FindingCriteria;

use crate::error::VaporError;

pub struct GuardDutyClient {
    inner: aws_sdk_guardduty::Client,
}

impl GuardDutyClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_guardduty::Client::new(config),
        }
    }

    pub async fn list_detectors(&self) -> Result<Vec<String>, VaporError> {
        let mut ids: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_detectors();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            ids.extend(output.detector_ids().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(ids)
    }

    pub async fn get_detector(
        &self,
        detector_id: &str,
    ) -> Result<aws_sdk_guardduty::operation::get_detector::GetDetectorOutput, VaporError> {
        self.inner
            .get_detector()
            .detector_id(detector_id)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn list_findings(
        &self,
        detector_id: &str,
        criteria: Option<FindingCriteria>,
    ) -> Result<Vec<String>, VaporError> {
        let mut ids: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_findings().detector_id(detector_id);
            if let Some(ref c) = criteria {
                req = req.finding_criteria(c.clone());
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            ids.extend(output.finding_ids().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(ids)
    }

    pub async fn get_findings(
        &self,
        detector_id: &str,
        finding_ids: Vec<String>,
    ) -> Result<Vec<aws_sdk_guardduty::types::Finding>, VaporError> {
        let output = self
            .inner
            .get_findings()
            .detector_id(detector_id)
            .set_finding_ids(Some(finding_ids))
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.findings().to_vec())
    }
}
