use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::step_functions::StepFunctionsClient;
use crate::schema::step_functions::types::{Execution, ExecutionDetail, StateMachine};

#[derive(Default)]
pub struct StepFunctionsQuery;

#[Object]
impl StepFunctionsQuery {
    async fn state_machines(&self, ctx: &Context<'_>) -> Result<Vec<StateMachine>> {
        let client = ctx.data::<StepFunctionsClient>()?;
        let list = client.list_state_machines().await?;
        let futs: Vec<_> = list
            .iter()
            .map(|sm| {
                let arn = sm.state_machine_arn().to_string();
                async move {
                    let (desc, tags) = futures::join!(
                        client.describe_state_machine(&arn),
                        client.list_tags_for_resource(&arn)
                    );
                    desc.ok()
                        .map(|d| StateMachine::from_describe(&d, &tags.unwrap_or_default()))
                }
            })
            .collect();
        let results = join_all(futs).await;
        Ok(results.into_iter().flatten().collect())
    }

    async fn executions(
        &self,
        ctx: &Context<'_>,
        state_machine_arn: String,
        status_filter: Option<String>,
    ) -> Result<Vec<Execution>> {
        let client = ctx.data::<StepFunctionsClient>()?;
        let execs = client
            .list_executions(&state_machine_arn, status_filter.as_deref())
            .await?;
        Ok(execs.into_iter().map(Execution::from).collect())
    }

    async fn execution_detail(
        &self,
        ctx: &Context<'_>,
        execution_arn: String,
    ) -> Result<ExecutionDetail> {
        let client = ctx.data::<StepFunctionsClient>()?;
        let detail = client.describe_execution(&execution_arn).await?;
        Ok(ExecutionDetail::from(detail))
    }
}
