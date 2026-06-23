use async_graphql::SimpleObject;
use aws_sdk_codepipeline::types::{
    ActionState as SdkActionState, PipelineExecutionSummary, PipelineSummary,
    StageState as SdkStageState,
};

#[derive(SimpleObject, Clone)]
pub struct Pipeline {
    pub name: String,
    pub arn: Option<String>,
    pub version: Option<i32>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl Pipeline {
    pub fn from_summary(s: PipelineSummary, arn: Option<String>) -> Self {
        Self {
            name: s.name().unwrap_or_default().to_string(),
            arn,
            version: s.version(),
            created: s.created().map(|t| t.to_string()),
            updated: s.updated().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct PipelineExecution {
    pub pipeline_name: String,
    pub execution_id: Option<String>,
    pub status: Option<String>,
    pub trigger: Option<String>,
    pub started_at: Option<String>,
    pub last_updated_at: Option<String>,
}

impl PipelineExecution {
    pub fn from_summary(pipeline_name: &str, s: &PipelineExecutionSummary) -> Self {
        Self {
            pipeline_name: pipeline_name.to_string(),
            execution_id: s.pipeline_execution_id().map(|v| v.to_string()),
            status: s.status().map(|st| st.as_str().to_string()),
            trigger: s
                .trigger()
                .and_then(|t| t.trigger_type().map(|tt| tt.as_str().to_string())),
            started_at: s.start_time().map(|t| t.to_string()),
            last_updated_at: s.last_update_time().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct StageState {
    pub stage_name: Option<String>,
    pub status: Option<String>,
    pub action_states: Vec<ActionState>,
}

impl From<&SdkStageState> for StageState {
    fn from(s: &SdkStageState) -> Self {
        let action_states = s
            .action_states()
            .iter()
            .map(ActionState::from)
            .collect();
        Self {
            stage_name: s.stage_name().map(|v| v.to_string()),
            status: s
                .latest_execution()
                .map(|e| e.status().as_str().to_string()),
            action_states,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ActionState {
    pub action_name: Option<String>,
    pub status: Option<String>,
    pub last_status_change: Option<String>,
    pub error_details: Option<String>,
    pub external_execution_url: Option<String>,
}

impl From<&SdkActionState> for ActionState {
    fn from(a: &SdkActionState) -> Self {
        let (status, last_status_change, error_details, external_execution_url) =
            match a.latest_execution() {
                Some(exec) => (
                    exec.status().map(|s| s.as_str().to_string()),
                    exec.last_status_change().map(|t| t.to_string()),
                    exec.error_details().and_then(|e| e.message()).map(|m| m.to_string()),
                    exec.external_execution_url().map(|u| u.to_string()),
                ),
                None => (None, None, None, None),
            };
        Self {
            action_name: a.action_name().map(|v| v.to_string()),
            status,
            last_status_change,
            error_details,
            external_execution_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_from_summary() {
        let summary = PipelineSummary::builder()
            .name("my-pipeline")
            .version(3)
            .build();
        let arn = Some("arn:aws:codepipeline:us-east-1:123456789012:my-pipeline".to_string());
        let pipeline = Pipeline::from_summary(summary, arn.clone());
        assert_eq!(pipeline.name, "my-pipeline");
        assert_eq!(pipeline.arn, arn);
        assert_eq!(pipeline.version, Some(3));
        assert!(pipeline.created.is_none());
        assert!(pipeline.updated.is_none());
    }

    #[test]
    fn test_pipeline_from_summary_minimal() {
        let summary = PipelineSummary::builder().build();
        let pipeline = Pipeline::from_summary(summary, None);
        assert_eq!(pipeline.name, "");
        assert!(pipeline.arn.is_none());
        assert!(pipeline.version.is_none());
    }

    #[test]
    fn test_pipeline_execution_from_summary() {
        let summary = PipelineExecutionSummary::builder()
            .pipeline_execution_id("exec-123")
            .status(aws_sdk_codepipeline::types::PipelineExecutionStatus::Succeeded)
            .build();
        let exec = PipelineExecution::from_summary("my-pipeline", &summary);
        assert_eq!(exec.pipeline_name, "my-pipeline");
        assert_eq!(exec.execution_id, Some("exec-123".to_string()));
        assert_eq!(exec.status, Some("Succeeded".to_string()));
        assert!(exec.trigger.is_none());
        assert!(exec.started_at.is_none());
        assert!(exec.last_updated_at.is_none());
    }

    #[test]
    fn test_stage_state_from_sdk() {
        let sdk_stage = SdkStageState::builder()
            .stage_name("Build")
            .build();
        let stage = StageState::from(&sdk_stage);
        assert_eq!(stage.stage_name, Some("Build".to_string()));
        assert!(stage.status.is_none());
        assert!(stage.action_states.is_empty());
    }

    #[test]
    fn test_action_state_from_sdk() {
        let sdk_action = SdkActionState::builder()
            .action_name("CodeBuild")
            .build();
        let action = ActionState::from(&sdk_action);
        assert_eq!(action.action_name, Some("CodeBuild".to_string()));
        assert!(action.status.is_none());
        assert!(action.last_status_change.is_none());
        assert!(action.error_details.is_none());
        assert!(action.external_execution_url.is_none());
    }
}
