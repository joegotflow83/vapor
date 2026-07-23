use async_graphql::{Context, Object, Result};
use aws_sdk_ec2::types::InstanceStateName;

use crate::aws::ec2::Ec2Client;
use super::types::{Instance, InstanceState, InstanceStateChange, RunInstancesInput};

fn state_name_to_instance_state(name: &InstanceStateName) -> InstanceState {
    match name {
        InstanceStateName::Pending => InstanceState::Pending,
        InstanceStateName::Running => InstanceState::Running,
        InstanceStateName::ShuttingDown => InstanceState::ShuttingDown,
        InstanceStateName::Terminated => InstanceState::Terminated,
        InstanceStateName::Stopping => InstanceState::Stopping,
        InstanceStateName::Stopped => InstanceState::Stopped,
        _ => InstanceState::Unknown,
    }
}

#[derive(Default)]
pub struct Ec2Mutation;

#[Object]
impl Ec2Mutation {
    async fn start_instances(&self, ctx: &Context<'_>, ids: Vec<String>) -> Result<Vec<InstanceStateChange>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let changes = ec2
            .start_instances(ids)
            .await?;
        Ok(changes
            .into_iter()
            .map(|(id, prev, curr)| InstanceStateChange {
                instance_id: id,
                previous_state: state_name_to_instance_state(&prev),
                current_state: state_name_to_instance_state(&curr),
            })
            .collect())
    }

