use async_graphql::SimpleObject;
use aws_sdk_emr::types::{Cluster, StepSummary};

#[derive(SimpleObject, Clone)]
pub struct EmrCluster {
    pub id: String,
    pub name: Option<String>,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub state_change_reason: Option<String>,
    pub release_label: Option<String>,
    pub applications: Vec<String>,
    pub instance_count: Option<i32>,
    pub master_public_dns: Option<String>,
    pub creation_time: Option<String>,
    pub termination_protected: bool,
    pub auto_terminate: bool,
}

impl From<Cluster> for EmrCluster {
    fn from(c: Cluster) -> Self {
        let status = c
            .status()
            .and_then(|s| s.state())
            .map(|s| s.as_str().to_string());

        let state_change_reason = c
            .status()
            .and_then(|s| s.state_change_reason())
            .and_then(|r| r.message())
            .map(|m| m.to_string());

        let creation_time = c
            .status()
            .and_then(|s| s.timeline())
            .and_then(|t| t.creation_date_time())
            .map(|dt| dt.to_string());

        let applications = c
            .applications()
            .iter()
            .filter_map(|a| a.name().map(|n| n.to_string()))
            .collect();

        Self {
            id: c.id().unwrap_or_default().to_string(),
            name: c.name().map(|s| s.to_string()),
            arn: c.cluster_arn().map(|s| s.to_string()),
            status,
            state_change_reason,
            release_label: c.release_label().map(|s| s.to_string()),
            applications,
            // `Cluster` summaries don't carry an instance count; left unset.
            instance_count: None,
            master_public_dns: c.master_public_dns_name().map(|s| s.to_string()),
            creation_time,
            termination_protected: c.termination_protected().unwrap_or(false),
            auto_terminate: c.auto_terminate().unwrap_or(false),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct EmrStep {
    pub id: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub action_on_failure: Option<String>,
    pub creation_time: Option<String>,
    pub end_time: Option<String>,
}

impl From<StepSummary> for EmrStep {
    fn from(s: StepSummary) -> Self {
        let status = s
            .status()
            .and_then(|st| st.state())
            .map(|state| state.as_str().to_string());

        let creation_time = s
            .status()
            .and_then(|st| st.timeline())
            .and_then(|t| t.creation_date_time())
            .map(|dt| dt.to_string());

        let end_time = s
            .status()
            .and_then(|st| st.timeline())
            .and_then(|t| t.end_date_time())
            .map(|dt| dt.to_string());

        Self {
            id: s.id().unwrap_or_default().to_string(),
            name: s.name().map(|n| n.to_string()),
            status,
            action_on_failure: s.action_on_failure().map(|a| a.as_str().to_string()),
            creation_time,
            end_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emr_cluster_full() {
        let cluster = EmrCluster {
            id: "j-ABC123".to_string(),
            name: Some("my-cluster".to_string()),
            arn: Some("arn:aws:elasticmapreduce:us-east-1:123456789012:cluster/j-ABC123".to_string()),
            status: Some("RUNNING".to_string()),
            state_change_reason: Some("User requested".to_string()),
            release_label: Some("emr-6.10.0".to_string()),
            applications: vec!["Spark".to_string(), "Hive".to_string()],
            instance_count: Some(5),
            master_public_dns: Some("ec2-1-2-3-4.compute-1.amazonaws.com".to_string()),
            creation_time: Some("2024-01-01T00:00:00Z".to_string()),
            termination_protected: true,
            auto_terminate: false,
        };

        assert_eq!(cluster.id, "j-ABC123");
        assert_eq!(cluster.name, Some("my-cluster".to_string()));
        assert_eq!(cluster.status, Some("RUNNING".to_string()));
        assert_eq!(cluster.release_label, Some("emr-6.10.0".to_string()));
        assert_eq!(cluster.applications, vec!["Spark".to_string(), "Hive".to_string()]);
        assert_eq!(cluster.instance_count, Some(5));
        assert!(cluster.termination_protected);
        assert!(!cluster.auto_terminate);
    }

    #[test]
    fn test_emr_cluster_minimal() {
        let cluster = EmrCluster {
            id: "j-MINIMAL".to_string(),
            name: None,
            arn: None,
            status: None,
            state_change_reason: None,
            release_label: None,
            applications: vec![],
            instance_count: None,
            master_public_dns: None,
            creation_time: None,
            termination_protected: false,
            auto_terminate: false,
        };

        assert_eq!(cluster.id, "j-MINIMAL");
        assert!(cluster.name.is_none());
        assert!(cluster.arn.is_none());
        assert!(cluster.status.is_none());
        assert!(cluster.applications.is_empty());
        assert!(cluster.instance_count.is_none());
        assert!(!cluster.termination_protected);
        assert!(!cluster.auto_terminate);
    }

    #[test]
    fn test_emr_step_full() {
        let step = EmrStep {
            id: "s-STEP123".to_string(),
            name: Some("my-step".to_string()),
            status: Some("COMPLETED".to_string()),
            action_on_failure: Some("CONTINUE".to_string()),
            creation_time: Some("2024-01-01T00:00:00Z".to_string()),
            end_time: Some("2024-01-01T01:00:00Z".to_string()),
        };

        assert_eq!(step.id, "s-STEP123");
        assert_eq!(step.name, Some("my-step".to_string()));
        assert_eq!(step.status, Some("COMPLETED".to_string()));
        assert_eq!(step.action_on_failure, Some("CONTINUE".to_string()));
        assert!(step.creation_time.is_some());
        assert!(step.end_time.is_some());
    }

    #[test]
    fn test_emr_step_minimal() {
        let step = EmrStep {
            id: "s-MINIMAL".to_string(),
            name: None,
            status: None,
            action_on_failure: None,
            creation_time: None,
            end_time: None,
        };

        assert_eq!(step.id, "s-MINIMAL");
        assert!(step.name.is_none());
        assert!(step.status.is_none());
        assert!(step.action_on_failure.is_none());
        assert!(step.creation_time.is_none());
        assert!(step.end_time.is_none());
    }
}
