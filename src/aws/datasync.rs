use aws_config::SdkConfig;

use crate::error::VaporError;

#[derive(Debug)]
pub struct DataSyncAgentInfo {
    pub agent_arn: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub creation_time: Option<aws_smithy_types::DateTime>,
}

#[derive(Debug)]
pub struct DataSyncLocationInfo {
    pub location_arn: String,
    pub location_uri: Option<String>,
    pub creation_time: Option<String>,
}

#[derive(Debug)]
pub struct DataSyncTaskInfo {
    pub task_arn: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub source_location_arn: Option<String>,
    pub destination_location_arn: Option<String>,
    pub creation_time: Option<aws_smithy_types::DateTime>,
}

#[derive(Debug)]
pub struct DataSyncTaskExecutionInfo {
    pub task_execution_arn: String,
    pub status: Option<String>,
    pub start_time: Option<aws_smithy_types::DateTime>,
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

    /// Lists DataSync agents, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListAgents` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-datasync` 1.114.0's `operation/list_agents/_list_agents_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself
    /// (kinesis.rs pattern). The N+1 `describe_agent` fan-out only covers the
    /// single page of ARNs collected this call, not the whole collection
    /// (mq.rs `list_brokers` pattern).
    pub async fn list_agents(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DataSyncAgentInfo>, Option<String>), VaporError> {
        let mut summaries: Vec<(String, Option<String>, Option<String>)> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_agents();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for agent in output.agents.unwrap_or_default() {
                summaries.push((
                    agent.agent_arn().unwrap_or_default().to_string(),
                    agent.name().map(|s| s.to_string()),
                    agent.status().map(|s| s.as_str().to_string()),
                ));
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut items = Vec::new();
        for (agent_arn, name, status) in summaries {
            let creation_time = self
                .inner
                .describe_agent()
                .agent_arn(&agent_arn)
                .send()
                .await
                .ok()
                .and_then(|d| d.creation_time().cloned());

            items.push(DataSyncAgentInfo {
                agent_arn,
                name,
                status,
                creation_time,
            });
        }

        Ok((items, token))
    }

    /// Lists DataSync locations, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListLocations`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-datasync` 1.114.0's
    /// `operation/list_locations/_list_locations_input.rs`), kinesis.rs
    /// server-side-capping pattern; no per-item fan-out since `creation_time`
    /// is unavailable for locations (see `specs/datasync.md`'s "Known
    /// limitation" note).
    pub async fn list_locations(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DataSyncLocationInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_locations();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for loc in output.locations.unwrap_or_default() {
                items.push(DataSyncLocationInfo {
                    location_arn: loc.location_arn().unwrap_or_default().to_string(),
                    location_uri: loc.location_uri().map(|s| s.to_string()),
                    creation_time: None,
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists DataSync tasks, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListTasks` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-datasync` 1.114.0's `operation/list_tasks/_list_tasks_input.rs`),
    /// kinesis.rs pattern. The N+1 `describe_task` fan-out only covers the
    /// single page of ARNs collected this call (mq.rs pattern).
    pub async fn list_tasks(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DataSyncTaskInfo>, Option<String>), VaporError> {
        let mut summaries: Vec<(String, Option<String>, Option<String>)> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_tasks();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for task in output.tasks.unwrap_or_default() {
                summaries.push((
                    task.task_arn().unwrap_or_default().to_string(),
                    task.name().map(|s| s.to_string()),
                    task.status().map(|s| s.as_str().to_string()),
                ));
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
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
                        d.creation_time().cloned(),
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

        Ok((items, token))
    }

    /// Lists executions for a DataSync task, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListTaskExecutions` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-datasync` 1.114.0's
    /// `operation/list_task_executions/_list_task_executions_input.rs`),
    /// kinesis.rs pattern. The N+1 `describe_task_execution` fan-out only
    /// covers the single page of ARNs collected this call (mq.rs pattern).
    pub async fn list_task_executions(
        &self,
        task_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DataSyncTaskExecutionInfo>, Option<String>), VaporError> {
        let mut exec_summaries: Vec<(String, Option<String>)> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_task_executions().task_arn(&task_arn);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - exec_summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for exec in output.task_executions.unwrap_or_default() {
                exec_summaries.push((
                    exec.task_execution_arn().unwrap_or_default().to_string(),
                    exec.status().map(|s| s.as_str().to_string()),
                ));
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if exec_summaries.len() as i32 >= l => break,
                _ => continue,
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
                            d.start_time().cloned(),
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

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://datasync.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_agents_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"Agents":[{"AgentArn":"arn:aws:datasync:us-east-1:111122223333:agent/agent-1","Name":"agent-one","Status":"ONLINE"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"AgentArn":"arn:aws:datasync:us-east-1:111122223333:agent/agent-1"}"#,
                ),
                json_response(
                    200,
                    r#"{"AgentArn":"arn:aws:datasync:us-east-1:111122223333:agent/agent-1","CreationTime":1700000000}"#,
                ),
            ),
        ]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_agents(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].agent_arn,
            "arn:aws:datasync:us-east-1:111122223333:agent/agent-1"
        );
        assert_eq!(items[0].name, Some("agent-one".to_string()));
        assert_eq!(items[0].status, Some("ONLINE".to_string()));
        assert_eq!(
            items[0].creation_time,
            Some(aws_smithy_types::DateTime::from_secs(1700000000))
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_agents_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Agents":[]}"#),
        )]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_agents(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_agents_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Agents":[{"AgentArn":"arn:agent-1","Name":"agent-one","Status":"ONLINE"}],"NextToken":"next-page"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"AgentArn":"arn:agent-1"}"#),
                json_response(200, r#"{"AgentArn":"arn:agent-1","CreationTime":1700000000}"#),
            ),
        ]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_agents(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("next-page".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_agents_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"Agents":[{"AgentArn":"arn:agent-1","Name":"agent-one","Status":"ONLINE"}],"NextToken":"page-2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"page-2"}"#),
                json_response(
                    200,
                    r#"{"Agents":[{"AgentArn":"arn:agent-2","Name":"agent-two","Status":"OFFLINE"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"AgentArn":"arn:agent-1"}"#),
                json_response(200, r#"{"AgentArn":"arn:agent-1","CreationTime":1700000000}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"AgentArn":"arn:agent-2"}"#),
                json_response(200, r#"{"AgentArn":"arn:agent-2","CreationTime":1700003600}"#),
            ),
        ]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_agents(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].agent_arn, "arn:agent-1");
        assert_eq!(items[1].agent_arn, "arn:agent-2");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_agents_swallows_describe_errors_as_none_fields() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"Agents":[{"AgentArn":"arn:agent-1"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"AgentArn":"arn:agent-1"}"#),
                json_error_response("AccessDeniedException", "not authorized"),
            ),
        ]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        // `list_agents` folds the per-agent `describe_agent` fan-out through
        // `.ok()`, so a describe-level error is swallowed into a `None`
        // `creation_time` rather than propagating as a `VaporError` (unlike
        // the top-level `list_agents` call itself, which does propagate via
        // `sdk_err` — see the `list_agents_propagates_errors` test below).
        let (items, token) = client.list_agents(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].agent_arn, "arn:agent-1");
        assert_eq!(items[0].creation_time, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_agents_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let err = client.list_agents(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_locations_lists_all_locations() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Locations":[{"LocationArn":"arn:loc-1","LocationUri":"s3://bucket/prefix"}]}"#,
            ),
        )]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_locations(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].location_arn, "arn:loc-1");
        assert_eq!(items[0].location_uri, Some("s3://bucket/prefix".to_string()));
        // `creation_time` is hard-coded `None` — DataSync's `ListLocations`
        // has no such field per `specs/datasync.md`'s "Known limitation".
        assert_eq!(items[0].creation_time, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_locations_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Locations":[{"LocationArn":"arn:loc-1"}],"NextToken":"next-loc"}"#,
            ),
        )]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_locations(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("next-loc".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_locations_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidRequestException", "bad location filter"),
        )]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let err = client.list_locations(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad location filter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tasks_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"Tasks":[{"TaskArn":"arn:task-1","Status":"AVAILABLE","Name":"task-one"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskArn":"arn:task-1"}"#),
                json_response(
                    200,
                    r#"{"TaskArn":"arn:task-1","Status":"AVAILABLE","Name":"task-one","SourceLocationArn":"arn:loc-src","DestinationLocationArn":"arn:loc-dst","CreationTime":1700000000}"#,
                ),
            ),
        ]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_tasks(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].task_arn, "arn:task-1");
        assert_eq!(items[0].name, Some("task-one".to_string()));
        assert_eq!(items[0].status, Some("AVAILABLE".to_string()));
        assert_eq!(items[0].source_location_arn, Some("arn:loc-src".to_string()));
        assert_eq!(
            items[0].destination_location_arn,
            Some("arn:loc-dst".to_string())
        );
        assert_eq!(
            items[0].creation_time,
            Some(aws_smithy_types::DateTime::from_secs(1700000000))
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tasks_swallows_describe_errors_as_none_fields() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"Tasks":[{"TaskArn":"arn:task-1","Status":"AVAILABLE"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskArn":"arn:task-1"}"#),
                json_error_response("AccessDeniedException", "not authorized"),
            ),
        ]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        // Same `.ok()`-swallowing fan-out shape as `list_agents` above —
        // `describe_task`'s error becomes `None` fields, not a propagated
        // `VaporError`.
        let (items, token) = client.list_tasks(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].task_arn, "arn:task-1");
        assert_eq!(items[0].status, Some("AVAILABLE".to_string()));
        assert_eq!(items[0].source_location_arn, None);
        assert_eq!(items[0].destination_location_arn, None);
        assert_eq!(items[0].creation_time, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tasks_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidRequestException", "bad task filter"),
        )]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let err = client.list_tasks(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad task filter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_task_executions_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskArn":"arn:task-1"}"#),
                json_response(
                    200,
                    r#"{"TaskExecutions":[{"TaskExecutionArn":"arn:task-1/execution/exec-1","Status":"SUCCESS"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskExecutionArn":"arn:task-1/execution/exec-1"}"#),
                json_response(
                    200,
                    r#"{"TaskExecutionArn":"arn:task-1/execution/exec-1","Status":"SUCCESS","StartTime":1700000000,"EstimatedFilesToTransfer":100,"FilesTransferred":100,"BytesTransferred":204800}"#,
                ),
            ),
        ]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_task_executions("arn:task-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].task_execution_arn, "arn:task-1/execution/exec-1");
        assert_eq!(items[0].status, Some("SUCCESS".to_string()));
        assert_eq!(
            items[0].start_time,
            Some(aws_smithy_types::DateTime::from_secs(1700000000))
        );
        assert_eq!(items[0].estimated_files_to_transfer, Some(100));
        assert_eq!(items[0].files_transferred, Some(100));
        assert_eq!(items[0].bytes_transferred, Some(204800));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_task_executions_swallows_describe_errors_as_none_fields() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskArn":"arn:task-1"}"#),
                json_response(
                    200,
                    r#"{"TaskExecutions":[{"TaskExecutionArn":"arn:exec-1","Status":"QUEUED"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskExecutionArn":"arn:exec-1"}"#),
                json_error_response("AccessDeniedException", "not authorized"),
            ),
        ]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        // Same `.ok()`-swallowing fan-out shape as `list_agents`/`list_tasks`
        // above — `describe_task_execution`'s error becomes `None` fields
        // (including the primitive `i64` counters), not a propagated
        // `VaporError`.
        let (items, token) = client
            .list_task_executions("arn:task-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].task_execution_arn, "arn:exec-1");
        assert_eq!(items[0].status, Some("QUEUED".to_string()));
        assert_eq!(items[0].start_time, None);
        assert_eq!(items[0].estimated_files_to_transfer, None);
        assert_eq!(items[0].files_transferred, None);
        assert_eq!(items[0].bytes_transferred, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_task_executions_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"TaskArn":"arn:task-1"}"#),
            json_error_response("InvalidRequestException", "bad task arn"),
        )]);
        let client = DataSyncClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_task_executions("arn:task-1".to_string(), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad task arn");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

