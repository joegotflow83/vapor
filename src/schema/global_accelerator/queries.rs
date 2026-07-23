use async_graphql::{Context, Object, Result};

use crate::aws::global_accelerator::GlobalAcceleratorClient;
use crate::schema::global_accelerator::types::{Accelerator, GaEndpointGroup, GaListener};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct GlobalAcceleratorQuery;

#[Object]
impl GlobalAcceleratorQuery {
    /// Lists accelerators, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn global_accelerators(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Accelerator>> {
        let client = ctx.data::<GlobalAcceleratorClient>()?;
        let (accelerators, token) = client.list_accelerators(limit, next_token).await?;
        Ok(Page {
            items: accelerators.into_iter().map(Accelerator::from).collect(),
            next_token: token,
        })
    }

    /// Lists listeners for an accelerator, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn global_accelerator_listeners(
        &self,
        ctx: &Context<'_>,
        accelerator_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<GaListener>> {
        let client = ctx.data::<GlobalAcceleratorClient>()?;
        let (listeners, token) = client
            .list_listeners(&accelerator_arn, limit, next_token)
            .await?;
        Ok(Page {
            items: listeners.into_iter().map(GaListener::from).collect(),
            next_token: token,
        })
    }

    /// Lists endpoint groups for a listener, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn global_accelerator_endpoint_groups(
        &self,
        ctx: &Context<'_>,
        listener_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<GaEndpointGroup>> {
        let client = ctx.data::<GlobalAcceleratorClient>()?;
        let (groups, token) = client
            .list_endpoint_groups(&listener_arn, limit, next_token)
            .await?;
        Ok(Page {
            items: groups.into_iter().map(GaEndpointGroup::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::global_accelerator::GlobalAcceleratorClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::GlobalAcceleratorQuery;

    const ENDPOINT: &str = "https://globalaccelerator.us-west-2.amazonaws.com/";

    #[tokio::test]
    async fn global_accelerators_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Accelerators":[{"AcceleratorArn":"arn:aws:globalaccelerator::111111111111:accelerator/acc-a","Name":"acc-a-name","Status":"DEPLOYED","Enabled":true,"IpAddressType":"IPV4","IpSets":[{"IpAddresses":["1.2.3.4"]}],"DnsName":"acc-a.awsglobalaccelerator.com","CreatedTime":1700000000}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(GlobalAcceleratorQuery)
            .data(GlobalAcceleratorClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ globalAccelerators(limit: 1) { items { arn name status enabled ipAddressType ipAddresses dnsName createdTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["globalAccelerators"]["items"];
        assert_eq!(
            items[0]["arn"],
            "arn:aws:globalaccelerator::111111111111:accelerator/acc-a"
        );
        assert_eq!(items[0]["name"], "acc-a-name");
        assert_eq!(items[0]["status"], "DEPLOYED");
        assert_eq!(items[0]["enabled"], true);
        assert_eq!(items[0]["ipAddressType"], "IPV4");
        assert_eq!(items[0]["ipAddresses"][0], "1.2.3.4");
        assert_eq!(items[0]["dnsName"], "acc-a.awsglobalaccelerator.com");
        assert!(items[0]["createdTime"].is_string());
        assert_eq!(json["globalAccelerators"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn global_accelerator_listeners_forwards_arn_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"AcceleratorArn":"acc-1","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Listeners":[{"ListenerArn":"arn:listener-a","Protocol":"TCP","PortRanges":[{"FromPort":80,"ToPort":80}]}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(GlobalAcceleratorQuery)
            .data(GlobalAcceleratorClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ globalAcceleratorListeners(acceleratorArn: "acc-1", limit: 1) { items { listenerArn protocol fromPort toPort } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["globalAcceleratorListeners"]["items"];
        assert_eq!(items[0]["listenerArn"], "arn:listener-a");
        assert_eq!(items[0]["protocol"], "TCP");
        assert_eq!(items[0]["fromPort"], 80);
        assert_eq!(items[0]["toPort"], 80);
        assert_eq!(json["globalAcceleratorListeners"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn global_accelerator_endpoint_groups_forwards_arn_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"ListenerArn":"listener-1","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"EndpointGroups":[{"EndpointGroupArn":"arn:group-a","EndpointGroupRegion":"us-east-1","HealthCheckProtocol":"HTTP","TrafficDialPercentage":100.0}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(GlobalAcceleratorQuery)
            .data(GlobalAcceleratorClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ globalAcceleratorEndpointGroups(listenerArn: "listener-1", limit: 1) { items { endpointGroupArn endpointGroupRegion healthCheckProtocol trafficDialPercentage } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["globalAcceleratorEndpointGroups"]["items"];
        assert_eq!(items[0]["endpointGroupArn"], "arn:group-a");
        assert_eq!(items[0]["endpointGroupRegion"], "us-east-1");
        assert_eq!(items[0]["healthCheckProtocol"], "HTTP");
        assert_eq!(items[0]["trafficDialPercentage"], 100.0);
        assert_eq!(json["globalAcceleratorEndpointGroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
