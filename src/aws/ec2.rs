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

    /// Lists instances, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `DescribeInstancesInput::max_results` (confirmed `Option<i32>`,
    /// verified against pinned `aws-sdk-ec2` 1.233.0's
    /// `operation/describe_instances/_describe_instances_input.rs`).
    #[allow(clippy::too_many_arguments)] // wide AWS DescribeInstances parameter set
    pub async fn describe_instances(
        &self,
        ids: Option<Vec<String>>,
        state: Option<String>,
        vpc_id: Option<String>,
        subnet_id: Option<String>,
        tags: Option<Vec<(String, Vec<String>)>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::Instance>, Option<String>), VaporError> {
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
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
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
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_instances();
            if let Some(ref ids) = ids {
                request = request.set_instance_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_instances.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            for reservation in output.reservations.into_iter().flatten() {
                all_instances.extend(reservation.instances.into_iter().flatten());
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all_instances.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all_instances, token))
    }

    /// Lists security groups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_instances` above.
    pub async fn describe_security_groups(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::SecurityGroup>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }

        if let Some(name) = name {
            filters.push(Filter::builder().name("group-name").values(name).build());
        }

        let mut all_groups: Vec<aws_sdk_ec2::types::SecurityGroup> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_security_groups();
            if let Some(ref ids) = ids {
                request = request.set_group_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_groups.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_groups.extend(output.security_groups.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all_groups.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all_groups, token))
    }

    /// Lists VPCs, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. Same server-side-capping shape as
    /// `describe_instances` above.
    pub async fn describe_vpcs(
        &self,
        ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::Vpc>, Option<String>), VaporError> {
        let mut all_vpcs: Vec<aws_sdk_ec2::types::Vpc> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_vpcs();
            if let Some(ref ids) = ids {
                request = request.set_vpc_ids(Some(ids.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_vpcs.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_vpcs.extend(output.vpcs.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all_vpcs.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all_vpcs, token))
    }

    /// Lists subnets, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_instances` above.
    pub async fn describe_subnets(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        az: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::Subnet>, Option<String>), VaporError> {
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
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_subnets();
            if let Some(ref ids) = ids {
                request = request.set_subnet_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_subnets.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_subnets.extend(output.subnets.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all_subnets.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all_subnets, token))
    }

    /// Lists volumes, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_instances` above.
    pub async fn describe_volumes(
        &self,
        ids: Option<Vec<String>>,
        state: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::Volume>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(state) = state {
            filters.push(Filter::builder().name("status").values(state).build());
        }

        let mut all_volumes: Vec<aws_sdk_ec2::types::Volume> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_volumes();
            if let Some(ref ids) = ids {
                request = request.set_volume_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_volumes.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_volumes.extend(output.volumes.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all_volumes.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all_volumes, token))
    }

    pub async fn start_instances(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<(String, InstanceStateName, InstanceStateName)>, VaporError> {
        let output = self
            .inner
            .start_instances()
            .set_instance_ids(Some(ids))
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

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
        let output = self
            .inner
            .stop_instances()
            .set_instance_ids(Some(ids))
            .force(force)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

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
        let output = self
            .inner
            .terminate_instances()
            .set_instance_ids(Some(ids))
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

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

    /// Describes key pairs. `DescribeKeyPairsInput`/`Output` have no
    /// `max_results`/`next_token` fields at all (verified against pinned
    /// `aws-sdk-ec2` 1.233.0's `operation/describe_key_pairs/*.rs`) — a
    /// genuinely non-paginated, single-call op (sts/opensearch-class
    /// carve-out), hence no `limit`/`next_token` params here.
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
            filters.push(
                Filter::builder()
                    .name("fingerprint")
                    .values(fingerprint)
                    .build(),
            );
        }

        let mut request = self.inner.describe_key_pairs();

        if let Some(ids) = ids {
            request = request.set_key_pair_ids(Some(ids));
        }

        if !filters.is_empty() {
            request = request.set_filters(Some(filters));
        }

        let output = request.send().await.map_err(crate::error::sdk_err)?;
        Ok(output.key_pairs().to_vec())
    }

    /// Lists images, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `DescribeImagesInput::max_results` (confirmed `Option<i32>`, verified
    /// against pinned `aws-sdk-ec2` 1.233.0's
    /// `operation/describe_images/_describe_images_input.rs`).
    #[allow(clippy::too_many_arguments)] // wide AWS DescribeImages parameter set
    pub async fn describe_images(
        &self,
        ids: Option<Vec<String>>,
        owners: Option<Vec<String>>,
        name: Option<String>,
        state: Option<String>,
        tags: Option<Vec<(String, Vec<String>)>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::Image>, Option<String>), VaporError> {
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

        let mut all: Vec<aws_sdk_ec2::types::Image> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self
                .inner
                .describe_images()
                .set_owners(Some(effective_owners.clone()));
            if let Some(ref ids) = ids {
                request = request.set_image_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.images.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all, token))
    }

    /// Describes elastic IPs. `DescribeAddressesInput`/`Output` have no
    /// `max_results`/`next_token` fields at all (verified against pinned
    /// `aws-sdk-ec2` 1.233.0's `operation/describe_addresses/*.rs`) — a
    /// genuinely non-paginated, single-call op (sts/opensearch-class
    /// carve-out), hence no `limit`/`next_token` params here.
    pub async fn describe_addresses(
        &self,
        allocation_ids: Option<Vec<String>>,
        public_ips: Option<Vec<String>>,
        instance_id: Option<String>,
    ) -> Result<Vec<aws_sdk_ec2::types::Address>, VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(instance_id) = instance_id {
            filters.push(
                Filter::builder()
                    .name("instance-id")
                    .values(instance_id)
                    .build(),
            );
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

        let output = request.send().await.map_err(crate::error::sdk_err)?;
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

        let mut request = self
            .inner
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

        let output = request.send().await.map_err(crate::error::sdk_err)?;
        Ok(output.instances().to_vec())
    }

    pub async fn reboot_instances(&self, ids: Vec<String>) -> Result<(), VaporError> {
        self.inner
            .reboot_instances()
            .set_instance_ids(Some(ids))
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(())
    }

    /// Lists route tables, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `DescribeRouteTablesInput::max_results` (confirmed `Option<i32>`, no
    /// documented minimum, verified against pinned `aws-sdk-ec2` 1.233.0's
    /// `operation/describe_route_tables/_describe_route_tables_input.rs`).
    pub async fn describe_route_tables(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::RouteTable>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }
        let mut all: Vec<aws_sdk_ec2::types::RouteTable> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_route_tables();
            if let Some(ref ids) = ids {
                request = request.set_route_table_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.route_tables.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all, token))
    }

    /// Lists network ACLs, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_route_tables` above.
    pub async fn describe_network_acls(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::NetworkAcl>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }
        let mut all: Vec<aws_sdk_ec2::types::NetworkAcl> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_network_acls();
            if let Some(ref ids) = ids {
                request = request.set_network_acl_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.network_acls.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all, token))
    }

    /// Lists internet gateways, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_route_tables` above.
    pub async fn describe_internet_gateways(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::InternetGateway>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(
                Filter::builder()
                    .name("attachment.vpc-id")
                    .values(vpc_id)
                    .build(),
            );
        }
        let mut all: Vec<aws_sdk_ec2::types::InternetGateway> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_internet_gateways();
            if let Some(ref ids) = ids {
                request = request.set_internet_gateway_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.internet_gateways.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all, token))
    }

    /// Lists NAT gateways, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_route_tables` above.
    pub async fn describe_nat_gateways(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        state: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::NatGateway>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }
        if let Some(state) = state {
            filters.push(Filter::builder().name("state").values(state).build());
        }
        let mut all: Vec<aws_sdk_ec2::types::NatGateway> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_nat_gateways();
            if let Some(ref ids) = ids {
                request = request.set_nat_gateway_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filter(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.nat_gateways.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all, token))
    }

    /// Lists VPC endpoints, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_route_tables` above.
    pub async fn describe_vpc_endpoints(
        &self,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        service_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::VpcEndpoint>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = Vec::new();
        if let Some(vpc_id) = vpc_id {
            filters.push(Filter::builder().name("vpc-id").values(vpc_id).build());
        }
        if let Some(service_name) = service_name {
            filters.push(
                Filter::builder()
                    .name("service-name")
                    .values(service_name)
                    .build(),
            );
        }
        let mut all: Vec<aws_sdk_ec2::types::VpcEndpoint> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_vpc_endpoints();
            if let Some(ref ids) = ids {
                request = request.set_vpc_endpoint_ids(Some(ids.clone()));
            }
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.vpc_endpoints.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all, token))
    }

    /// Lists transit gateways, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_route_tables` above.
    pub async fn describe_transit_gateways(
        &self,
        ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::TransitGateway>, Option<String>), VaporError> {
        let mut all: Vec<aws_sdk_ec2::types::TransitGateway> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_transit_gateways();
            if let Some(ref ids) = ids {
                request = request.set_transit_gateway_ids(Some(ids.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.transit_gateways.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all, token))
    }

    /// Lists launch templates, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_instances` above.
    pub async fn describe_launch_templates(
        &self,
        ids: Option<Vec<String>>,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::LaunchTemplate>, Option<String>), VaporError> {
        let mut all: Vec<aws_sdk_ec2::types::LaunchTemplate> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self.inner.describe_launch_templates();
            if let Some(ref ids) = ids {
                request = request.set_launch_template_ids(Some(ids.clone()));
            }
            if let Some(ref names) = names {
                request = request.set_launch_template_names(Some(names.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.launch_templates.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all, token))
    }

    /// Lists launch template versions, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. Same
    /// server-side-capping shape as `describe_instances` above.
    pub async fn describe_launch_template_versions(
        &self,
        launch_template_id: String,
        versions: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_ec2::types::LaunchTemplateVersion>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut all: Vec<aws_sdk_ec2::types::LaunchTemplateVersion> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self
                .inner
                .describe_launch_template_versions()
                .launch_template_id(&launch_template_id);
            if let Some(ref versions) = versions {
                request = request.set_versions(Some(versions.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.launch_template_versions.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all, token))
    }

    /// Lists snapshots, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_instances` above.
    pub async fn describe_snapshots(
        &self,
        ids: Option<Vec<String>>,
        volume_id: Option<String>,
        state: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::Snapshot>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = Vec::new();

        if let Some(vol_id) = volume_id {
            filters.push(Filter::builder().name("volume-id").values(vol_id).build());
        }

        if let Some(s) = state {
            filters.push(Filter::builder().name("status").values(s).build());
        }

        let mut all: Vec<aws_sdk_ec2::types::Snapshot> = Vec::new();
        let mut token = next_token;
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
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.snapshots.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all, token))
    }

    /// Lists VPC flow logs, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side-capping
    /// shape as `describe_route_tables` above.
    pub async fn describe_flow_logs(
        &self,
        resource_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ec2::types::FlowLog>, Option<String>), VaporError> {
        let mut filters: Vec<Filter> = vec![Filter::builder()
            .name("resource-type")
            .values("VPC")
            .build()];

        if let Some(ref id) = resource_id {
            filters.push(Filter::builder().name("resource-id").values(id).build());
        }

        let mut all: Vec<aws_sdk_ec2::types::FlowLog> = Vec::new();
        let mut token = next_token;
        loop {
            let mut request = self
                .inner
                .describe_flow_logs()
                .set_filter(Some(filters.clone()));
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all.extend(output.flow_logs.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        ec2_error_response, request, sdk_config, xml_response, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://ec2.us-east-1.amazonaws.com/";

    /// `run_instances` auto-generates a random `ClientToken` (ec2-sdk-side
    /// idempotency token) whenever the caller doesn't supply one, making the
    /// request body non-deterministic against `StaticReplayClient`'s
    /// map-based `UrlEncodedForm` body match. Pin it via
    /// `idempotency_token_provider` (same mechanism the pinned SDK's own
    /// `Config::apply_test_defaults` uses) so `ClientToken` is a fixed,
    /// assertable value instead — `Ec2Client::new` only takes a generic
    /// `SdkConfig`, which has no such field, so this bypasses it and builds
    /// the wrapper directly (its `inner` field is visible to this submodule).
    fn ec2_client_with_fixed_token(http_client: StaticReplayClient) -> Ec2Client {
        let ec2_config = aws_sdk_ec2::config::Builder::from(&sdk_config(http_client))
            .idempotency_token_provider("test-client-token")
            .build();
        Ec2Client {
            inner: aws_sdk_ec2::Client::from_conf(ec2_config),
        }
    }

    #[tokio::test]
    async fn describe_instances_happy_path_with_ids_and_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeInstances&Version=2016-11-15&InstanceId.1=i-1&\
                 Filter.1.Name=instance-state-name&Filter.1.Value.1=running&\
                 Filter.2.Name=vpc-id&Filter.2.Value.1=vpc-1&\
                 Filter.3.Name=subnet-id&Filter.3.Value.1=subnet-1&\
                 Filter.4.Name=tag%3AName&Filter.4.Value.1=web",
            ),
            xml_response(
                200,
                "<DescribeInstancesResponse><reservationSet><item><instancesSet>\
                 <item><instanceId>i-1</instanceId></item>\
                 </instancesSet></item></reservationSet></DescribeInstancesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (instances, token) = client
            .describe_instances(
                Some(vec!["i-1".to_string()]),
                Some("running".to_string()),
                Some("vpc-1".to_string()),
                Some("subnet-1".to_string()),
                Some(vec![("Name".to_string(), vec!["web".to_string()])]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_id(), Some("i-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instances_resumes_from_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeInstances&Version=2016-11-15&NextToken=cursor-a"),
            xml_response(200, "<DescribeInstancesResponse><reservationSet></reservationSet></DescribeInstancesResponse>"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (instances, token) = client
            .describe_instances(
                None,
                None,
                None,
                None,
                None,
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(instances.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instances_stops_at_limit_with_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeInstances&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeInstancesResponse><reservationSet><item><instancesSet>\
                 <item><instanceId>i-1</instanceId></item>\
                 </instancesSet></item></reservationSet><nextToken>page2</nextToken></DescribeInstancesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (instances, token) = client
            .describe_instances(None, None, None, None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instances_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeInstances&Version=2016-11-15&MaxResults=100"),
                xml_response(
                    200,
                    "<DescribeInstancesResponse><reservationSet><item><instancesSet>\
                     <item><instanceId>i-1</instanceId></item>\
                     </instancesSet></item></reservationSet><nextToken>p2</nextToken></DescribeInstancesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeInstances&Version=2016-11-15&NextToken=p2&MaxResults=99"),
                xml_response(
                    200,
                    "<DescribeInstancesResponse><reservationSet><item><instancesSet>\
                     <item><instanceId>i-2</instanceId></item>\
                     </instancesSet></item></reservationSet></DescribeInstancesResponse>",
                ),
            ),
        ]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (instances, token) = client
            .describe_instances(None, None, None, None, None, Some(100), None)
            .await
            .unwrap();

        assert_eq!(instances.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeInstances&Version=2016-11-15"),
            ec2_error_response("InvalidInstanceID.NotFound", "no such instance"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_instances(None, None, None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInstanceID.NotFound".to_string()));
                assert_eq!(message, "no such instance");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_security_groups_happy_path_with_ids_and_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeSecurityGroups&Version=2016-11-15&GroupId.1=sg-1&\
                 Filter.1.Name=vpc-id&Filter.1.Value.1=vpc-1&\
                 Filter.2.Name=group-name&Filter.2.Value.1=web-sg",
            ),
            xml_response(
                200,
                "<DescribeSecurityGroupsResponse><securityGroupInfo>\
                 <item><groupId>sg-1</groupId><groupName>web-sg</groupName></item>\
                 </securityGroupInfo></DescribeSecurityGroupsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .describe_security_groups(
                Some(vec!["sg-1".to_string()]),
                Some("vpc-1".to_string()),
                Some("web-sg".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].group_id(), Some("sg-1"));
        assert_eq!(groups[0].group_name(), Some("web-sg"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_security_groups_stops_at_limit_with_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeSecurityGroups&Version=2016-11-15&MaxResults=1",
            ),
            xml_response(
                200,
                "<DescribeSecurityGroupsResponse><securityGroupInfo>\
                 <item><groupId>sg-a</groupId></item>\
                 </securityGroupInfo><nextToken>p2</nextToken></DescribeSecurityGroupsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .describe_security_groups(None, None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_security_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeSecurityGroups&Version=2016-11-15"),
            ec2_error_response("InvalidGroup.NotFound", "no such group"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_security_groups(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidGroup.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_vpcs_happy_path_with_ids() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeVpcs&Version=2016-11-15&VpcId.1=vpc-1&VpcId.2=vpc-2",
            ),
            xml_response(
                200,
                "<DescribeVpcsResponse><vpcSet>\
                 <item><vpcId>vpc-1</vpcId></item><item><vpcId>vpc-2</vpcId></item>\
                 </vpcSet></DescribeVpcsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (vpcs, token) = client
            .describe_vpcs(
                Some(vec!["vpc-1".to_string(), "vpc-2".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(vpcs.len(), 2);
        assert_eq!(vpcs[0].vpc_id(), Some("vpc-1"));
        assert_eq!(vpcs[1].vpc_id(), Some("vpc-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_vpcs_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeVpcs&Version=2016-11-15&MaxResults=100"),
                xml_response(
                    200,
                    "<DescribeVpcsResponse><vpcSet><item><vpcId>vpc-a</vpcId></item></vpcSet>\
                     <nextToken>p2</nextToken></DescribeVpcsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeVpcs&Version=2016-11-15&NextToken=p2&MaxResults=99"),
                xml_response(200, "<DescribeVpcsResponse><vpcSet><item><vpcId>vpc-b</vpcId></item></vpcSet></DescribeVpcsResponse>"),
            ),
        ]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (vpcs, token) = client.describe_vpcs(None, Some(100), None).await.unwrap();

        assert_eq!(vpcs.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_vpcs_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeVpcs&Version=2016-11-15"),
            ec2_error_response("InvalidVpcID.NotFound", "no such vpc"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client.describe_vpcs(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidVpcID.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subnets_happy_path_with_ids_and_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeSubnets&Version=2016-11-15&\
                 Filter.1.Name=vpc-id&Filter.1.Value.1=vpc-1&\
                 Filter.2.Name=availability-zone&Filter.2.Value.1=us-east-1a&\
                 SubnetId.1=subnet-1",
            ),
            xml_response(
                200,
                "<DescribeSubnetsResponse><subnetSet>\
                 <item><subnetId>subnet-1</subnetId><vpcId>vpc-1</vpcId></item>\
                 </subnetSet></DescribeSubnetsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (subnets, token) = client
            .describe_subnets(
                Some(vec!["subnet-1".to_string()]),
                Some("vpc-1".to_string()),
                Some("us-east-1a".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(subnets.len(), 1);
        assert_eq!(subnets[0].subnet_id(), Some("subnet-1"));
        assert_eq!(subnets[0].vpc_id(), Some("vpc-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subnets_stops_at_limit_with_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeSubnets&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeSubnetsResponse><subnetSet><item><subnetId>subnet-a</subnetId></item></subnetSet>\
                 <nextToken>p2</nextToken></DescribeSubnetsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (subnets, token) = client
            .describe_subnets(None, None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(subnets.len(), 1);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subnets_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeSubnets&Version=2016-11-15"),
            ec2_error_response("InvalidSubnetID.NotFound", "no such subnet"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_subnets(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidSubnetID.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_volumes_happy_path_with_ids_and_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeVolumes&Version=2016-11-15&VolumeId.1=vol-1&\
                 Filter.1.Name=status&Filter.1.Value.1=in-use",
            ),
            xml_response(200, "<DescribeVolumesResponse><volumeSet><item><volumeId>vol-1</volumeId></item></volumeSet></DescribeVolumesResponse>"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (volumes, token) = client
            .describe_volumes(
                Some(vec!["vol-1".to_string()]),
                Some("in-use".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].volume_id(), Some("vol-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_volumes_stops_at_limit_with_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeVolumes&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeVolumesResponse><volumeSet><item><volumeId>vol-a</volumeId></item></volumeSet>\
                 <nextToken>p2</nextToken></DescribeVolumesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (volumes, token) = client
            .describe_volumes(None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(volumes.len(), 1);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_volumes_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeVolumes&Version=2016-11-15"),
            ec2_error_response("InvalidVolume.NotFound", "no such volume"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_volumes(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidVolume.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_key_pairs_happy_path_with_ids_and_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeKeyPairs&Version=2016-11-15&KeyPairId.1=key-1&\
                 Filter.1.Name=key-name&Filter.1.Value.1=mykey&\
                 Filter.2.Name=fingerprint&Filter.2.Value.1=ab%3Acd",
            ),
            xml_response(
                200,
                "<DescribeKeyPairsResponse><keySet>\
                 <item><keyPairId>key-1</keyPairId><keyName>mykey</keyName></item>\
                 </keySet></DescribeKeyPairsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let key_pairs = client
            .describe_key_pairs(
                Some(vec!["key-1".to_string()]),
                Some("mykey".to_string()),
                Some("ab:cd".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(key_pairs.len(), 1);
        assert_eq!(key_pairs[0].key_pair_id(), Some("key-1"));
        assert_eq!(key_pairs[0].key_name(), Some("mykey"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_key_pairs_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeKeyPairs&Version=2016-11-15"),
            ec2_error_response("InvalidKeyPair.NotFound", "no such key pair"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_key_pairs(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidKeyPair.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_images_happy_path_with_default_owner_ids_and_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeImages&Version=2016-11-15&ImageId.1=ami-1&Owner.1=self&\
                 Filter.1.Name=name&Filter.1.Value.1=my-image&\
                 Filter.2.Name=state&Filter.2.Value.1=available&\
                 Filter.3.Name=tag%3Aenv&Filter.3.Value.1=prod",
            ),
            xml_response(
                200,
                "<DescribeImagesResponse><imagesSet>\
                 <item><imageId>ami-1</imageId><name>my-image</name></item>\
                 </imagesSet></DescribeImagesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (images, token) = client
            .describe_images(
                Some(vec!["ami-1".to_string()]),
                None,
                Some("my-image".to_string()),
                Some("available".to_string()),
                Some(vec![("env".to_string(), vec!["prod".to_string()])]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].image_id(), Some("ami-1"));
        assert_eq!(images[0].name(), Some("my-image"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_images_stops_at_limit_with_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeImages&Version=2016-11-15&Owner.1=self&MaxResults=1"),
            xml_response(
                200,
                "<DescribeImagesResponse><imagesSet><item><imageId>ami-a</imageId></item></imagesSet>\
                 <nextToken>p2</nextToken></DescribeImagesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (images, token) = client
            .describe_images(None, None, None, None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_images_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeImages&Version=2016-11-15&Owner.1=self",
            ),
            ec2_error_response("InvalidAMIID.NotFound", "no such image"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_images(None, None, None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidAMIID.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_addresses_happy_path_with_all_params() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeAddresses&Version=2016-11-15&PublicIp.1=1.2.3.4&\
                 Filter.1.Name=instance-id&Filter.1.Value.1=i-1&\
                 AllocationId.1=eipalloc-1",
            ),
            xml_response(
                200,
                "<DescribeAddressesResponse><addressesSet>\
                 <item><allocationId>eipalloc-1</allocationId><publicIp>1.2.3.4</publicIp></item>\
                 </addressesSet></DescribeAddressesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let addresses = client
            .describe_addresses(
                Some(vec!["eipalloc-1".to_string()]),
                Some(vec!["1.2.3.4".to_string()]),
                Some("i-1".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].allocation_id(), Some("eipalloc-1"));
        assert_eq!(addresses[0].public_ip(), Some("1.2.3.4"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_addresses_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeAddresses&Version=2016-11-15"),
            ec2_error_response("InvalidAddress.NotFound", "no such address"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_addresses(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidAddress.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn run_instances_happy_path_with_all_params() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=RunInstances&Version=2016-11-15&\
                 ImageId=ami-1&InstanceType=t3.micro&\
                 KeyName=mykey&MaxCount=1&MinCount=1&SecurityGroupId.1=sg-1&SubnetId=subnet-1&\
                 TagSpecification.1.ResourceType=instance&TagSpecification.1.Tag.1.Key=Name&\
                 TagSpecification.1.Tag.1.Value=web1&ClientToken=test-client-token",
            ),
            xml_response(
                200,
                "<RunInstancesResponse><instancesSet><item><instanceId>i-1</instanceId></item></instancesSet></RunInstancesResponse>",
            ),
        )]);
        let client = ec2_client_with_fixed_token(http_client.clone());

        let instances = client
            .run_instances(
                "ami-1".to_string(),
                "t3.micro".to_string(),
                1,
                1,
                Some("mykey".to_string()),
                Some(vec!["sg-1".to_string()]),
                Some("subnet-1".to_string()),
                Some(vec![("Name".to_string(), "web1".to_string())]),
            )
            .await
            .unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_id(), Some("i-1"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn run_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=RunInstances&Version=2016-11-15&\
                 ImageId=ami-1&InstanceType=t3.micro&MaxCount=1&MinCount=1&ClientToken=test-client-token",
            ),
            ec2_error_response("InsufficientInstanceCapacity", "not enough capacity"),
        )]);
        let client = ec2_client_with_fixed_token(http_client.clone());

        let err = client
            .run_instances(
                "ami-1".to_string(),
                "t3.micro".to_string(),
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InsufficientInstanceCapacity".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn reboot_instances_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=RebootInstances&Version=2016-11-15&InstanceId.1=i-1&InstanceId.2=i-2"),
            xml_response(200, "<RebootInstancesResponse><requestId>test</requestId><return>true</return></RebootInstancesResponse>"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        client
            .reboot_instances(vec!["i-1".to_string(), "i-2".to_string()])
            .await
            .unwrap();

        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn reboot_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=RebootInstances&Version=2016-11-15&InstanceId.1=i-1",
            ),
            ec2_error_response("IncorrectInstanceState", "instance not running"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .reboot_instances(vec!["i-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("IncorrectInstanceState".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn start_instances_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=StartInstances&Version=2016-11-15&InstanceId.1=i-1",
            ),
            xml_response(
                200,
                "<StartInstancesResponse><instancesSet><item><instanceId>i-1</instanceId>\
                 <currentState><code>0</code><name>pending</name></currentState>\
                 <previousState><code>80</code><name>stopped</name></previousState>\
                 </item></instancesSet></StartInstancesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let changes = client
            .start_instances(vec!["i-1".to_string()])
            .await
            .unwrap();

        assert_eq!(
            changes,
            vec![(
                "i-1".to_string(),
                InstanceStateName::Stopped,
                InstanceStateName::Pending
            )]
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn start_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=StartInstances&Version=2016-11-15&InstanceId.1=i-1",
            ),
            ec2_error_response("IncorrectInstanceState", "instance not stopped"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .start_instances(vec!["i-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("IncorrectInstanceState".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn stop_instances_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=StopInstances&Version=2016-11-15&InstanceId.1=i-1&Force=true",
            ),
            xml_response(
                200,
                "<StopInstancesResponse><instancesSet><item><instanceId>i-1</instanceId>\
                 <currentState><code>64</code><name>stopping</name></currentState>\
                 <previousState><code>16</code><name>running</name></previousState>\
                 </item></instancesSet></StopInstancesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let changes = client
            .stop_instances(vec!["i-1".to_string()], true)
            .await
            .unwrap();

        assert_eq!(
            changes,
            vec![(
                "i-1".to_string(),
                InstanceStateName::Running,
                InstanceStateName::Stopping
            )]
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn terminate_instances_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=TerminateInstances&Version=2016-11-15&InstanceId.1=i-1",
            ),
            xml_response(
                200,
                "<TerminateInstancesResponse><instancesSet><item><instanceId>i-1</instanceId>\
                 <currentState><code>32</code><name>shutting-down</name></currentState>\
                 <previousState><code>16</code><name>running</name></previousState>\
                 </item></instancesSet></TerminateInstancesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let changes = client
            .terminate_instances(vec!["i-1".to_string()])
            .await
            .unwrap();

        assert_eq!(
            changes,
            vec![(
                "i-1".to_string(),
                InstanceStateName::Running,
                InstanceStateName::ShuttingDown
            )]
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_route_tables_happy_path_with_ids_and_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeRouteTables&Version=2016-11-15&RouteTableId.1=rtb-1&\
                 Filter.1.Name=vpc-id&Filter.1.Value.1=vpc-1",
            ),
            xml_response(
                200,
                "<DescribeRouteTablesResponse><routeTableSet>\
                 <item><routeTableId>rtb-1</routeTableId><vpcId>vpc-1</vpcId></item>\
                 </routeTableSet></DescribeRouteTablesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (route_tables, token) = client
            .describe_route_tables(
                Some(vec!["rtb-1".to_string()]),
                Some("vpc-1".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(route_tables.len(), 1);
        assert_eq!(route_tables[0].route_table_id(), Some("rtb-1"));
        assert_eq!(route_tables[0].vpc_id(), Some("vpc-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_route_tables_stops_at_limit_with_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeRouteTables&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeRouteTablesResponse><routeTableSet><item><routeTableId>rtb-a</routeTableId></item></routeTableSet>\
                 <nextToken>p2</nextToken></DescribeRouteTablesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (route_tables, token) = client
            .describe_route_tables(None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(route_tables.len(), 1);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_route_tables_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeRouteTables&Version=2016-11-15"),
            ec2_error_response("InvalidRouteTableID.NotFound", "no such route table"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_route_tables(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidRouteTableID.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_network_acls_happy_path_with_ids_and_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeNetworkAcls&Version=2016-11-15&NetworkAclId.1=acl-1&\
                 Filter.1.Name=vpc-id&Filter.1.Value.1=vpc-1",
            ),
            xml_response(
                200,
                "<DescribeNetworkAclsResponse><networkAclSet>\
                 <item><networkAclId>acl-1</networkAclId><vpcId>vpc-1</vpcId></item>\
                 </networkAclSet></DescribeNetworkAclsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (acls, token) = client
            .describe_network_acls(
                Some(vec!["acl-1".to_string()]),
                Some("vpc-1".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(acls.len(), 1);
        assert_eq!(acls[0].network_acl_id(), Some("acl-1"));
        assert_eq!(acls[0].vpc_id(), Some("vpc-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_network_acls_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeNetworkAcls&Version=2016-11-15"),
            ec2_error_response("InvalidNetworkAclID.NotFound", "no such network acl"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_network_acls(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidNetworkAclID.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_internet_gateways_happy_path_with_ids_and_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeInternetGateways&Version=2016-11-15&InternetGatewayId.1=igw-1&\
                 Filter.1.Name=attachment.vpc-id&Filter.1.Value.1=vpc-1",
            ),
            xml_response(
                200,
                "<DescribeInternetGatewaysResponse><internetGatewaySet>\
                 <item><internetGatewayId>igw-1</internetGatewayId></item>\
                 </internetGatewaySet></DescribeInternetGatewaysResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (gateways, token) = client
            .describe_internet_gateways(
                Some(vec!["igw-1".to_string()]),
                Some("vpc-1".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(gateways.len(), 1);
        assert_eq!(gateways[0].internet_gateway_id(), Some("igw-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_internet_gateways_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeInternetGateways&Version=2016-11-15",
            ),
            ec2_error_response(
                "InvalidInternetGatewayID.NotFound",
                "no such internet gateway",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_internet_gateways(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidInternetGatewayID.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_nat_gateways_happy_path_with_ids_and_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeNatGateways&Version=2016-11-15&\
                 Filter.1.Name=vpc-id&Filter.1.Value.1=vpc-1&\
                 Filter.2.Name=state&Filter.2.Value.1=available&\
                 NatGatewayId.1=nat-1",
            ),
            xml_response(
                200,
                "<DescribeNatGatewaysResponse><natGatewaySet>\
                 <item><natGatewayId>nat-1</natGatewayId><vpcId>vpc-1</vpcId></item>\
                 </natGatewaySet></DescribeNatGatewaysResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (gateways, token) = client
            .describe_nat_gateways(
                Some(vec!["nat-1".to_string()]),
                Some("vpc-1".to_string()),
                Some("available".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(gateways.len(), 1);
        assert_eq!(gateways[0].nat_gateway_id(), Some("nat-1"));
        assert_eq!(gateways[0].vpc_id(), Some("vpc-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_nat_gateways_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeNatGateways&Version=2016-11-15"),
            ec2_error_response("NatGatewayNotFound", "no such nat gateway"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_nat_gateways(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("NatGatewayNotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_vpc_endpoints_happy_path_with_ids_and_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeVpcEndpoints&Version=2016-11-15&VpcEndpointId.1=vpce-1&\
                 Filter.1.Name=vpc-id&Filter.1.Value.1=vpc-1&\
                 Filter.2.Name=service-name&Filter.2.Value.1=com.amazonaws.s3",
            ),
            xml_response(
                200,
                "<DescribeVpcEndpointsResponse><vpcEndpointSet>\
                 <item><vpcEndpointId>vpce-1</vpcEndpointId><vpcId>vpc-1</vpcId></item>\
                 </vpcEndpointSet></DescribeVpcEndpointsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (endpoints, token) = client
            .describe_vpc_endpoints(
                Some(vec!["vpce-1".to_string()]),
                Some("vpc-1".to_string()),
                Some("com.amazonaws.s3".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].vpc_endpoint_id(), Some("vpce-1"));
        assert_eq!(endpoints[0].vpc_id(), Some("vpc-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_vpc_endpoints_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeVpcEndpoints&Version=2016-11-15"),
            ec2_error_response("InvalidVpcEndpointId.NotFound", "no such vpc endpoint"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_vpc_endpoints(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidVpcEndpointId.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_transit_gateways_happy_path_with_ids() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeTransitGateways&Version=2016-11-15&TransitGatewayIds.1=tgw-1",
            ),
            xml_response(
                200,
                "<DescribeTransitGatewaysResponse><transitGatewaySet>\
                 <item><transitGatewayId>tgw-1</transitGatewayId></item>\
                 </transitGatewaySet></DescribeTransitGatewaysResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (gateways, token) = client
            .describe_transit_gateways(Some(vec!["tgw-1".to_string()]), None, None)
            .await
            .unwrap();

        assert_eq!(gateways.len(), 1);
        assert_eq!(gateways[0].transit_gateway_id(), Some("tgw-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_transit_gateways_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeTransitGateways&Version=2016-11-15",
            ),
            ec2_error_response(
                "InvalidTransitGatewayID.NotFound",
                "no such transit gateway",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_transit_gateways(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidTransitGatewayID.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_launch_templates_happy_path_with_ids_and_names() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLaunchTemplates&Version=2016-11-15&LaunchTemplateId.1=lt-1&\
                 LaunchTemplateName.1=my-template",
            ),
            xml_response(
                200,
                "<DescribeLaunchTemplatesResponse><launchTemplates>\
                 <item><launchTemplateId>lt-1</launchTemplateId><launchTemplateName>my-template</launchTemplateName></item>\
                 </launchTemplates></DescribeLaunchTemplatesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (templates, token) = client
            .describe_launch_templates(
                Some(vec!["lt-1".to_string()]),
                Some(vec!["my-template".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].launch_template_id(), Some("lt-1"));
        assert_eq!(templates[0].launch_template_name(), Some("my-template"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_launch_templates_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLaunchTemplates&Version=2016-11-15",
            ),
            ec2_error_response(
                "InvalidLaunchTemplateId.NotFound",
                "no such launch template",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_launch_templates(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidLaunchTemplateId.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_launch_template_versions_happy_path_with_versions() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLaunchTemplateVersions&Version=2016-11-15&LaunchTemplateId=lt-1&\
                 LaunchTemplateVersion.1=1&LaunchTemplateVersion.2=2",
            ),
            xml_response(
                200,
                "<DescribeLaunchTemplateVersionsResponse><launchTemplateVersionSet>\
                 <item><launchTemplateId>lt-1</launchTemplateId><versionNumber>1</versionNumber></item>\
                 </launchTemplateVersionSet></DescribeLaunchTemplateVersionsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (versions, token) = client
            .describe_launch_template_versions(
                "lt-1".to_string(),
                Some(vec!["1".to_string(), "2".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].launch_template_id(), Some("lt-1"));
        assert_eq!(versions[0].version_number(), Some(1));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_launch_template_versions_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLaunchTemplateVersions&Version=2016-11-15&LaunchTemplateId=lt-1",
            ),
            ec2_error_response("InvalidLaunchTemplateId.VersionNotFound", "no such version"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_launch_template_versions("lt-1".to_string(), None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(
                code,
                Some("InvalidLaunchTemplateId.VersionNotFound".to_string())
            ),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_snapshots_happy_path_with_ids_and_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeSnapshots&Version=2016-11-15&Owner.1=self&SnapshotId.1=snap-1&\
                 Filter.1.Name=volume-id&Filter.1.Value.1=vol-1&\
                 Filter.2.Name=status&Filter.2.Value.1=completed",
            ),
            xml_response(
                200,
                "<DescribeSnapshotsResponse><snapshotSet>\
                 <item><snapshotId>snap-1</snapshotId></item>\
                 </snapshotSet></DescribeSnapshotsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (snapshots, token) = client
            .describe_snapshots(
                Some(vec!["snap-1".to_string()]),
                Some("vol-1".to_string()),
                Some("completed".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].snapshot_id(), Some("snap-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_snapshots_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeSnapshots&Version=2016-11-15&Owner.1=self",
            ),
            ec2_error_response("InvalidSnapshot.NotFound", "no such snapshot"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_snapshots(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidSnapshot.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_flow_logs_happy_path_with_resource_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeFlowLogs&Version=2016-11-15&\
                 Filter.1.Name=resource-type&Filter.1.Value.1=VPC&\
                 Filter.2.Name=resource-id&Filter.2.Value.1=vpc-1",
            ),
            xml_response(200, "<DescribeFlowLogsResponse><flowLogSet><item><flowLogId>fl-1</flowLogId></item></flowLogSet></DescribeFlowLogsResponse>"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let (flow_logs, token) = client
            .describe_flow_logs(Some("vpc-1".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(flow_logs.len(), 1);
        assert_eq!(flow_logs[0].flow_log_id(), Some("fl-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_flow_logs_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeFlowLogs&Version=2016-11-15&Filter.1.Name=resource-type&Filter.1.Value.1=VPC",
            ),
            ec2_error_response("InvalidFlowLogId.NotFound", "no such flow log"),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_flow_logs(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => {
                assert_eq!(code, Some("InvalidFlowLogId.NotFound".to_string()))
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
