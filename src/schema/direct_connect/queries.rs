use async_graphql::{Context, Object, Result};

use crate::aws::direct_connect::DirectConnectClient;
use crate::schema::direct_connect::types::{DxConnection, DxVirtualInterface};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct DirectConnectQuery;

#[Object]
impl DirectConnectQuery {
    async fn dx_connections(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DxConnection>> {
        let client = ctx.data::<DirectConnectClient>()?;
        let (connections, token) = client.describe_connections(limit, next_token).await?;
        Ok(Page {
            items: connections.into_iter().map(DxConnection::from).collect(),
            next_token: token,
        })
    }

    async fn dx_virtual_interfaces(
        &self,
        ctx: &Context<'_>,
        connection_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DxVirtualInterface>> {
        let client = ctx.data::<DirectConnectClient>()?;
        let (vifs, token) = client
            .describe_virtual_interfaces(connection_id.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: vifs.into_iter().map(DxVirtualInterface::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    const ENDPOINT: &str = "https://directconnect.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn dx_connections_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"connections":[{"connectionId":"dxcon-1","connectionName":"my-dx","connectionState":"available","bandwidth":"1Gbps"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(DirectConnectQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ dxConnections(limit: 1) { items { connectionId connectionName connectionState bandwidth } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["dxConnections"]["items"][0]["connectionId"], "dxcon-1");
        assert_eq!(
            data["dxConnections"]["items"][0]["connectionState"],
            "available"
        );
        assert_eq!(data["dxConnections"]["nextToken"], "page2-token");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn dx_virtual_interfaces_filters_by_connection_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"connectionId":"dxcon-1"}"#),
            json_response(
                200,
                r#"{"virtualInterfaces":[{"virtualInterfaceId":"dxvif-1","connectionId":"dxcon-1","virtualInterfaceType":"private","virtualInterfaceState":"available"}]}"#,
            ),
        )]);
        let client = DirectConnectClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(DirectConnectQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ dxVirtualInterfaces(connectionId: "dxcon-1") { items { virtualInterfaceId connectionId virtualInterfaceType virtualInterfaceState } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data["dxVirtualInterfaces"]["items"][0]["virtualInterfaceId"],
            "dxvif-1"
        );
        assert_eq!(
            data["dxVirtualInterfaces"]["items"][0]["connectionId"],
            "dxcon-1"
        );
        assert_eq!(
            data["dxVirtualInterfaces"]["nextToken"],
            serde_json::Value::Null
        );
        http_client.relaxed_requests_match();
    }
}
