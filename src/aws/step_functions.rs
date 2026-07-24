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

    /// Lists state machines, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `max_results` is handed to
    /// AWS via `ListStateMachinesInput::max_results` so a capped page
    /// boundary lands exactly on the returned token (kinesis/eventbridge
    /// pattern); `.into_paginator()` dropped since it hides the token.
    pub async fn list_state_machines(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_sfn::types::StateMachineListItem>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_state_machines();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token().map(|t| t.to_string());
            items.extend(output.state_machines);

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn describe_state_machine(
        &self,
        arn: &str,
    ) -> Result<
        aws_sdk_sfn::operation::describe_state_machine::DescribeStateMachineOutput,
        VaporError,
    > {
        self.inner
            .describe_state_machine()
            .state_machine_arn(arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)
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
            .map_err(crate::error::sdk_err)?;
        Ok(output.tags().to_vec())
    }

    pub async fn describe_execution(
        &self,
        execution_arn: &str,
    ) -> Result<aws_sdk_sfn::operation::describe_execution::DescribeExecutionOutput, VaporError>
    {
        self.inner
            .describe_execution()
            .execution_arn(execution_arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Lists executions for a state machine, optionally filtered by status,
    /// capped at `limit` results (default unlimited), and resumed from
    /// `next_token`. `max_results` is handed to AWS directly (same pattern
    /// as `list_state_machines`); the request is rebuilt each iteration so
    /// `status_filter` is reapplied per page (fsx/transcribe precedent).
    pub async fn list_executions(
        &self,
        state_machine_arn: &str,
        status_filter: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_sfn::types::ExecutionListItem>, Option<String>), VaporError> {
        let status = status_filter.map(parse_execution_status).transpose()?;

        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_executions()
                .state_machine_arn(state_machine_arn);
            if let Some(s) = status.clone() {
                req = req.status_filter(s);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token().map(|t| t.to_string());
            items.extend(output.executions);

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

/// Parses a GraphQL `statusFilter` string into an `ExecutionStatus`, rejecting
/// unrecognized values instead of silently defaulting to a live state (the
/// EC2 `InstanceState` bug's category — a typo or a future SDK-added variant
/// must produce a clear error, not a wrong-but-plausible result set).
fn parse_execution_status(status: &str) -> Result<ExecutionStatus, VaporError> {
    match status {
        "RUNNING" => Ok(ExecutionStatus::Running),
        "SUCCEEDED" => Ok(ExecutionStatus::Succeeded),
        "FAILED" => Ok(ExecutionStatus::Failed),
        "TIMED_OUT" => Ok(ExecutionStatus::TimedOut),
        "ABORTED" => Ok(ExecutionStatus::Aborted),
        other => Err(VaporError::InvalidInput(format!(
            "statusFilter must be one of RUNNING, SUCCEEDED, FAILED, TIMED_OUT, ABORTED; got {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_execution_status_accepts_known_values() {
        assert!(matches!(
            parse_execution_status("RUNNING"),
            Ok(ExecutionStatus::Running)
        ));
        assert!(matches!(
            parse_execution_status("SUCCEEDED"),
            Ok(ExecutionStatus::Succeeded)
        ));
        assert!(matches!(
            parse_execution_status("FAILED"),
            Ok(ExecutionStatus::Failed)
        ));
        assert!(matches!(
            parse_execution_status("TIMED_OUT"),
            Ok(ExecutionStatus::TimedOut)
        ));
        assert!(matches!(
            parse_execution_status("ABORTED"),
            Ok(ExecutionStatus::Aborted)
        ));
    }

    #[test]
    fn parse_execution_status_rejects_unrecognized_value() {
        let err = parse_execution_status("running").unwrap_err();
        assert!(matches!(err, VaporError::InvalidInput(_)));

        let err = parse_execution_status("PENDING_REDRIVE").unwrap_err();
        assert!(matches!(err, VaporError::InvalidInput(_)));
    }
}
