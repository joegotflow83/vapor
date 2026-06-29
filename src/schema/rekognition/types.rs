use async_graphql::SimpleObject;

use crate::aws::rekognition::{
    RekognitionCollectionInfo, RekognitionDatasetInfo, RekognitionProjectInfo,
    RekognitionStreamProcessorInfo,
};

#[derive(SimpleObject, Clone)]
pub struct RekognitionCollection {
    pub collection_id: Option<String>,
    pub collection_arn: Option<String>,
    pub creation_timestamp: Option<String>,
    pub face_model_version: Option<String>,
    pub face_count: Option<i64>,
}

impl From<RekognitionCollectionInfo> for RekognitionCollection {
    fn from(info: RekognitionCollectionInfo) -> Self {
        Self {
            collection_id: info.collection_id,
            collection_arn: info.collection_arn,
            creation_timestamp: info.creation_timestamp,
            face_model_version: info.face_model_version,
            face_count: info.face_count,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RekognitionDataset {
    pub creation_timestamp: Option<String>,
    pub dataset_type: Option<String>,
    pub dataset_arn: Option<String>,
    pub status: Option<String>,
}

impl From<RekognitionDatasetInfo> for RekognitionDataset {
    fn from(info: RekognitionDatasetInfo) -> Self {
        Self {
            creation_timestamp: info.creation_timestamp,
            dataset_type: info.dataset_type,
            dataset_arn: info.dataset_arn,
            status: info.status,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RekognitionProject {
    pub project_arn: Option<String>,
    pub creation_timestamp: Option<String>,
    pub status: Option<String>,
    pub project_name: Option<String>,
    pub datasets: Vec<RekognitionDataset>,
    pub feature: Option<String>,
}

impl From<RekognitionProjectInfo> for RekognitionProject {
    fn from(info: RekognitionProjectInfo) -> Self {
        Self {
            project_arn: info.project_arn,
            creation_timestamp: info.creation_timestamp,
            status: info.status,
            project_name: info.project_name,
            datasets: info.datasets.into_iter().map(RekognitionDataset::from).collect(),
            feature: info.feature,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RekognitionStreamProcessor {
    pub name: Option<String>,
    pub status: Option<String>,
    pub stream_processor_arn: Option<String>,
}

impl From<RekognitionStreamProcessorInfo> for RekognitionStreamProcessor {
    fn from(info: RekognitionStreamProcessorInfo) -> Self {
        Self {
            name: info.name,
            status: info.status,
            stream_processor_arn: info.stream_processor_arn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::rekognition::{
        RekognitionCollectionInfo, RekognitionDatasetInfo, RekognitionProjectInfo,
        RekognitionStreamProcessorInfo,
    };

    #[test]
    fn test_collection_from_full() {
        let info = RekognitionCollectionInfo {
            collection_id: Some("my-collection".to_string()),
            collection_arn: Some(
                "arn:aws:rekognition:us-east-1:123456789012:collection/my-collection".to_string(),
            ),
            creation_timestamp: Some("2024-01-15T10:30:00Z".to_string()),
            face_model_version: Some("7.0".to_string()),
            face_count: Some(42),
        };
        let result = RekognitionCollection::from(info);
        assert_eq!(result.collection_id, Some("my-collection".to_string()));
        assert!(result.collection_arn.is_some());
        assert_eq!(result.face_model_version, Some("7.0".to_string()));
        assert_eq!(result.face_count, Some(42));
    }

    #[test]
    fn test_collection_from_minimal() {
        let info = RekognitionCollectionInfo {
            collection_id: Some("empty-collection".to_string()),
            collection_arn: None,
            creation_timestamp: None,
            face_model_version: None,
            face_count: Some(0),
        };
        let result = RekognitionCollection::from(info);
        assert_eq!(result.collection_id, Some("empty-collection".to_string()));
        assert!(result.collection_arn.is_none());
        assert_eq!(result.face_count, Some(0));
    }

    #[test]
    fn test_dataset_from() {
        let info = RekognitionDatasetInfo {
            creation_timestamp: Some("2024-01-10T08:00:00Z".to_string()),
            dataset_type: Some("TRAIN".to_string()),
            dataset_arn: Some(
                "arn:aws:rekognition:us-east-1:123456789012:project/my-project/dataset/train/1"
                    .to_string(),
            ),
            status: Some("CREATE_COMPLETE".to_string()),
        };
        let result = RekognitionDataset::from(info);
        assert_eq!(result.dataset_type, Some("TRAIN".to_string()));
        assert_eq!(result.status, Some("CREATE_COMPLETE".to_string()));
        assert!(result.dataset_arn.is_some());
    }

    #[test]
    fn test_project_from_full() {
        let info = RekognitionProjectInfo {
            project_arn: Some(
                "arn:aws:rekognition:us-east-1:123456789012:project/my-project".to_string(),
            ),
            creation_timestamp: Some("2024-01-05T09:00:00Z".to_string()),
            status: Some("CREATED".to_string()),
            project_name: Some("my-project".to_string()),
            datasets: vec![
                RekognitionDatasetInfo {
                    creation_timestamp: Some("2024-01-10T08:00:00Z".to_string()),
                    dataset_type: Some("TRAIN".to_string()),
                    dataset_arn: Some("arn:aws:rekognition:us-east-1:123:project/my-project/dataset/train/1".to_string()),
                    status: Some("CREATE_COMPLETE".to_string()),
                },
                RekognitionDatasetInfo {
                    creation_timestamp: Some("2024-01-10T08:00:00Z".to_string()),
                    dataset_type: Some("TEST".to_string()),
                    dataset_arn: Some("arn:aws:rekognition:us-east-1:123:project/my-project/dataset/test/1".to_string()),
                    status: Some("CREATE_COMPLETE".to_string()),
                },
            ],
            feature: Some("CUSTOM_LABELS".to_string()),
        };
        let result = RekognitionProject::from(info);
        assert_eq!(result.project_name, Some("my-project".to_string()));
        assert_eq!(result.status, Some("CREATED".to_string()));
        assert_eq!(result.datasets.len(), 2);
        assert_eq!(result.datasets[0].dataset_type, Some("TRAIN".to_string()));
        assert_eq!(result.datasets[1].dataset_type, Some("TEST".to_string()));
        assert_eq!(result.feature, Some("CUSTOM_LABELS".to_string()));
    }

    #[test]
    fn test_project_from_no_datasets() {
        let info = RekognitionProjectInfo {
            project_arn: Some("arn:aws:rekognition:us-east-1:123:project/empty".to_string()),
            creation_timestamp: None,
            status: Some("CREATING".to_string()),
            project_name: Some("empty".to_string()),
            datasets: vec![],
            feature: Some("CONTENT_MODERATION".to_string()),
        };
        let result = RekognitionProject::from(info);
        assert_eq!(result.datasets.len(), 0);
        assert_eq!(result.status, Some("CREATING".to_string()));
        assert_eq!(result.feature, Some("CONTENT_MODERATION".to_string()));
    }

    #[test]
    fn test_stream_processor_from_full() {
        let info = RekognitionStreamProcessorInfo {
            name: Some("my-processor".to_string()),
            status: Some("RUNNING".to_string()),
            stream_processor_arn: Some(
                "arn:aws:rekognition:us-east-1:123456789012:streamprocessor/my-processor"
                    .to_string(),
            ),
        };
        let result = RekognitionStreamProcessor::from(info);
        assert_eq!(result.name, Some("my-processor".to_string()));
        assert_eq!(result.status, Some("RUNNING".to_string()));
        assert!(result.stream_processor_arn.is_some());
    }

    #[test]
    fn test_stream_processor_from_stopped() {
        let info = RekognitionStreamProcessorInfo {
            name: Some("stopped-processor".to_string()),
            status: Some("STOPPED".to_string()),
            stream_processor_arn: None,
        };
        let result = RekognitionStreamProcessor::from(info);
        assert_eq!(result.status, Some("STOPPED".to_string()));
        assert!(result.stream_processor_arn.is_none());
    }
}
