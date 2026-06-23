use aws_config::SdkConfig;
use aws_sdk_ec2::types::{Filter, InstanceStateName};

use crate::error::VaporError;

pub struct Ec2Client {
    inner: aws_sdk_ec2::Client,
}

impl Ec2Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ec2::Client::new(config),
        }
    }

    pub async fn describe_instances(
        &self,
        ids: Option<Vec<String>>,
        state: Option<String>,
        vpc_id: Option<String>,
        subnet_id: Option<String>,
        tags: Option<Vec<(String, Vec<String>)>>,
    ) -> Result<Vec<aws_sdk_ec2::types::Instance>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(state) = state {
            filters.push(
                Filter::builder()
                    .name("instance-state-name")
                    .values(state)
                    .build(),
            );
        }

        if let Some(vpc_id) = vpc_id {
            filters.push(
                Filter::builder()
                    .name("vpc-id")
                    .values(vpc_id)
                    .build(),
            );
        }

        if let Some(subnet_id) = subnet_id {
            filters.push(
                Filter::builder()
                    .name("subnet-id")
                    .values(subnet_id)
                    .build(),
            );
        }

        if let Some(ref tags) = tags {
            for (key, values) in tags {
                let filter_name = format!("tag:{key}");
                filters.push(
                    Filter::builder()
                        .name(filter_name)
                        .set_values(Some(values.clone()))
                        .build(),
                );
            }
        }

        let mut all_instances: Vec<aws_sdk_ec2::types::Instance> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_instances();

            if let Some(ref ids) = ids {
                request = request.set_instance_ids(Some(ids.clone()));
            }

            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for reservation in output.reservations() {
                for instance in reservation.instances() {
                    all_instances.push(instance.clone());
                }
            }

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_instances)
    }

    pub async fn describe_security_groups(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        name: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::SecurityGroup>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }

        if let Some(name) = name {
            filters.push(Filter::builder().name("group-name").values(name).build());
        }

        let mut all_groups: Vec<aws_sdk_ec2::types::SecurityGroup> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_security_groups();

            if let Some(ref ids) = ids {
                request = request.set_group_ids(Some(ids.clone()));
            }

            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_groups.extend(output.security_groups().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_groups)
    }

    pub async fn describe_vpcs(
        &self,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_ec2::types::Vpc>, VaporError> {
        let mut all_vpcs: Vec<aws_sdk_ec2::types::Vpc> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_vpcs();

            if let Some(ref ids) = ids {
                request = request.set_vpc_ids(Some(ids.clone()));
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_vpcs.extend(output.vpcs().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_vpcs)
    }

    pub async fn describe_subnets(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        az: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::Subnet>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }

        if let Some(az) = az {
            filters.push(
                Filter::builder()
                    .name("availability-zone")
                    .values(az)
                    .build(),
            );
        }

        let mut all_subnets: Vec<aws_sdk_ec2::types::Subnet> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_subnets();

            if let Some(ref ids) = ids {
                request = request.set_subnet_ids(Some(ids.clone()));
            }

            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_subnets.extend(output.subnets().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_subnets)
    }

    pub async fn describe_volumes(
        &self,
        ids: Option<Vec<String>>,
        state: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::Volume>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(state) = state {
            filters.push(Filter::builder().name("status").values(state).build());
        }

        let mut all_volumes: Vec<aws_sdk_ec2::types::Volume> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_volumes();

            if let Some(ref ids) = ids {
                request = request.set_volume_ids(Some(ids.clone()));
            }

            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_volumes.extend(output.volumes().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_volumes)
    }

    pub async fn start_instances(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<(String, InstanceStateName, InstanceStateName)>, VaporError> {
        let output = self.inner
            .start_instances()
            .set_instance_ids(Some(ids))
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        let changes = output
            .starting_instances()
            .iter()
            .filter_map(|c| {
                let id = c.instance_id()?.to_string();
                let prev = c.previous_state()?.name()?.clone();
                let curr = c.current_state()?.name()?.clone();
                Some((id, prev, curr))
            })
            .collect();

        Ok(changes)
    }

    pub async fn stop_instances(
        &self,
        ids: Vec<String>,
        force: bool,
    ) -> Result<Vec<(String, InstanceStateName, InstanceStateName)>, VaporError> {
        let output = self.inner
            .stop_instances()
            .set_instance_ids(Some(ids))
            .force(force)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        let changes = output
            .stopping_instances()
            .iter()
            .filter_map(|c| {
                let id = c.instance_id()?.to_string();
                let prev = c.previous_state()?.name()?.clone();
                let curr = c.current_state()?.name()?.clone();
                Some((id, prev, curr))
            })
            .collect();

        Ok(changes)
    }

    pub async fn terminate_instances(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<(String, InstanceStateName, InstanceStateName)>, VaporError> {
        let output = self.inner
            .terminate_instances()
            .set_instance_ids(Some(ids))
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        let changes = output
            .terminating_instances()
            .iter()
            .filter_map(|c| {
                let id = c.instance_id()?.to_string();
                let prev = c.previous_state()?.name()?.clone();
                let curr = c.current_state()?.name()?.clone();
                Some((id, prev, curr))
            })
            .collect();

        Ok(changes)
    }

    pub async fn describe_key_pairs(
        &self,
        ids: Option<Vec<String>>,
        name: Option<String>,
        fingerprint: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::KeyPairInfo>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(name) = name {
            filters.push(Filter::builder().name("key-name").values(name).build());
        }

        if let Some(fingerprint) = fingerprint {
            filters.push(Filter::builder().name("fingerprint").values(fingerprint).build());
        }

        let mut request = self.inner.describe_key_pairs();

        if let Some(ids) = ids {
            request = request.set_key_pair_ids(Some(ids));
        }

        if !filters.is_empty() {
            request = request.set_filters(Some(filters));
        }

        let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.key_pairs().to_vec())
    }

    pub async fn describe_images(
        &self,
        ids: Option<Vec<String>>,
        owners: Option<Vec<String>>,
        name: Option<String>,
        state: Option<String>,
        tags: Option<Vec<(String, Vec<String>)>>,
    ) -> Result<Vec<aws_sdk_ec2::types::Image>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(name) = name {
            filters.push(Filter::builder().name("name").values(name).build());
        }

        if let Some(state) = state {
            filters.push(Filter::builder().name("state").values(state).build());
        }

        if let Some(ref tags) = tags {
            for (key, values) in tags {
                let filter_name = format!("tag:{key}");
                filters.push(
                    Filter::builder()
                        .name(filter_name)
                        .set_values(Some(values.clone()))
                        .build(),
                );
            }
        }

        let effective_owners = owners.unwrap_or_else(|| vec!["self".to_string()]);

        let mut request = self.inner.describe_images();

        if let Some(ids) = ids {
            request = request.set_image_ids(Some(ids));
        }

        request = request.set_owners(Some(effective_owners));

        if !filters.is_empty() {
            request = request.set_filters(Some(filters));
        }

        let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.images().to_vec())
    }

    pub async fn describe_addresses(
        &self,
        allocation_ids: Option<Vec<String>>,
        public_ips: Option<Vec<String>>,
        instance_id: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::Address>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(instance_id) = instance_id {
            filters.push(Filter::builder().name("instance-id").values(instance_id).build());
        }

        let mut request = self.inner.describe_addresses();

        if let Some(ids) = allocation_ids {
            request = request.set_allocation_ids(Some(ids));
        }

        if let Some(ips) = public_ips {
            request = request.set_public_ips(Some(ips));
        }

        if !filters.is_empty() {
            request = request.set_filters(Some(filters));
        }

        let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.addresses().to_vec())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn run_instances(
        &self,
        image_id: String,
        instance_type: String,
        min_count: i32,
        max_count: i32,
        key_name: Option<String>,
        security_group_ids: Option<Vec<String>>,
        subnet_id: Option<String>,
        tags: Option<Vec<(String, String)>>,
    ) -> Result<Vec<aws_sdk_ec2::types::Instance>, VaporError> {
        use aws_sdk_ec2::types::{InstanceType, ResourceType, Tag as SdkTag, TagSpecification};


        let it = InstanceType::from(instance_type.as_str());

        let mut request = self.inner
            .run_instances()
            .image_id(image_id)
            .instance_type(it)
            .min_count(min_count)
            .max_count(max_count);

        if let Some(kn) = key_name {
            request = request.key_name(kn);
        }

        if let Some(sg_ids) = security_group_ids {
            request = request.set_security_group_ids(Some(sg_ids));
        }

        if let Some(sn) = subnet_id {
            request = request.subnet_id(sn);
        }

        if let Some(tags) = tags {
            let sdk_tags: Vec<SdkTag> = tags
                .into_iter()
                .map(|(k, v)| SdkTag::builder().key(k).value(v).build())
                .collect();
            let tag_spec = TagSpecification::builder()
                .resource_type(ResourceType::Instance)
                .set_tags(Some(sdk_tags))
                .build();
            request = request.tag_specifications(tag_spec);
        }

        let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.instances().to_vec())
    }

    pub async fn reboot_instances(&self, ids: Vec<String>) -> Result<(), VaporError> {
        self.inner
            .reboot_instances()
            .set_instance_ids(Some(ids))
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(())
    }

    pub async fn describe_route_tables(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::RouteTable>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }
        let mut all: Vec<aws_sdk_ec2::types::RouteTable> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self.inner.describe_route_tables();
            if let Some(ref ids) = ids {
                request = request.set_route_table_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }
            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.route_tables().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(all)
    }

    pub async fn describe_network_acls(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::NetworkAcl>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }
        let mut all: Vec<aws_sdk_ec2::types::NetworkAcl> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self.inner.describe_network_acls();
            if let Some(ref ids) = ids {
                request = request.set_network_acl_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }
            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.network_acls().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(all)
    }

    pub async fn describe_internet_gateways(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::InternetGateway>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("attachment.vpc-id").values(vpc_id).build());
        }
        let mut all: Vec<aws_sdk_ec2::types::InternetGateway> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self.inner.describe_internet_gateways();
            if let Some(ref ids) = ids {
                request = request.set_internet_gateway_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }
            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.internet_gateways().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(all)
    }

    pub async fn describe_nat_gateways(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        state: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::NatGateway>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }
        if let Some(state) = state {
            filters.push(Filter::builder().name("state").values(state).build());
        }
        let mut all: Vec<aws_sdk_ec2::types::NatGateway> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self.inner.describe_nat_gateways();
            if let Some(ref ids) = ids {
                request = request.set_nat_gateway_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filter(Some(filters.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }
            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.nat_gateways().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(all)
    }

    pub async fn describe_vpc_endpoints(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        service_name: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::VpcEndpoint>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }
        if let Some(service_name) = service_name {
            filters.push(Filter::builder().name("service-name").values(service_name).build());
        }
        let mut all: Vec<aws_sdk_ec2::types::VpcEndpoint> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self.inner.describe_vpc_endpoints();
            if let Some(ref ids) = ids {
                request = request.set_vpc_endpoint_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }
            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.vpc_endpoints().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(all)
    }

    pub async fn describe_transit_gateways(
        &self,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_ec2::types::TransitGateway>, VaporError> {
        let mut all: Vec<aws_sdk_ec2::types::TransitGateway> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self.inner.describe_transit_gateways();
            if let Some(ref ids) = ids {
                request = request.set_transit_gateway_ids(Some(ids.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }
            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.transit_gateways().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(all)
    }

    pub async fn describe_launch_templates(
        &self,
        ids: Option<Vec<String>>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_ec2::types::LaunchTemplate>, VaporError> {
        let mut all: Vec<aws_sdk_ec2::types::LaunchTemplate> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self.inner.describe_launch_templates();
            if let Some(ref ids) = ids {
                request = request.set_launch_template_ids(Some(ids.clone()));
            }
            if let Some(ref names) = names {
                request = request.set_launch_template_names(Some(names.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }
            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.launch_templates().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(all)
    }

    pub async fn describe_launch_template_versions(
        &self,
        launch_template_id: String,
        versions: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_ec2::types::LaunchTemplateVersion>, VaporError> {
        let mut all: Vec<aws_sdk_ec2::types::LaunchTemplateVersion> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self
                .inner
                .describe_launch_template_versions()
                .launch_template_id(&launch_template_id);
            if let Some(ref vers) = versions {
                request = request.set_versions(Some(vers.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }
            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.launch_template_versions().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(all)
    }

    pub async fn describe_snapshots(
        &self,
        ids: Option<Vec<String>>,
        volume_id: Option<String>,
        state: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::Snapshot>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(vol_id) = volume_id {
            filters.push(Filter::builder().name("volume-id").values(vol_id).build());
        }

        if let Some(s) = state {
            filters.push(Filter::builder().name("status").values(s).build());
        }

        let mut all: Vec<aws_sdk_ec2::types::Snapshot> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self
                .inner
                .describe_snapshots()
                .set_owner_ids(Some(vec!["self".to_string()]));

            if let Some(ref ids) = ids {
                request = request.set_snapshot_ids(Some(ids.clone()));
            }

            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.snapshots().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all)
    }

    pub async fn describe_flow_logs(
        &self,
        resource_id: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::FlowLog>, VaporError> {
        let mut filters: Vec<Filter> = vec![
            Filter::builder().name("resource-type").values("VPC").build(),
        ];

        if let Some(ref id) = resource_id {
            filters.push(Filter::builder().name("resource-id").values(id).build());
        }

        let mut all: Vec<aws_sdk_ec2::types::FlowLog> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self
                .inner
                .describe_flow_logs()
                .set_filter(Some(filters.clone()));

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all.extend(output.flow_logs().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all)
    }
}
