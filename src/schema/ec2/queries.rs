use async_graphql::{Context, Object, Result};

use crate::aws::ec2::Ec2Client;
use crate::schema::ec2::types::{
    ElasticIp, Image, Instance, InstanceState, KeyPair, LaunchTemplate, LaunchTemplateVersion,
    SecurityGroup, Snapshot, Subnet, TagFilter, Volume, Vpc,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct Ec2Query;

#[Object]
impl Ec2Query {
    /// `limit`/`next_token` paginate the returned list.
    #[allow(clippy::too_many_arguments)] // GraphQL resolver args mirror the AWS query
    async fn instances(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        state: Option<InstanceState>,
        vpc_id: Option<String>,
        subnet_id: Option<String>,
        tags: Option<Vec<TagFilter>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Instance>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let state_str = state.map(|s| s.as_aws_str().to_string());

        let tag_filters = tags.map(|ts| {
            ts.into_iter()
                .map(|t| (t.key, vec![t.value]))
                .collect::<Vec<_>>()
        });

        let (aws_instances, next_token) = ec2
            .describe_instances(
                ids,
                state_str,
                vpc_id,
                subnet_id,
                tag_filters,
                limit,
                next_token,
            )
            .await?;

        Ok(Page {
            items: aws_instances.into_iter().map(Instance::from).collect(),
            next_token,
        })
    }

    /// `limit`/`next_token` paginate the returned list.
    async fn security_groups(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SecurityGroup>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let (aws_sgs, next_token) = ec2
            .describe_security_groups(ids, vpc_id, name, limit, next_token)
            .await?;

        Ok(Page {
            items: aws_sgs.into_iter().map(SecurityGroup::from).collect(),
            next_token,
        })
    }

    /// `limit`/`next_token` paginate the returned list.
    async fn vpcs(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Vpc>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let (aws_vpcs, next_token) = ec2.describe_vpcs(ids, limit, next_token).await?;

        Ok(Page {
            items: aws_vpcs.into_iter().map(Vpc::from).collect(),
            next_token,
        })
    }

    /// `limit`/`next_token` paginate the returned list.
    async fn subnets(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        vpc_id: Option<String>,
        az: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Subnet>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let (aws_subnets, next_token) = ec2
            .describe_subnets(ids, vpc_id, az, limit, next_token)
            .await?;

        Ok(Page {
            items: aws_subnets.into_iter().map(Subnet::from).collect(),
            next_token,
        })
    }

    /// `limit`/`next_token` paginate the returned list.
    async fn volumes(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        state: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Volume>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let (aws_volumes, next_token) = ec2.describe_volumes(ids, state, limit, next_token).await?;

        Ok(Page {
            items: aws_volumes.into_iter().map(Volume::from).collect(),
            next_token,
        })
    }

    async fn key_pairs(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        name: Option<String>,
        fingerprint: Option<String>,
    ) -> Result<Vec<KeyPair>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let aws_kps = ec2.describe_key_pairs(ids, name, fingerprint).await?;

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

    /// `limit`/`next_token` paginate the returned list.
    #[allow(clippy::too_many_arguments)] // GraphQL resolver args mirror the AWS query
    async fn images(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        owners: Option<Vec<String>>,
        name: Option<String>,
        state: Option<String>,
        tags: Option<Vec<TagFilter>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Image>> {
        let ec2 = ctx.data::<Ec2Client>()?;

        let tag_filters = tags.map(|ts| {
            ts.into_iter()
                .map(|t| (t.key, vec![t.value]))
                .collect::<Vec<_>>()
        });

        let (aws_images, next_token) = ec2
            .describe_images(ids, owners, name, state, tag_filters, limit, next_token)
            .await?;

        Ok(Page {
            items: aws_images.into_iter().map(Image::from).collect(),
            next_token,
        })
    }

    /// `limit`/`next_token` paginate the returned list.
    async fn launch_templates(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LaunchTemplate>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (items, next_token) = ec2
            .describe_launch_templates(ids, names, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(LaunchTemplate::from).collect(),
            next_token,
        })
    }

    /// `limit`/`next_token` paginate the returned list.
    async fn launch_template_versions(
        &self,
        ctx: &Context<'_>,
        launch_template_id: String,
        versions: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LaunchTemplateVersion>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (items, next_token) = ec2
            .describe_launch_template_versions(launch_template_id, versions, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(LaunchTemplateVersion::from).collect(),
            next_token,
        })
    }

    /// `limit`/`next_token` paginate the returned list.
    async fn snapshots(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        volume_id: Option<String>,
        state: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Snapshot>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (items, next_token) = ec2
            .describe_snapshots(ids, volume_id, state, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(Snapshot::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::ec2::Ec2Client;
    use crate::aws::test_util::{
        request, sdk_config, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    const ENDPOINT: &str = "https://ec2.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn instances_maps_items_and_forwards_limit() {
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
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ instances(limit: 1) { items { id } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["instances"]["items"][0]["id"], "i-1");
        assert_eq!(data["instances"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn security_groups_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeSecurityGroups&Version=2016-11-15&MaxResults=1",
            ),
            xml_response(
                200,
                "<DescribeSecurityGroupsResponse><securityGroupInfo>\
                 <item><groupId>sg-a</groupId><groupName>web-sg</groupName></item>\
                 </securityGroupInfo><nextToken>p2</nextToken></DescribeSecurityGroupsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ securityGroups(limit: 1) { items { id name } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["securityGroups"]["items"][0]["id"], "sg-a");
        assert_eq!(data["securityGroups"]["items"][0]["name"], "web-sg");
        assert_eq!(data["securityGroups"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn vpcs_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeVpcs&Version=2016-11-15&MaxResults=1",
            ),
            xml_response(
                200,
                "<DescribeVpcsResponse><vpcSet><item><vpcId>vpc-a</vpcId>\
                 <cidrBlock>10.0.0.0/16</cidrBlock></item></vpcSet>\
                 <nextToken>p2</nextToken></DescribeVpcsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ vpcs(limit: 1) { items { id cidrBlock } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["vpcs"]["items"][0]["id"], "vpc-a");
        assert_eq!(data["vpcs"]["items"][0]["cidrBlock"], "10.0.0.0/16");
        assert_eq!(data["vpcs"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn subnets_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeSubnets&Version=2016-11-15&MaxResults=1",
            ),
            xml_response(
                200,
                "<DescribeSubnetsResponse><subnetSet><item><subnetId>subnet-a</subnetId>\
                 <vpcId>vpc-a</vpcId></item></subnetSet>\
                 <nextToken>p2</nextToken></DescribeSubnetsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ subnets(limit: 1) { items { id vpcId } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["subnets"]["items"][0]["id"], "subnet-a");
        assert_eq!(data["subnets"]["items"][0]["vpcId"], "vpc-a");
        assert_eq!(data["subnets"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn volumes_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeVolumes&Version=2016-11-15&MaxResults=1",
            ),
            xml_response(
                200,
                "<DescribeVolumesResponse><volumeSet><item><volumeId>vol-a</volumeId>\
                 <status>in-use</status></item></volumeSet>\
                 <nextToken>p2</nextToken></DescribeVolumesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ volumes(limit: 1) { items { id state } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["volumes"]["items"][0]["id"], "vol-a");
        assert_eq!(data["volumes"]["items"][0]["state"], "in-use");
        assert_eq!(data["volumes"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn key_pairs_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeKeyPairs&Version=2016-11-15"),
            xml_response(
                200,
                "<DescribeKeyPairsResponse><keySet>\
                 <item><keyPairId>key-1</keyPairId><keyName>mykey</keyName></item>\
                 </keySet></DescribeKeyPairsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema.execute(r#"{ keyPairs { keyPairId name } }"#).await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["keyPairs"][0]["keyPairId"], "key-1");
        assert_eq!(data["keyPairs"][0]["name"], "mykey");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn elastic_ips_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeAddresses&Version=2016-11-15"),
            xml_response(
                200,
                "<DescribeAddressesResponse><addressesSet>\
                 <item><allocationId>eipalloc-1</allocationId><publicIp>1.2.3.4</publicIp></item>\
                 </addressesSet></DescribeAddressesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ elasticIps { allocationId publicIp } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["elasticIps"][0]["allocationId"], "eipalloc-1");
        assert_eq!(data["elasticIps"][0]["publicIp"], "1.2.3.4");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn images_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeImages&Version=2016-11-15&Owner.1=self&MaxResults=1",
            ),
            xml_response(
                200,
                "<DescribeImagesResponse><imagesSet><item><imageId>ami-a</imageId>\
                 <imageState>available</imageState></item></imagesSet>\
                 <nextToken>p2</nextToken></DescribeImagesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ images(limit: 1) { items { id state } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["images"]["items"][0]["id"], "ami-a");
        assert_eq!(data["images"]["items"][0]["state"], "available");
        assert_eq!(data["images"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn launch_templates_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLaunchTemplates&Version=2016-11-15&MaxResults=1",
            ),
            xml_response(
                200,
                "<DescribeLaunchTemplatesResponse><launchTemplates>\
                 <item><launchTemplateId>lt-1</launchTemplateId>\
                 <launchTemplateName>my-template</launchTemplateName></item>\
                 </launchTemplates><nextToken>p2</nextToken></DescribeLaunchTemplatesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ launchTemplates(limit: 1) { items { id name } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["launchTemplates"]["items"][0]["id"], "lt-1");
        assert_eq!(data["launchTemplates"]["items"][0]["name"], "my-template");
        assert_eq!(data["launchTemplates"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn launch_template_versions_maps_items_and_forwards_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLaunchTemplateVersions&Version=2016-11-15&LaunchTemplateId=lt-1",
            ),
            xml_response(
                200,
                "<DescribeLaunchTemplateVersionsResponse><launchTemplateVersionSet>\
                 <item><launchTemplateId>lt-1</launchTemplateId><versionNumber>1</versionNumber></item>\
                 </launchTemplateVersionSet></DescribeLaunchTemplateVersionsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(
                r#"{ launchTemplateVersions(launchTemplateId: "lt-1") { items { launchTemplateId versionNumber } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data["launchTemplateVersions"]["items"][0]["launchTemplateId"],
            "lt-1"
        );
        assert_eq!(
            data["launchTemplateVersions"]["items"][0]["versionNumber"],
            1
        );
        assert_eq!(
            data["launchTemplateVersions"]["nextToken"],
            serde_json::Value::Null
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn snapshots_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeSnapshots&Version=2016-11-15&MaxResults=1&Owner.1=self",
            ),
            xml_response(
                200,
                "<DescribeSnapshotsResponse><snapshotSet><item><snapshotId>snap-a</snapshotId>\
                 <status>completed</status></item></snapshotSet>\
                 <nextToken>p2</nextToken></DescribeSnapshotsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(Ec2Query).data(client).finish();

        let res = schema
            .execute(r#"{ snapshots(limit: 1) { items { id state } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["snapshots"]["items"][0]["id"], "snap-a");
        assert_eq!(data["snapshots"]["items"][0]["state"], "completed");
        assert_eq!(data["snapshots"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }
}
