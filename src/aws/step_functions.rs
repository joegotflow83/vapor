use aws_config::SdkConfig;
use aws_sdk_sfn::types::ExecutionStatus;

use crate::error::VaporError;

pub struct StepFunctionsClient {
    inner: aws_sdk_sfn::Client,
}

impl StepFunctionsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_sfn::Client::new(config),
        }
    }

    pub async fn list_state_machines(
        &self,
    ) -> Result<Vec<aws_sdk_sfn::types::StateMachineListItem>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_state_machines();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.state_machines().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_state_machine(
        &self,
        arn: &str,
    ) -> Result<aws_sdk_sfn::operation::describe_state_machine::DescribeStateMachineOutput, VaporError>
    {
        self.inner
            .describe_state_machine()
            .state_machine_arn(arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn list_tags_for_resource(
        &self,
        arn: &str,
    ) -> Result<Vec<aws_sdk_sfn::types::Tag>, VaporError> {
        let output = self
            .inner
            .list_tags_for_resource()
            .resource_arn(arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.tags().to_vec())
    }

    pub async fn describe_execution(
        &self,
        execution_arn: &str,
    ) -> Result<aws_sdk_sfn::operation::describe_execution::DescribeExecutionOutput, VaporError> {
        self.inner
            .describe_execution()
            .execution_arn(execution_arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn list_executions(
        &self,
        state_machine_arn: &str,
        status_filter: Option<&str>,
    ) -> Result<Vec<aws_sdk_sfn::types::ExecutionListItem>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_executions()
                .state_machine_arn(state_machine_arn);

            if let Some(status) = status_filter {
                let s = match status {
                    "RUNNING" => ExecutionStatus::Running,
                    "SUCCEEDED" => ExecutionStatus::Succeeded,
                    "FAILED" => ExecutionStatus::Failed,
                    "TIMED_OUT" => ExecutionStatus::TimedOut,
                    "ABORTED" => ExecutionStatus::Aborted,
                    _ => ExecutionStatus::Running,
                };
                req = req.status_filter(s);
            }

            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.executions().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
