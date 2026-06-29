use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct DataSyncAgentInfo {
    pub agent_arn: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub creation_time: Option<String>,
}

pub struct DataSyncLocationInfo {
    pub location_arn: String,
    pub location_uri: Option<String>,
    pub creation_time: Option<String>,
}

pub struct DataSyncTaskInfo {
    pub task_arn: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub source_location_arn: Option<String>,
    pub destination_location_arn: Option<String>,
    pub creation_time: Option<String>,
}

pub struct DataSyncTaskExecutionInfo {
    pub task_execution_arn: String,
    pub status: Option<String>,
    pub start_time: Option<String>,
    pub estimated_files_to_transfer: Option<i64>,
    pub files_transferred: Option<i64>,
    pub bytes_transferred: Option<i64>,
}

pub struct DataSyncClient {
    inner: aws_sdk_datasync::Client,
}

impl DataSyncClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_datasync::Client::new(config),
        }
    }

    pub async fn list_agents(&self) -> Result<Vec<DataSyncAgentInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_agents();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for agent in output.agents() {
                items.push(DataSyncAgentInfo {
                    agent_arn: agent.agent_arn().unwrap_or_default().to_string(),
                    name: agent.name().map(|s| s.to_string()),
                    status: agent.status().map(|s| s.as_str().to_string()),
                    creation_time: None,
                });
            }
            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_locations(&self) -> Result<Vec<DataSyncLocationInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_locations();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for loc in output.locations() {
                items.push(DataSyncLocationInfo {
                    location_arn: loc.location_arn().unwrap_or_default().to_string(),
                    location_uri: loc.location_uri().map(|s| s.to_string()),
                    creation_time: None,
                });
            }
            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_tasks(&self) -> Result<Vec<DataSyncTaskInfo>, VaporError> {
        let mut summaries = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_tasks();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for task in output.tasks() {
                summaries.push((
                    task.task_arn().unwrap_or_default().to_string(),
                    task.name().map(|s| s.to_string()),
                    task.status().map(|s| s.as_str().to_string()),
                ));
            }
            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        let mut items = Vec::new();
        for (task_arn, name, status) in summaries {
            let (source_location_arn, destination_location_arn, creation_time) = self
                .inner
                .describe_task()
                .task_arn(&task_arn)
                .send()
                .await
                .ok()
                .map(|d| {
                    (
                        d.source_location_arn().map(|s| s.to_string()),
                        d.destination_location_arn().map(|s| s.to_string()),
                        d.creation_time().map(|t| t.to_string()),
                    )
                })
                .unwrap_or((None, None, None));

            items.push(DataSyncTaskInfo {
                task_arn,
                name,
                status,
                source_location_arn,
                destination_location_arn,
                creation_time,
            });
        }

        Ok(items)
    }

    pub async fn list_task_executions(
        &self,
        task_arn: String,
    ) -> Result<Vec<DataSyncTaskExecutionInfo>, VaporError> {
        let mut exec_summaries = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_task_executions().task_arn(&task_arn);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for exec in output.task_executions() {
                exec_summaries.push((
                    exec.task_execution_arn().unwrap_or_default().to_string(),
                    exec.status().map(|s| s.as_str().to_string()),
                ));
            }
            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        let mut items = Vec::new();
        for (exec_arn, status) in exec_summaries {
            let (start_time, estimated_files_to_transfer, files_transferred, bytes_transferred) =
                self.inner
                    .describe_task_execution()
                    .task_execution_arn(&exec_arn)
                    .send()
                    .await
                    .ok()
                    .map(|d| {
                        (
                            d.start_time().map(|t| t.to_string()),
                            Some(d.estimated_files_to_transfer()),
                            Some(d.files_transferred()),
                            Some(d.bytes_transferred()),
                        )
                    })
                    .unwrap_or((None, None, None, None));

            items.push(DataSyncTaskExecutionInfo {
                task_execution_arn: exec_arn,
                status,
                start_time,
                estimated_files_to_transfer,
                files_transferred,
                bytes_transferred,
            });
        }

        Ok(items)
    }
}
