use async_graphql::{Context, Object, Result};

use crate::aws::ec2::Ec2Client;
use crate::schema::ec2::types::{
    ElasticIp, Image, Instance, InstanceState, KeyPair, LaunchTemplate, LaunchTemplateVersion,
    SecurityGroup, Snapshot, Subnet, TagFilter, Volume, Vpc,
};

#[derive(Default)]
pub struct Ec2Query;

#[Object]
impl Ec2Query {
    async fn instances(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        state: Option<InstanceState>,
        vpc_id: Option<String>,
        subnet_id: Option<String>,
        tags: Option<Vec<TagFilter>>,
    ) -> Result<Vec<Instance>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let state_str = state.map(|s| {
            match s {
                InstanceState::Pending => "pending",
                InstanceState::Running => "running",
                InstanceState::ShuttingDown => "shutting-down",
                InstanceState::Terminated => "terminated",
                InstanceState::Stopping => "stopping",
                InstanceState::Stopped => "stopped",
            }
            .to_string()
        });

        let tag_filters = tags.map(|ts| {
            ts.into_iter()
                .map(|t| (t.key, vec![t.value]))
                .collect::<Vec<_>>()
        });

        let aws_instances = ec2
            .describe_instances(ids, state_str, vpc_id, subnet_id, tag_filters)
            .await?;

        Ok(aws_instances.into_iter().map(Instance::from).collect())
    }

    async fn security_groups(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        name: Option<String>,
    ) -> Result<Vec<SecurityGroup>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let aws_sgs = ec2
            .describe_security_groups(ids, vpc_id, name)
            .await?;

        Ok(aws_sgs.into_iter().map(SecurityGroup::from).collect())
    }

    async fn vpcs(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<Vpc>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let aws_vpcs = ec2
            .describe_vpcs(ids)
            .await?;

        Ok(aws_vpcs.into_iter().map(Vpc::from).collect())
    }

    async fn subnets(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        az: Option<String>,
    ) -> Result<Vec<Subnet>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let aws_subnets = ec2
            .describe_subnets(ids, vpc_id, az)
            .await?;

        Ok(aws_subnets.into_iter().map(Subnet::from).collect())
    }

    async fn volumes(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        state: Option<String>,
    ) -> Result<Vec<Volume>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let aws_volumes = ec2
            .describe_volumes(ids, state)
            .await?;

        Ok(aws_volumes.into_iter().map(Volume::from).collect())
    }

    async fn key_pairs(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        name: Option<String>,
        fingerprint: Option<String>,
    ) -> Result<Vec<KeyPair>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let aws_kps = ec2
            .describe_key_pairs(ids, name, fingerprint)
            .await?;

        Ok(aws_kps.into_iter().map(KeyPair::from).collect())
    }

    async fn elastic_ips(
        &self,
        ctx: &Context<'_>,
        allocation_ids: Option<Vec<String>>,
        public_ips: Option<Vec<String>>,
        instance_id: Option<String>,
    ) -> Result<Vec<ElasticIp>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let aws_addresses = ec2
            .describe_addresses(allocation_ids, public_ips, instance_id)
            .await?;

        Ok(aws_addresses.into_iter().map(ElasticIp::from).collect())
    }

    async fn images(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        owners: Option<Vec<String>>,
        name: Option<String>,
        state: Option<String>,
        tags: Option<Vec<TagFilter>>,
    ) -> Result<Vec<Image>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let tag_filters = tags.map(|ts| {
            ts.into_iter()
                .map(|t| (t.key, vec![t.value]))
                .collect::<Vec<_>>()
        });

        let aws_images = ec2
            .describe_images(ids, owners, name, state, tag_filters)
            .await?;

        Ok(aws_images.into_iter().map(Image::from).collect())
    }

    async fn launch_templates(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<LaunchTemplate>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let items = ec2.describe_launch_templates(ids, names).await?;
        Ok(items.into_iter().map(LaunchTemplate::from).collect())
    }

    async fn launch_template_versions(
        &self,
        ctx: &Context<'_>,
        launch_template_id: String,
        versions: Option<Vec<String>>,
    ) -> Result<Vec<LaunchTemplateVersion>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let items = ec2
            .describe_launch_template_versions(launch_template_id, versions)
            .await?;
        Ok(items.into_iter().map(LaunchTemplateVersion::from).collect())
    }

    async fn snapshots(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        volume_id: Option<String>,
        state: Option<String>,
    ) -> Result<Vec<Snapshot>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let items = ec2.describe_snapshots(ids, volume_id, state).await?;
        Ok(items.into_iter().map(Snapshot::from).collect())
    }
}
