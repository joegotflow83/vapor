use async_graphql::{Context, Object, Result};

use crate::aws::ec2::Ec2Client;
use crate::schema::vpc::types::{InternetGateway, NatGateway, NetworkAcl, RouteTable, TransitGateway, VpcEndpoint, VpcFlowLog};

#[derive(Default)]
pub struct VpcQuery;

#[Object]
impl VpcQuery {
    async fn route_tables(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>) -> Result<Vec<RouteTable>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let results = ec2.describe_route_tables(ids, vpc_id).await?;
        Ok(results.into_iter().map(RouteTable::from).collect())
    }

    async fn network_acls(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>) -> Result<Vec<NetworkAcl>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let results = ec2.describe_network_acls(ids, vpc_id).await?;
        Ok(results.into_iter().map(NetworkAcl::from).collect())
    }

    async fn internet_gateways(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>) -> Result<Vec<InternetGateway>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let results = ec2.describe_internet_gateways(ids, vpc_id).await?;
        Ok(results.into_iter().map(InternetGateway::from).collect())
    }

    async fn nat_gateways(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>, state: Option<String>) -> Result<Vec<NatGateway>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let results = ec2.describe_nat_gateways(ids, vpc_id, state).await?;
        Ok(results.into_iter().map(NatGateway::from).collect())
    }

    async fn vpc_endpoints(&self, ctx: &Context<'_>, vpc_id: Option<String>, ids: Option<Vec<String>>, service_name: Option<String>) -> Result<Vec<VpcEndpoint>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let results = ec2.describe_vpc_endpoints(ids, vpc_id, service_name).await?;
        Ok(results.into_iter().map(VpcEndpoint::from).collect())
    }

    async fn transit_gateways(&self, ctx: &Context<'_>, ids: Option<Vec<String>>) -> Result<Vec<TransitGateway>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let results = ec2.describe_transit_gateways(ids).await?;
        Ok(results.into_iter().map(TransitGateway::from).collect())
    }

    async fn vpc_flow_logs(&self, ctx: &Context<'_>, resource_id: Option<String>) -> Result<Vec<VpcFlowLog>> {
        let ec2 = ctx.data::<Ec2Client>()?;
        let results = ec2.describe_flow_logs(resource_id).await?;
        Ok(results.into_iter().map(VpcFlowLog::from).collect())
    }
}
