use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::step_functions::StepFunctionsClient;
use crate::schema::pagination::Page;
use crate::schema::step_functions::types::{Execution, ExecutionDetail, StateMachine};

#[derive(Default)]
pub struct StepFunctionsQuery;

#[Object]
impl StepFunctionsQuery {
    /// Lists state machines, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn state_machines(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<StateMachine>> {
        let client = ctx.data::<StepFunctionsClient>()?;
        let (list, token) = client.list_state_machines(limit, next_token).await?;
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
        Ok(Page {
            items: results.into_iter().flatten().collect(),
            next_token: token,
        })
    }

    /// Lists executions for a state machine, optionally filtered by status,
    /// capped at `limit` results (default unlimited), and resumed from
    /// `next_token`.
    async fn executions(
        &self,
        ctx: &Context<'_>,
        state_machine_arn: String,
        status_filter: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Execution>> {
        let client = ctx.data::<StepFunctionsClient>()?;
        let (execs, token) = client
            .list_executions(
                &state_machine_arn,
                status_filter.as_deref(),
                limit,
                next_token,
            )
            .await?;
        Ok(Page {
            items: execs.into_iter().map(Execution::from).collect(),
            next_token: token,
        })
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

#[cfg(test)]
mod tests {
    use crate::aws::step_functions::StepFunctionsClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::StepFunctionsQuery;

    const BASE: &str = "https://states.us-east-1.amazonaws.com";
    const SM_ARN: &str = "arn:aws:states:us-east-1:123456789012:stateMachine:MyMachine";
    const EXEC_ARN: &str = "arn:aws:states:us-east-1:123456789012:execution:MyMachine:exec1";

    #[tokio::test]
    async fn state_machines_fans_out_describe_and_tags() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"maxResults":1}"#),
                json_response(
                    200,
                    format!(
                        r#"{{"stateMachines":[{{"stateMachineArn":"{SM_ARN}","name":"MyMachine","type":"STANDARD","creationDate":1700000000}}],"nextToken":"page2"}}"#
                    ),
                ),
            ),
            ReplayEvent::new(
                request(BASE, format!(r#"{{"stateMachineArn":"{SM_ARN}"}}"#)),
                json_response(
                    200,
                    format!(
                        r#"{{"stateMachineArn":"{SM_ARN}","name":"MyMachine","definition":"{{}}","roleArn":"arn:aws:iam::123456789012:role/MyRole","type":"STANDARD","creationDate":1700000000,"status":"ACTIVE"}}"#
                    ),
                ),
            ),
            ReplayEvent::new(
                request(BASE, format!(r#"{{"resourceArn":"{SM_ARN}"}}"#)),
                json_response(200, r#"{"tags":[{"key":"env","value":"prod"}]}"#),
            ),
        ]);
        let schema = build_query_schema(StepFunctionsQuery)
            .data(StepFunctionsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ stateMachines(limit: 1) { items { arn name machineType status tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["stateMachines"]["items"][0];
        assert_eq!(item["arn"], SM_ARN);
        assert_eq!(item["name"], "MyMachine");
        assert_eq!(item["machineType"], "STANDARD");
        assert_eq!(item["status"], "ACTIVE");
        assert_eq!(item["tags"][0]["key"], "env");
        assert_eq!(item["tags"][0]["value"], "prod");
        assert_eq!(json["stateMachines"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn executions_forwards_status_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                format!(r#"{{"stateMachineArn":"{SM_ARN}","statusFilter":"FAILED"}}"#),
            ),
            json_response(
                200,
                format!(
                    r#"{{"executions":[{{"executionArn":"{EXEC_ARN}","stateMachineArn":"{SM_ARN}","name":"exec1","status":"FAILED","startDate":1700000000}}]}}"#
                ),
            ),
        )]);
        let schema = build_query_schema(StepFunctionsQuery)
            .data(StepFunctionsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ executions(stateMachineArn: "{SM_ARN}", statusFilter: "FAILED") {{ items {{ executionArn stateMachineArn name status }} nextToken }} }}"#
            ))
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["executions"]["items"][0];
        assert_eq!(item["executionArn"], EXEC_ARN);
        assert_eq!(item["stateMachineArn"], SM_ARN);
        assert_eq!(item["status"], "FAILED");
        assert_eq!(json["executions"]["nextToken"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn execution_detail_maps_failed_execution() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, format!(r#"{{"executionArn":"{EXEC_ARN}"}}"#)),
            json_response(
                200,
                format!(
                    r#"{{"executionArn":"{EXEC_ARN}","stateMachineArn":"{SM_ARN}","name":"exec1","status":"FAILED","startDate":1700000000,"error":"States.TaskFailed","cause":"boom"}}"#
                ),
            ),
        )]);
        let schema = build_query_schema(StepFunctionsQuery)
            .data(StepFunctionsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ executionDetail(executionArn: "{EXEC_ARN}") {{ executionArn stateMachineArn name status error cause }} }}"#
            ))
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let detail = &json["executionDetail"];
        assert_eq!(detail["executionArn"], EXEC_ARN);
        assert_eq!(detail["stateMachineArn"], SM_ARN);
        assert_eq!(detail["status"], "FAILED");
        assert_eq!(detail["error"], "States.TaskFailed");
        assert_eq!(detail["cause"], "boom");
        http_client.relaxed_requests_match();
    }
}
