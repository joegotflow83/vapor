use async_graphql::{Context, Object, Result};

use crate::aws::ec2::Ec2Client;
use crate::schema::pagination::Page;
use crate::schema::vpc::types::{InternetGateway, NatGateway, NetworkAcl, RouteTable, TransitGateway, VpcEndpoint, VpcFlowLog};

#[derive(Default)]
pub struct VpcQuery;

#[Object]
impl VpcQuery {
    /// Lists route tables. `limit` caps the total number of results (default
    /// unlimited); pass `nextToken` from a prior page to resume.
    async fn route_tables(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>, limit: Option<i32>, next_token: Option<String>) -> Result<Page<RouteTable>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (results, next_token) = ec2.describe_route_tables(ids, vpc_id, limit, next_token).await?;
        Ok(Page { items: results.into_iter().map(RouteTable::from).collect(), next_token })
    }

    /// Lists network ACLs. `limit` caps the total number of results (default
    /// unlimited); pass `nextToken` from a prior page to resume.
    async fn network_acls(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>, limit: Option<i32>, next_token: Option<String>) -> Result<Page<NetworkAcl>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (results, next_token) = ec2.describe_network_acls(ids, vpc_id, limit, next_token).await?;
        Ok(Page { items: results.into_iter().map(NetworkAcl::from).collect(), next_token })
    }

    /// Lists internet gateways. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn internet_gateways(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>, limit: Option<i32>, next_token: Option<String>) -> Result<Page<InternetGateway>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (results, next_token) = ec2.describe_internet_gateways(ids, vpc_id, limit, next_token).await?;
        Ok(Page { items: results.into_iter().map(InternetGateway::from).collect(), next_token })
    }

    /// Lists NAT gateways. `limit` caps the total number of results (default
    /// unlimited); pass `nextToken` from a prior page to resume.
    async fn nat_gateways(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>, state: Option<String>, limit: Option<i32>, next_token: Option<String>) -> Result<Page<NatGateway>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (results, next_token) = ec2.describe_nat_gateways(ids, vpc_id, state, limit, next_token).await?;
        Ok(Page { items: results.into_iter().map(NatGateway::from).collect(), next_token })
    }

    /// Lists VPC endpoints. `limit` caps the total number of results (default
    /// unlimited); pass `nextToken` from a prior page to resume.
    async fn vpc_endpoints(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>, service_name: Option<String>, limit: Option<i32>, next_token: Option<String>) -> Result<Page<VpcEndpoint>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (results, next_token) = ec2.describe_vpc_endpoints(ids, vpc_id, service_name, limit, next_token).await?;
        Ok(Page { items: results.into_iter().map(VpcEndpoint::from).collect(), next_token })
    }

    /// Lists transit gateways. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn transit_gateways(&self, ctx: &Context<'_>, ids: Option<Vec<String>>, limit: Option<i32>, next_token: Option<String>) -> Result<Page<TransitGateway>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (results, next_token) = ec2.describe_transit_gateways(ids, limit, next_token).await?;
        Ok(Page { items: results.into_iter().map(TransitGateway::from).collect(), next_token })
    }

    /// Lists VPC flow logs. `limit` caps the total number of results (default
    /// unlimited); pass `nextToken` from a prior page to resume.
    async fn vpc_flow_logs(&self, ctx: &Context<'_>, resource_id: Option<String>, limit: Option<i32>, next_token: Option<String>) -> Result<Page<VpcFlowLog>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let (results, next_token) = ec2.describe_flow_logs(resource_id, limit, next_token).await?;
        Ok(Page { items: results.into_iter().map(VpcFlowLog::from).collect(), next_token })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::ec2::Ec2Client;
    use crate::aws::test_util::{request, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::VpcQuery;

    const ENDPOINT: &str = "https://ec2.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn route_tables_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeRouteTables&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeRouteTablesResponse><routeTableSet>\
                 <item><routeTableId>rtb-a</routeTableId><vpcId>vpc-1</vpcId></item>\
                 </routeTableSet><nextToken>p2</nextToken></DescribeRouteTablesResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(VpcQuery).data(client).finish();

        let res = schema.execute(r#"{ routeTables(limit: 1) { items { id vpcId } nextToken } }"#).await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["routeTables"]["items"][0]["id"], "rtb-a");
        assert_eq!(data["routeTables"]["items"][0]["vpcId"], "vpc-1");
        assert_eq!(data["routeTables"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn network_acls_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeNetworkAcls&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeNetworkAclsResponse><networkAclSet>\
                 <item><networkAclId>acl-a</networkAclId><vpcId>vpc-1</vpcId></item>\
                 </networkAclSet><nextToken>p2</nextToken></DescribeNetworkAclsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(VpcQuery).data(client).finish();

        let res = schema.execute(r#"{ networkAcls(limit: 1) { items { id vpcId } nextToken } }"#).await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["networkAcls"]["items"][0]["id"], "acl-a");
        assert_eq!(data["networkAcls"]["items"][0]["vpcId"], "vpc-1");
        assert_eq!(data["networkAcls"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn internet_gateways_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeInternetGateways&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeInternetGatewaysResponse><internetGatewaySet>\
                 <item><internetGatewayId>igw-a</internetGatewayId></item>\
                 </internetGatewaySet><nextToken>p2</nextToken></DescribeInternetGatewaysResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(VpcQuery).data(client).finish();

        let res = schema.execute(r#"{ internetGateways(limit: 1) { items { id } nextToken } }"#).await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["internetGateways"]["items"][0]["id"], "igw-a");
        assert_eq!(data["internetGateways"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn nat_gateways_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeNatGateways&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeNatGatewaysResponse><natGatewaySet>\
                 <item><natGatewayId>nat-a</natGatewayId><vpcId>vpc-1</vpcId></item>\
                 </natGatewaySet><nextToken>p2</nextToken></DescribeNatGatewaysResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(VpcQuery).data(client).finish();

        let res = schema.execute(r#"{ natGateways(limit: 1) { items { id vpcId } nextToken } }"#).await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["natGateways"]["items"][0]["id"], "nat-a");
        assert_eq!(data["natGateways"]["items"][0]["vpcId"], "vpc-1");
        assert_eq!(data["natGateways"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn vpc_endpoints_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeVpcEndpoints&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeVpcEndpointsResponse><vpcEndpointSet>\
                 <item><vpcEndpointId>vpce-a</vpcEndpointId><vpcId>vpc-1</vpcId></item>\
                 </vpcEndpointSet><nextToken>p2</nextToken></DescribeVpcEndpointsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(VpcQuery).data(client).finish();

        let res = schema.execute(r#"{ vpcEndpoints(limit: 1) { items { id vpcId } nextToken } }"#).await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["vpcEndpoints"]["items"][0]["id"], "vpce-a");
        assert_eq!(data["vpcEndpoints"]["items"][0]["vpcId"], "vpc-1");
        assert_eq!(data["vpcEndpoints"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn transit_gateways_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeTransitGateways&Version=2016-11-15&MaxResults=1"),
            xml_response(
                200,
                "<DescribeTransitGatewaysResponse><transitGatewaySet>\
                 <item><transitGatewayId>tgw-a</transitGatewayId></item>\
                 </transitGatewaySet><nextToken>p2</nextToken></DescribeTransitGatewaysResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(VpcQuery).data(client).finish();

        let res = schema.execute(r#"{ transitGateways(limit: 1) { items { id } nextToken } }"#).await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["transitGateways"]["items"][0]["id"], "tgw-a");
        assert_eq!(data["transitGateways"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn vpc_flow_logs_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeFlowLogs&Version=2016-11-15&\
                 Filter.1.Name=resource-type&Filter.1.Value.1=VPC&MaxResults=1",
            ),
            xml_response(
                200,
                "<DescribeFlowLogsResponse><flowLogSet>\
                 <item><flowLogId>fl-a</flowLogId></item>\
                 </flowLogSet><nextToken>p2</nextToken></DescribeFlowLogsResponse>",
            ),
        )]);
        let client = Ec2Client::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(VpcQuery).data(client).finish();

        let res = schema.execute(r#"{ vpcFlowLogs(limit: 1) { items { flowLogId } nextToken } }"#).await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["vpcFlowLogs"]["items"][0]["flowLogId"], "fl-a");
        assert_eq!(data["vpcFlowLogs"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }
}
