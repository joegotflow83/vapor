use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct RekognitionCollectionInfo {
    pub collection_id: Option<String>,
    pub collection_arn: Option<String>,
    pub creation_timestamp: Option<String>,
    pub face_model_version: Option<String>,
    pub face_count: Option<i64>,
}

pub struct RekognitionDatasetInfo {
    pub creation_timestamp: Option<String>,
    pub dataset_type: Option<String>,
    pub dataset_arn: Option<String>,
    pub status: Option<String>,
}

pub struct RekognitionProjectInfo {
    pub project_arn: Option<String>,
    pub creation_timestamp: Option<String>,
    pub status: Option<String>,
    pub project_name: Option<String>,
    pub datasets: Vec<RekognitionDatasetInfo>,
    pub feature: Option<String>,
}

pub struct RekognitionStreamProcessorInfo {
    pub name: Option<String>,
    pub status: Option<String>,
    pub stream_processor_arn: Option<String>,
}

pub struct RekognitionClient {
    inner: aws_sdk_rekognition::Client,
}

impl RekognitionClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_rekognition::Client::new(config),
        }
    }

    pub async fn list_collections(&self) -> Result<Vec<RekognitionCollectionInfo>, VaporError> {
        let mut ids = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_collections();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for id in output.collection_ids() {
                ids.push(id.to_string());
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        // N+1: describe each collection for face count, model version, and ARN
        let mut items = Vec::new();
        for collection_id in ids {
            let desc = self
                .inner
                .describe_collection()
                .collection_id(&collection_id)
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            items.push(RekognitionCollectionInfo {
                collection_id: Some(collection_id),
                collection_arn: desc.collection_arn().map(|s| s.to_string()),
                creation_timestamp: desc.creation_timestamp().map(|t| t.to_string()),
                face_model_version: desc.face_model_version().map(|s| s.to_string()),
                face_count: desc.face_count(),
            });
        }

        Ok(items)
    }

    pub async fn describe_projects(&self) -> Result<Vec<RekognitionProjectInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_projects();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for proj in output.project_descriptions() {
                let datasets = proj
                    .datasets()
                    .iter()
                    .map(|d| RekognitionDatasetInfo {
                        creation_timestamp: d.creation_timestamp().map(|t| t.to_string()),
                        dataset_type: d.dataset_type().map(|dt| dt.as_str().to_string()),
                        dataset_arn: d.dataset_arn().map(|s| s.to_string()),
                        status: d.status().map(|s| s.as_str().to_string()),
                    })
                    .collect();

                items.push(RekognitionProjectInfo {
                    project_arn: proj.project_arn().map(|s| s.to_string()),
                    creation_timestamp: proj.creation_timestamp().map(|t| t.to_string()),
                    status: proj.status().map(|s| s.as_str().to_string()),
                    // ProjectDescription exposes only project_arn, no separate name.
                    project_name: None,
                    datasets,
                    feature: proj.feature().map(|f| f.as_str().to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_stream_processors(
        &self,
    ) -> Result<Vec<RekognitionStreamProcessorInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_stream_processors();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for sp in output.stream_processors() {
                let name = sp.name().map(|s| s.to_string());
                // N+1: describe to get the ARN (not available in list response)
                let stream_processor_arn = if let Some(ref n) = name {
                    self.inner
                        .describe_stream_processor()
                        .name(n)
                        .send()
                        .await
                        .ok()
                        .and_then(|d| d.stream_processor_arn().map(|s| s.to_string()))
                } else {
                    None
                };

                items.push(RekognitionStreamProcessorInfo {
                    name,
                    status: sp.status().map(|s| s.as_str().to_string()),
                    stream_processor_arn,
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
