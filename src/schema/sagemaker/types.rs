use async_graphql::SimpleObject;
use aws_sdk_sagemaker::operation::describe_endpoint::DescribeEndpointOutput;
use aws_sdk_sagemaker::operation::describe_training_job::DescribeTrainingJobOutput;
use aws_sdk_sagemaker::types::ModelSummary;
use chrono::{DateTime, Utc};

use crate::schema::time::to_utc;

#[derive(SimpleObject, Clone)]
pub struct SageMakerEndpoint {
    pub name: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub endpoint_config_name: Option<String>,
    pub creation_time: Option<DateTime<Utc>>,
    pub last_modified_time: Option<DateTime<Utc>>,
}

impl From<DescribeEndpointOutput> for SageMakerEndpoint {
    fn from(e: DescribeEndpointOutput) -> Self {
        Self {
            name: e.endpoint_name().unwrap_or_default().to_string(),
            arn: e.endpoint_arn().map(|s| s.to_string()),
            status: e.endpoint_status().map(|s| s.as_str().to_string()),
            endpoint_config_name: e.endpoint_config_name().map(|s| s.to_string()),
            creation_time: to_utc(e.creation_time()),
            last_modified_time: to_utc(e.last_modified_time()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SageMakerTrainingJob {
    pub name: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub creation_time: Option<DateTime<Utc>>,
    pub training_end_time: Option<DateTime<Utc>>,
    pub instance_type: Option<String>,
    pub instance_count: Option<i32>,
}

impl From<DescribeTrainingJobOutput> for SageMakerTrainingJob {
    fn from(j: DescribeTrainingJobOutput) -> Self {
        let rc = j.resource_config();
        Self {
            name: j.training_job_name().unwrap_or_default().to_string(),
            arn: j.training_job_arn().map(|s| s.to_string()),
            status: j.training_job_status().map(|s| s.as_str().to_string()),
            creation_time: to_utc(j.creation_time()),
            training_end_time: to_utc(j.training_end_time()),
            instance_type: rc
                .and_then(|c| c.instance_type())
                .map(|it| it.as_str().to_string()),
            instance_count: rc.and_then(|c| c.instance_count()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SageMakerModel {
    pub name: String,
    pub arn: Option<String>,
    pub creation_time: Option<DateTime<Utc>>,
}

impl From<ModelSummary> for SageMakerModel {
    fn from(m: ModelSummary) -> Self {
        Self {
            name: m.model_name().unwrap_or_default().to_string(),
            arn: m.model_arn().map(|s| s.to_string()),
            creation_time: to_utc(m.creation_time()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sagemaker_endpoint_full() {
        let endpoint = SageMakerEndpoint {
            name: "my-endpoint".to_string(),
            arn: Some("arn:aws:sagemaker:us-east-1:123456789012:endpoint/my-endpoint".to_string()),
            status: Some("InService".to_string()),
            endpoint_config_name: Some("my-config".to_string()),
            creation_time: Some(
                DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            last_modified_time: Some(
                DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
        };

        assert_eq!(endpoint.name, "my-endpoint");
        assert_eq!(endpoint.status, Some("InService".to_string()));
        assert_eq!(endpoint.endpoint_config_name, Some("my-config".to_string()));
        assert!(endpoint.creation_time.is_some());
        assert!(endpoint.last_modified_time.is_some());
    }

    #[test]
    fn test_sagemaker_endpoint_minimal() {
        let endpoint = SageMakerEndpoint {
            name: "minimal-endpoint".to_string(),
            arn: None,
            status: None,
            endpoint_config_name: None,
            creation_time: None,
            last_modified_time: None,
        };

        assert_eq!(endpoint.name, "minimal-endpoint");
        assert!(endpoint.arn.is_none());
        assert!(endpoint.status.is_none());
        assert!(endpoint.endpoint_config_name.is_none());
        assert!(endpoint.creation_time.is_none());
        assert!(endpoint.last_modified_time.is_none());
    }

    #[test]
    fn test_sagemaker_training_job_full() {
        let job = SageMakerTrainingJob {
            name: "my-training-job".to_string(),
            arn: Some(
                "arn:aws:sagemaker:us-east-1:123456789012:training-job/my-training-job".to_string(),
            ),
            status: Some("Completed".to_string()),
            creation_time: Some(
                DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            training_end_time: Some(
                DateTime::parse_from_rfc3339("2024-01-01T02:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            instance_type: Some("ml.m5.xlarge".to_string()),
            instance_count: Some(2),
        };

        assert_eq!(job.name, "my-training-job");
        assert_eq!(job.status, Some("Completed".to_string()));
        assert_eq!(job.instance_type, Some("ml.m5.xlarge".to_string()));
        assert_eq!(job.instance_count, Some(2));
        assert!(job.training_end_time.is_some());
    }

    #[test]
    fn test_sagemaker_training_job_minimal() {
        let job = SageMakerTrainingJob {
            name: "minimal-job".to_string(),
            arn: None,
            status: None,
            creation_time: None,
            training_end_time: None,
            instance_type: None,
            instance_count: None,
        };

        assert_eq!(job.name, "minimal-job");
        assert!(job.arn.is_none());
        assert!(job.status.is_none());
        assert!(job.instance_type.is_none());
        assert!(job.instance_count.is_none());
    }

    #[test]
    fn test_sagemaker_model_full() {
        let model = SageMakerModel {
            name: "my-model".to_string(),
            arn: Some("arn:aws:sagemaker:us-east-1:123456789012:model/my-model".to_string()),
            creation_time: Some(
                DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
        };

        assert_eq!(model.name, "my-model");
        assert!(model.arn.is_some());
        assert!(model.creation_time.is_some());
    }

    #[test]
    fn test_sagemaker_model_minimal() {
        let model = SageMakerModel {
            name: "minimal-model".to_string(),
            arn: None,
            creation_time: None,
        };

        assert_eq!(model.name, "minimal-model");
        assert!(model.arn.is_none());
        assert!(model.creation_time.is_none());
    }
}
