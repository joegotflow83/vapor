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
        _ => InstanceState::Running,
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