    async fn stop_instances(
        &self,
        ctx: &Context<'_>,
        ids: Vec<String>,
        force: Option<bool>,
    ) -> Result<Vec<InstanceStateChange>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let changes = ec2
            .stop_instances(ids, force.unwrap_or(false))
            .await?;
        Ok(changes
            .into_iter()
            .map(|(id, prev, curr)| InstanceStateChange {
                instance_id: id,
                previous_state: state_name_to_instance_state(&prev),
                current_state: state_name_to_instance_state(&curr),
            })
            .collect())
    }

    async fn terminate_instances(&self, ctx: &Context<'_>, ids: Vec<String>) -> Result<Vec<InstanceStateChange>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let changes = ec2
            .terminate_instances(ids)
            .await?;
        Ok(changes
            .into_iter()
            .map(|(id, prev, curr)| InstanceStateChange {
                instance_id: id,
                previous_state: state_name_to_instance_state(&prev),
                current_state: state_name_to_instance_state(&curr),
            })
            .collect())
    }

    async fn reboot_instances(&self, ctx: &Context<'_>, ids: Vec<String>) -> Result<bool> {
        let ec2 = ctx.data::<Ec2Client>()?;
        ec2.reboot_instances(ids).await?;
        Ok(true)
    }

    async fn run_instances(&self, ctx: &Context<'_>, input: RunInstancesInput) -> Result<Vec<Instance>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let tags = input.tags.map(|ts| ts.into_iter().map(|t| (t.key, t.value)).collect());
        let instances = ec2
            .run_instances(
                input.image_id,
                input.instance_type,
                input.min_count,
                input.max_count,
                input.key_name,
                input.security_group_ids,
                input.subnet_id,
                tags,
            )
            .await?;
        Ok(instances.into_iter().map(Instance::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_name_to_instance_state_maps_known_variants() {
        assert_eq!(state_name_to_instance_state(&InstanceStateName::Pending), InstanceState::Pending);
        assert_eq!(state_name_to_instance_state(&InstanceStateName::Running), InstanceState::Running);
        assert_eq!(state_name_to_instance_state(&InstanceStateName::ShuttingDown), InstanceState::ShuttingDown);
        assert_eq!(state_name_to_instance_state(&InstanceStateName::Terminated), InstanceState::Terminated);
        assert_eq!(state_name_to_instance_state(&InstanceStateName::Stopping), InstanceState::Stopping);
        assert_eq!(state_name_to_instance_state(&InstanceStateName::Stopped), InstanceState::Stopped);
    }

    #[test]
    fn state_name_to_instance_state_unrecognized_value_is_unknown_not_running() {
        // A tool that can terminate instances must never misreport an
        // unrecognized state as Running.
        let unrecognized = InstanceStateName::from("some-future-state");
        assert_eq!(state_name_to_instance_state(&unrecognized), InstanceState::Unknown);
    }

    use crate::aws::ec2::Ec2Client;
    use crate::aws::test_util::{ec2_error_response, request, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_mutation_schema;

    const ENDPOINT: &str = "https://ec2.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn start_instances_maps_state_change() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=StartInstances&Version=2016-11-15&InstanceId.1=i-1"),
            xml_response(
                200,
                "<StartInstancesResponse><instancesSet><item><instanceId>i-1</instanceId>\
                 <previousState><name>stopped</name></previousState>\
                 <currentState><name>pending</name></currentState></item></instancesSet>\
                 </StartInstancesResponse>",
            ),
        )]);
        let schema = build_mutation_schema(Ec2Mutation)
            .data(Ec2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"mutation { startInstances(ids: ["i-1"]) { instanceId previousState currentState } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let change = &json["startInstances"][0];
        assert_eq!(change["instanceId"], "i-1");
        assert_eq!(change["previousState"], "STOPPED");
        assert_eq!(change["currentState"], "PENDING");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn stop_instances_defaults_force_false_and_maps_state_change() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=StopInstances&Version=2016-11-15&InstanceId.1=i-1&Force=false"),
            xml_response(
                200,
                "<StopInstancesResponse><instancesSet><item><instanceId>i-1</instanceId>\
                 <previousState><name>running</name></previousState>\
                 <currentState><name>stopping</name></currentState></item></instancesSet>\
                 </StopInstancesResponse>",
            ),
        )]);
        let schema = build_mutation_schema(Ec2Mutation)
            .data(Ec2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"mutation { stopInstances(ids: ["i-1"]) { instanceId previousState currentState } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let change = &json["stopInstances"][0];
        assert_eq!(change["previousState"], "RUNNING");
        assert_eq!(change["currentState"], "STOPPING");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn terminate_instances_maps_state_change() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=TerminateInstances&Version=2016-11-15&InstanceId.1=i-1"),
            xml_response(
                200,
                "<TerminateInstancesResponse><instancesSet><item><instanceId>i-1</instanceId>\
                 <previousState><name>running</name></previousState>\
                 <currentState><name>shutting-down</name></currentState></item></instancesSet>\
                 </TerminateInstancesResponse>",
            ),
        )]);
        let schema = build_mutation_schema(Ec2Mutation)
            .data(Ec2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"mutation { terminateInstances(ids: ["i-1"]) { instanceId previousState currentState } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let change = &json["terminateInstances"][0];
        assert_eq!(change["previousState"], "RUNNING");
        assert_eq!(change["currentState"], "SHUTTING_DOWN");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn reboot_instances_returns_true_on_success() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=RebootInstances&Version=2016-11-15&InstanceId.1=i-1"),
            xml_response(200, "<RebootInstancesResponse><return>true</return></RebootInstancesResponse>"),
        )]);
        let schema = build_mutation_schema(Ec2Mutation)
            .data(Ec2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute(r#"mutation { rebootInstances(ids: ["i-1"]) }"#).await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["rebootInstances"], true);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn reboot_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=RebootInstances&Version=2016-11-15&InstanceId.1=i-1"),
            ec2_error_response("InvalidInstanceID.NotFound", "instance not found"),
        )]);
        let schema = build_mutation_schema(Ec2Mutation)
            .data(Ec2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute(r#"mutation { rebootInstances(ids: ["i-1"]) }"#).await;

        assert!(!res.errors.is_empty(), "expected an error, got success");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn run_instances_maps_input_and_returns_instances() {
        // No `relaxed_requests_match()` here (unlike the other tests in this
        // module): `run_instances` auto-generates a random `ClientToken`
        // whenever none is pinned via `idempotency_token_provider` (see
        // `aws::ec2::tests::ec2_client_with_fixed_token`), and `Ec2Client`'s
        // `inner` field isn't visible outside its own module, so this
        // resolver-layer test can't pin it. `StaticReplayClient` hands back
        // the queued response regardless of the actual request content, so
        // the response-mapping assertions below are unaffected either way.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=RunInstances&Version=2016-11-15&\
                 ImageId=ami-1&InstanceType=t3.micro&MaxCount=1&MinCount=1",
            ),
            xml_response(
                200,
                "<RunInstancesResponse><instancesSet><item><instanceId>i-1</instanceId></item></instancesSet></RunInstancesResponse>",
            ),
        )]);
        let schema = build_mutation_schema(Ec2Mutation)
            .data(Ec2Client::new(&sdk_config(http_client)))
            .finish();

        let res = schema
            .execute(
                r#"mutation { runInstances(input: { imageId: "ami-1", instanceType: "t3.micro", minCount: 1, maxCount: 1 }) { id } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["runInstances"][0]["id"], "i-1");
    }
}
