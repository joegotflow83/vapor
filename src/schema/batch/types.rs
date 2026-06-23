use async_graphql::SimpleObject;
use aws_sdk_batch::types::{ComputeEnvironmentDetail, JobDefinition, JobQueueDetail};

#[derive(SimpleObject, Clone)]
pub struct BatchJobQueue {
    pub name: String,
    pub arn: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub priority: i32,
}

impl From<JobQueueDetail> for BatchJobQueue {
    fn from(q: JobQueueDetail) -> Self {
        Self {
            name: q.job_queue_name().unwrap_or_default().to_string(),
            arn: q.job_queue_arn().map(|s| s.to_string()),
            state: q.state().map(|s| s.as_str().to_string()),
            status: q.status().map(|s| s.as_str().to_string()),
            priority: q.priority().unwrap_or(0),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BatchComputeEnvironment {
    pub name: String,
    pub arn: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub compute_type: Option<String>,
    pub instance_types: Vec<String>,
    pub max_vcpus: Option<i32>,
    pub desired_vcpus: Option<i32>,
}

impl From<ComputeEnvironmentDetail> for BatchComputeEnvironment {
    fn from(c: ComputeEnvironmentDetail) -> Self {
        let instance_types = c
            .compute_resources()
            .map(|r| r.instance_types().to_vec())
            .unwrap_or_default();

        let max_vcpus = c.compute_resources().and_then(|r| r.maxv_cpus());
        let desired_vcpus = c.compute_resources().and_then(|r| r.desiredv_cpus());

        Self {
            name: c.compute_environment_name().unwrap_or_default().to_string(),
            arn: c.compute_environment_arn().map(|s| s.to_string()),
            state: c.state().map(|s| s.as_str().to_string()),
            status: c.status().map(|s| s.as_str().to_string()),
            compute_type: c.r#type().map(|t| t.as_str().to_string()),
            instance_types,
            max_vcpus,
            desired_vcpus,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BatchJobDefinition {
    pub name: String,
    pub arn: Option<String>,
    pub revision: i32,
    pub status: Option<String>,
    pub job_type: Option<String>,
}

impl From<JobDefinition> for BatchJobDefinition {
    fn from(d: JobDefinition) -> Self {
        Self {
            name: d.job_definition_name().unwrap_or_default().to_string(),
            arn: d.job_definition_arn().map(|s| s.to_string()),
            revision: d.revision().unwrap_or(0),
            status: d.status().map(|s| s.to_string()),
            job_type: d.r#type().map(|t| t.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_job_queue_full() {
        let q = BatchJobQueue {
            name: "my-queue".to_string(),
            arn: Some("arn:aws:batch:us-east-1:123456789012:job-queue/my-queue".to_string()),
            state: Some("ENABLED".to_string()),
            status: Some("VALID".to_string()),
            priority: 10,
        };

        assert_eq!(q.name, "my-queue");
        assert_eq!(q.arn, Some("arn:aws:batch:us-east-1:123456789012:job-queue/my-queue".to_string()));
        assert_eq!(q.state, Some("ENABLED".to_string()));
        assert_eq!(q.status, Some("VALID".to_string()));
        assert_eq!(q.priority, 10);
    }

    #[test]
    fn test_batch_job_queue_minimal() {
        let q = BatchJobQueue {
            name: "minimal-queue".to_string(),
            arn: None,
            state: None,
            status: None,
            priority: 1,
        };

        assert_eq!(q.name, "minimal-queue");
        assert!(q.arn.is_none());
        assert!(q.state.is_none());
        assert!(q.status.is_none());
        assert_eq!(q.priority, 1);
    }

    #[test]
    fn test_batch_compute_environment_full() {
        let ce = BatchComputeEnvironment {
            name: "my-ce".to_string(),
            arn: Some("arn:aws:batch:us-east-1:123456789012:compute-environment/my-ce".to_string()),
            state: Some("ENABLED".to_string()),
            status: Some("VALID".to_string()),
            compute_type: Some("MANAGED".to_string()),
            instance_types: vec!["m5.large".to_string(), "m5.xlarge".to_string()],
            max_vcpus: Some(256),
            desired_vcpus: Some(0),
        };

        assert_eq!(ce.name, "my-ce");
        assert_eq!(ce.state, Some("ENABLED".to_string()));
        assert_eq!(ce.compute_type, Some("MANAGED".to_string()));
        assert_eq!(ce.instance_types, vec!["m5.large".to_string(), "m5.xlarge".to_string()]);
        assert_eq!(ce.max_vcpus, Some(256));
        assert_eq!(ce.desired_vcpus, Some(0));
    }

    #[test]
    fn test_batch_compute_environment_minimal() {
        let ce = BatchComputeEnvironment {
            name: "minimal-ce".to_string(),
            arn: None,
            state: None,
            status: None,
            compute_type: None,
            instance_types: vec![],
            max_vcpus: None,
            desired_vcpus: None,
        };

        assert_eq!(ce.name, "minimal-ce");
        assert!(ce.arn.is_none());
        assert!(ce.state.is_none());
        assert!(ce.compute_type.is_none());
        assert!(ce.instance_types.is_empty());
        assert!(ce.max_vcpus.is_none());
        assert!(ce.desired_vcpus.is_none());
    }

    #[test]
    fn test_batch_job_definition_full() {
        let jd = BatchJobDefinition {
            name: "my-job-def".to_string(),
            arn: Some("arn:aws:batch:us-east-1:123456789012:job-definition/my-job-def:1".to_string()),
            revision: 1,
            status: Some("ACTIVE".to_string()),
            job_type: Some("container".to_string()),
        };

        assert_eq!(jd.name, "my-job-def");
        assert_eq!(jd.revision, 1);
        assert_eq!(jd.status, Some("ACTIVE".to_string()));
        assert_eq!(jd.job_type, Some("container".to_string()));
    }

    #[test]
    fn test_batch_job_definition_minimal() {
        let jd = BatchJobDefinition {
            name: "minimal-job-def".to_string(),
            arn: None,
            revision: 1,
            status: None,
            job_type: None,
        };

        assert_eq!(jd.name, "minimal-job-def");
        assert!(jd.arn.is_none());
        assert!(jd.status.is_none());
        assert!(jd.job_type.is_none());
    }
}
