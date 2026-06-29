use async_graphql::SimpleObject;

use crate::aws::datasync::{
    DataSyncAgentInfo, DataSyncLocationInfo, DataSyncTaskExecutionInfo, DataSyncTaskInfo,
};

#[derive(SimpleObject, Clone)]
pub struct DataSyncAgent {
    pub agent_arn: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub creation_time: Option<String>,
}

impl From<DataSyncAgentInfo> for DataSyncAgent {
    fn from(a: DataSyncAgentInfo) -> Self {
        Self {
            agent_arn: a.agent_arn,
            name: a.name,
            status: a.status,
            creation_time: a.creation_time,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DataSyncLocation {
    pub location_arn: String,
    pub location_uri: Option<String>,
    pub creation_time: Option<String>,
}

impl From<DataSyncLocationInfo> for DataSyncLocation {
    fn from(l: DataSyncLocationInfo) -> Self {
        Self {
            location_arn: l.location_arn,
            location_uri: l.location_uri,
            creation_time: l.creation_time,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DataSyncTask {
    pub task_arn: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub source_location_arn: Option<String>,
    pub destination_location_arn: Option<String>,
    pub creation_time: Option<String>,
}

impl From<DataSyncTaskInfo> for DataSyncTask {
    fn from(t: DataSyncTaskInfo) -> Self {
        Self {
            task_arn: t.task_arn,
            name: t.name,
            status: t.status,
            source_location_arn: t.source_location_arn,
            destination_location_arn: t.destination_location_arn,
            creation_time: t.creation_time,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DataSyncTaskExecution {
    pub task_execution_arn: String,
    pub status: Option<String>,
    pub start_time: Option<String>,
    pub estimated_files_to_transfer: Option<i64>,
    pub files_transferred: Option<i64>,
    pub bytes_transferred: Option<i64>,
}

impl From<DataSyncTaskExecutionInfo> for DataSyncTaskExecution {
    fn from(e: DataSyncTaskExecutionInfo) -> Self {
        Self {
            task_execution_arn: e.task_execution_arn,
            status: e.status,
            start_time: e.start_time,
            estimated_files_to_transfer: e.estimated_files_to_transfer,
            files_transferred: e.files_transferred,
            bytes_transferred: e.bytes_transferred,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::datasync::{
        DataSyncAgentInfo, DataSyncLocationInfo, DataSyncTaskExecutionInfo, DataSyncTaskInfo,
    };

    #[test]
    fn test_agent_from() {
        let info = DataSyncAgentInfo {
            agent_arn: "arn:aws:datasync:us-east-1:123:agent/agent-01".to_string(),
            name: Some("my-agent".to_string()),
            status: Some("ONLINE".to_string()),
            creation_time: None,
        };
        let result = DataSyncAgent::from(info);
        assert_eq!(result.agent_arn, "arn:aws:datasync:us-east-1:123:agent/agent-01");
        assert_eq!(result.name, Some("my-agent".to_string()));
        assert_eq!(result.status, Some("ONLINE".to_string()));
        assert!(result.creation_time.is_none());
    }

    #[test]
    fn test_agent_minimal() {
        let info = DataSyncAgentInfo {
            agent_arn: "arn:aws:datasync:us-east-1:123:agent/agent-02".to_string(),
            name: None,
            status: None,
            creation_time: None,
        };
        let result = DataSyncAgent::from(info);
        assert!(result.name.is_none());
        assert!(result.status.is_none());
    }

    #[test]
    fn test_location_from() {
        let info = DataSyncLocationInfo {
            location_arn: "arn:aws:datasync:us-east-1:123:location/loc-01".to_string(),
            location_uri: Some("s3://my-bucket/prefix/".to_string()),
            creation_time: None,
        };
        let result = DataSyncLocation::from(info);
        assert_eq!(result.location_arn, "arn:aws:datasync:us-east-1:123:location/loc-01");
        assert_eq!(result.location_uri, Some("s3://my-bucket/prefix/".to_string()));
    }

    #[test]
    fn test_location_minimal() {
        let info = DataSyncLocationInfo {
            location_arn: "arn:aws:datasync:us-east-1:123:location/loc-02".to_string(),
            location_uri: None,
            creation_time: None,
        };
        let result = DataSyncLocation::from(info);
        assert!(result.location_uri.is_none());
    }

    #[test]
    fn test_task_from() {
        let info = DataSyncTaskInfo {
            task_arn: "arn:aws:datasync:us-east-1:123:task/task-01".to_string(),
            name: Some("my-task".to_string()),
            status: Some("AVAILABLE".to_string()),
            source_location_arn: Some("arn:aws:datasync:us-east-1:123:location/src".to_string()),
            destination_location_arn: Some("arn:aws:datasync:us-east-1:123:location/dst".to_string()),
            creation_time: None,
        };
        let result = DataSyncTask::from(info);
        assert_eq!(result.task_arn, "arn:aws:datasync:us-east-1:123:task/task-01");
        assert_eq!(result.name, Some("my-task".to_string()));
        assert_eq!(result.status, Some("AVAILABLE".to_string()));
        assert!(result.source_location_arn.is_some());
        assert!(result.destination_location_arn.is_some());
    }

    #[test]
    fn test_task_minimal() {
        let info = DataSyncTaskInfo {
            task_arn: "arn:aws:datasync:us-east-1:123:task/task-02".to_string(),
            name: None,
            status: None,
            source_location_arn: None,
            destination_location_arn: None,
            creation_time: None,
        };
        let result = DataSyncTask::from(info);
        assert!(result.name.is_none());
        assert!(result.status.is_none());
        assert!(result.source_location_arn.is_none());
    }

    #[test]
    fn test_task_execution_from() {
        let info = DataSyncTaskExecutionInfo {
            task_execution_arn: "arn:aws:datasync:us-east-1:123:task/task-01/execution/exec-01"
                .to_string(),
            status: Some("SUCCESS".to_string()),
            start_time: Some("2024-01-01T00:00:00Z".to_string()),
            estimated_files_to_transfer: Some(1000),
            files_transferred: Some(1000),
            bytes_transferred: Some(1_000_000),
        };
        let result = DataSyncTaskExecution::from(info);
        assert_eq!(result.status, Some("SUCCESS".to_string()));
        assert_eq!(result.estimated_files_to_transfer, Some(1000));
        assert_eq!(result.files_transferred, Some(1000));
        assert_eq!(result.bytes_transferred, Some(1_000_000));
    }

    #[test]
    fn test_task_execution_minimal() {
        let info = DataSyncTaskExecutionInfo {
            task_execution_arn: "arn:aws:datasync:us-east-1:123:task/task-01/execution/exec-02"
                .to_string(),
            status: None,
            start_time: None,
            estimated_files_to_transfer: None,
            files_transferred: None,
            bytes_transferred: None,
        };
        let result = DataSyncTaskExecution::from(info);
        assert!(result.status.is_none());
        assert!(result.estimated_files_to_transfer.is_none());
        assert!(result.bytes_transferred.is_none());
    }
}
