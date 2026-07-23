use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

use crate::aws::rekognition::{RekognitionCollectionInfo, RekognitionStreamProcessorInfo};
use crate::schema::time::to_utc;

#[derive(SimpleObject, Clone)]
pub struct RekognitionCollection {
    pub collection_id: Option<String>,
    pub collection_arn: Option<String>,
    pub creation_timestamp: Option<DateTime<Utc>>,
    pub face_model_version: Option<String>,
    pub face_count: Option<i64>,
}

impl From<RekognitionCollectionInfo> for RekognitionCollection {
    fn from(info: RekognitionCollectionInfo) -> Self {
        Self {
            collection_id: info.collection_id,
            collection_arn: info.collection_arn,
            creation_timestamp: to_utc(info.creation_timestamp.as_ref()),
            face_model_version: info.face_model_version,
            face_count: info.face_count,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RekognitionDataset {
    pub creation_timestamp: Option<DateTime<Utc>>,
    pub dataset_type: Option<String>,
    pub dataset_arn: Option<String>,
    pub status: Option<String>,
}

impl From<aws_sdk_rekognition::types::DatasetMetadata> for RekognitionDataset {
    fn from(d: aws_sdk_rekognition::types::DatasetMetadata) -> Self {
        Self {
            creation_timestamp: to_utc(d.creation_timestamp.as_ref()),
            dataset_type: d.dataset_type.map(|dt| dt.as_str().to_string()),
            dataset_arn: d.dataset_arn,
            status: d.status.map(|s| s.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RekognitionProject {
    pub project_arn: Option<String>,
    pub creation_timestamp: Option<DateTime<Utc>>,
    pub status: Option<String>,
    pub project_name: Option<String>,
    pub datasets: Vec<RekognitionDataset>,
    pub feature: Option<String>,
}

impl From<aws_sdk_rekognition::types::ProjectDescription> for RekognitionProject {
    fn from(proj: aws_sdk_rekognition::types::ProjectDescription) -> Self {
        Self {
            project_arn: proj.project_arn,
            creation_timestamp: to_utc(proj.creation_timestamp.as_ref()),
            status: proj.status.map(|s| s.as_str().to_string()),
            // ProjectDescription exposes only project_arn, no separate name.
            project_name: None,
            datasets: proj
                .datasets
                .unwrap_or_default()
                .into_iter()
                .map(RekognitionDataset::from)
                .collect(),
            feature: proj.feature.map(|f| f.as_str().to_string()),
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
    use aws_smithy_types::DateTime as SmithyDateTime;

    #[test]
    fn test_collection_from_full() {
        let info = RekognitionCollectionInfo {
            collection_id: Some("my-collection".to_string()),
            collection_arn: Some(
                "arn:aws:rekognition:us-east-1:123456789012:collection/my-collection".to_string(),
            ),
            creation_timestamp: Some(SmithyDateTime::from_secs(1_705_314_600)),
            face_model_version: Some("7.0".to_string()),
            face_count: Some(42),
        };
        let result = RekognitionCollection::from(info);
        assert_eq!(result.collection_id, Some("my-collection".to_string()));
        assert!(result.collection_arn.is_some());
        assert!(result.creation_timestamp.is_some());
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
        assert!(result.creation_timestamp.is_none());
        assert_eq!(result.face_count, Some(0));
    }

    #[test]
    fn test_dataset_from() {
        let info = aws_sdk_rekognition::types::DatasetMetadata::builder()
            .creation_timestamp(SmithyDateTime::from_secs(1_705_000_000))
            .dataset_type(aws_sdk_rekognition::types::DatasetType::Train)
            .dataset_arn(
                "arn:aws:rekognition:us-east-1:123456789012:project/my-project/dataset/train/1",
            )
            .status(aws_sdk_rekognition::types::DatasetStatus::CreateComplete)
            .build();
        let result = RekognitionDataset::from(info);
        assert_eq!(result.dataset_type, Some("TRAIN".to_string()));
        assert_eq!(result.status, Some("CREATE_COMPLETE".to_string()));
        assert!(result.dataset_arn.is_some());
        assert!(result.creation_timestamp.is_some());
    }

    #[test]
    fn test_project_from_full() {
        let dataset1 = aws_sdk_rekognition::types::DatasetMetadata::builder()
            .creation_timestamp(SmithyDateTime::from_secs(1_705_000_000))
            .dataset_type(aws_sdk_rekognition::types::DatasetType::Train)
            .dataset_arn("arn:aws:rekognition:us-east-1:123:project/my-project/dataset/train/1")
            .status(aws_sdk_rekognition::types::DatasetStatus::CreateComplete)
            .build();
        let dataset2 = aws_sdk_rekognition::types::DatasetMetadata::builder()
            .creation_timestamp(SmithyDateTime::from_secs(1_705_000_000))
            .dataset_type(aws_sdk_rekognition::types::DatasetType::Test)
            .dataset_arn("arn:aws:rekognition:us-east-1:123:project/my-project/dataset/test/1")
            .status(aws_sdk_rekognition::types::DatasetStatus::CreateComplete)
            .build();
        let proj = aws_sdk_rekognition::types::ProjectDescription::builder()
            .project_arn("arn:aws:rekognition:us-east-1:123456789012:project/my-project")
            .creation_timestamp(SmithyDateTime::from_secs(1_704_442_800))
            .status(aws_sdk_rekognition::types::ProjectStatus::Created)
            .datasets(dataset1)
            .datasets(dataset2)
            .feature(aws_sdk_rekognition::types::CustomizationFeature::CustomLabels)
            .build();
        let result = RekognitionProject::from(proj);
        assert!(result.project_arn.is_some());
        assert!(result.creation_timestamp.is_some());
        assert_eq!(result.status, Some("CREATED".to_string()));
        assert_eq!(result.datasets.len(), 2);
        assert_eq!(result.datasets[0].dataset_type, Some("TRAIN".to_string()));
        assert_eq!(result.datasets[1].dataset_type, Some("TEST".to_string()));
        assert_eq!(result.feature, Some("CUSTOM_LABELS".to_string()));
        // ProjectDescription has no separate name field.
        assert!(result.project_name.is_none());
    }

    #[test]
    fn test_project_from_no_datasets() {
        let proj = aws_sdk_rekognition::types::ProjectDescription::builder()
            .project_arn("arn:aws:rekognition:us-east-1:123:project/empty")
            .status(aws_sdk_rekognition::types::ProjectStatus::Creating)
            .feature(aws_sdk_rekognition::types::CustomizationFeature::ContentModeration)
            .build();
        let result = RekognitionProject::from(proj);
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
