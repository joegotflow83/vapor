use async_graphql::SimpleObject;
use aws_sdk_sfn::operation::describe_execution::DescribeExecutionOutput as SdkExecutionDetail;
use aws_sdk_sfn::types::ExecutionListItem as SdkExecution;

#[derive(SimpleObject, Clone)]
#[graphql(name = "StepFunctionsTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct StateMachine {
    pub arn: String,
    pub name: String,
    pub machine_type: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl StateMachine {
    pub fn from_describe(
        output: &aws_sdk_sfn::operation::describe_state_machine::DescribeStateMachineOutput,
        sdk_tags: &[aws_sdk_sfn::types::Tag],
    ) -> Self {
        Self {
            arn: output.state_machine_arn().to_string(),
            name: output.name().to_string(),
            machine_type: Some(output.r#type().as_str().to_string()),
            status: output.status().map(|s| s.as_str().to_string()),
            created_at: Some(output.creation_date().to_string()),
            tags: sdk_tags
                .iter()
                .filter_map(|t| {
                    Some(Tag {
                        key: t.key()?.to_string(),
                        value: t.value()?.to_string(),
                    })
                })
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Execution {
    pub execution_arn: String,
    pub state_machine_arn: String,
    pub name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
}

impl From<SdkExecution> for Execution {
    fn from(e: SdkExecution) -> Self {
        Self {
            execution_arn: e.execution_arn().to_string(),
            state_machine_arn: e.state_machine_arn().to_string(),
            name: e.name().to_string(),
            status: e.status().as_str().to_string(),
            started_at: Some(e.start_date().to_string()),
            stopped_at: e.stop_date().map(|d| d.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ExecutionDetail {
    pub execution_arn: String,
    pub state_machine_arn: String,
    pub name: Option<String>,
    pub status: String,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub cause: Option<String>,
}

impl From<SdkExecutionDetail> for ExecutionDetail {
    fn from(d: SdkExecutionDetail) -> Self {
        Self {
            execution_arn: d.execution_arn().to_string(),
            state_machine_arn: d.state_machine_arn().to_string(),
            name: d.name().map(|s| s.to_string()),
            status: d.status().as_str().to_string(),
            started_at: Some(d.start_date().to_string()),
            stopped_at: d.stop_date().map(|t| t.to_string()),
            input: d.input().map(|s| s.to_string()),
            output: d.output().map(|s| s.to_string()),
            error: d.error().map(|s| s.to_string()),
            cause: d.cause().map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_from_sdk() {
        use aws_sdk_sfn::types::ExecutionStatus;

        let exec = SdkExecution::builder()
            .execution_arn("arn:aws:states:us-east-1:123456789012:execution:MyMachine:exec1")
            .state_machine_arn("arn:aws:states:us-east-1:123456789012:stateMachine:MyMachine")
            .name("exec1")
            .status(ExecutionStatus::Succeeded)
            .start_date(aws_sdk_sfn::primitives::DateTime::from_secs(1700000000))
            .stop_date(aws_sdk_sfn::primitives::DateTime::from_secs(1700000060))
            .build()
            .unwrap();
        let result = Execution::from(exec);
        assert_eq!(result.name, "exec1");
        assert_eq!(result.status, "SUCCEEDED");
        assert!(result.started_at.is_some());
        assert!(result.stopped_at.is_some());
    }

    #[test]
    fn test_execution_running_no_stop_date() {
        use aws_sdk_sfn::types::ExecutionStatus;

        let exec = SdkExecution::builder()
            .execution_arn("arn:aws:states:us-east-1:123456789012:execution:MyMachine:exec2")
            .state_machine_arn("arn:aws:states:us-east-1:123456789012:stateMachine:MyMachine")
            .name("exec2")
            .status(ExecutionStatus::Running)
            .start_date(aws_sdk_sfn::primitives::DateTime::from_secs(1700000000))
            .build()
            .unwrap();
        let result = Execution::from(exec);
        assert_eq!(result.status, "RUNNING");
        assert!(result.stopped_at.is_none());
    }

    #[test]
    fn test_tag_struct() {
        let tag = Tag {
            key: "env".to_string(),
            value: "prod".to_string(),
        };
        assert_eq!(tag.key, "env");
        assert_eq!(tag.value, "prod");
    }

    #[test]
    fn test_state_machine_from_describe() {
        use aws_sdk_sfn::types::{StateMachineStatus, StateMachineType, Tag as SdkTag};

        let output = aws_sdk_sfn::operation::describe_state_machine::DescribeStateMachineOutput::builder()
            .state_machine_arn("arn:aws:states:us-east-1:123456789012:stateMachine:MyMachine")
            .name("MyMachine")
            .definition("{}")
            .role_arn("arn:aws:iam::123456789012:role/MyRole")
            .r#type(StateMachineType::Standard)
            .creation_date(aws_sdk_sfn::primitives::DateTime::from_secs(1700000000))
            .status(StateMachineStatus::Active)
            .build()
            .unwrap();

        let sdk_tag = SdkTag::builder()
            .key("env")
            .value("prod")
            .build();

        let result = StateMachine::from_describe(&output, &[sdk_tag]);

        assert_eq!(result.arn, "arn:aws:states:us-east-1:123456789012:stateMachine:MyMachine");
        assert_eq!(result.name, "MyMachine");
        assert_eq!(result.machine_type, Some("STANDARD".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert!(result.created_at.is_some());
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_execution_detail_from_sdk() {
        use aws_sdk_sfn::types::ExecutionStatus;

        let output = aws_sdk_sfn::operation::describe_execution::DescribeExecutionOutput::builder()
            .execution_arn("arn:aws:states:us-east-1:123456789012:execution:MyMachine:exec1")
            .state_machine_arn("arn:aws:states:us-east-1:123456789012:stateMachine:MyMachine")
            .name("exec1")
            .status(ExecutionStatus::Succeeded)
            .start_date(aws_sdk_sfn::primitives::DateTime::from_secs(1700000000))
            .stop_date(aws_sdk_sfn::primitives::DateTime::from_secs(1700000060))
            .input(r#"{"key":"value"}"#)
            .output(r#"{"result":"ok"}"#)
            .build()
            .unwrap();
        let detail = ExecutionDetail::from(output);
        assert_eq!(detail.execution_arn, "arn:aws:states:us-east-1:123456789012:execution:MyMachine:exec1");
        assert_eq!(detail.status, "SUCCEEDED");
        assert_eq!(detail.name, Some("exec1".to_string()));
        assert_eq!(detail.input, Some(r#"{"key":"value"}"#.to_string()));
        assert_eq!(detail.output, Some(r#"{"result":"ok"}"#.to_string()));
        assert!(detail.error.is_none());
        assert!(detail.cause.is_none());
        assert!(detail.stopped_at.is_some());
    }

    #[test]
    fn test_execution_detail_failed() {
        use aws_sdk_sfn::types::ExecutionStatus;

        let output = aws_sdk_sfn::operation::describe_execution::DescribeExecutionOutput::builder()
            .execution_arn("arn:aws:states:us-east-1:123456789012:execution:MyMachine:exec2")
            .state_machine_arn("arn:aws:states:us-east-1:123456789012:stateMachine:MyMachine")
            .status(ExecutionStatus::Failed)
            .start_date(aws_sdk_sfn::primitives::DateTime::from_secs(1700000000))
            .error("States.TaskFailed")
            .cause("Task threw an exception")
            .build()
            .unwrap();
        let detail = ExecutionDetail::from(output);
        assert_eq!(detail.status, "FAILED");
        assert_eq!(detail.error, Some("States.TaskFailed".to_string()));
        assert_eq!(detail.cause, Some("Task threw an exception".to_string()));
        assert!(detail.output.is_none());
        assert!(detail.stopped_at.is_none());
    }

    #[test]
    fn test_state_machine_struct() {
        let sm = StateMachine {
            arn: "arn:aws:states:us-east-1:123456789012:stateMachine:Test".to_string(),
            name: "Test".to_string(),
            machine_type: Some("STANDARD".to_string()),
            status: Some("ACTIVE".to_string()),
            created_at: Some("2023-11-14T22:13:20Z".to_string()),
            tags: vec![Tag {
                key: "env".to_string(),
                value: "prod".to_string(),
            }],
        };
        assert_eq!(sm.name, "Test");
        assert_eq!(sm.status, Some("ACTIVE".to_string()));
        assert_eq!(sm.tags.len(), 1);
        assert_eq!(sm.tags[0].key, "env");
    }
}
