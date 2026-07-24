use async_graphql::{Context, Object, Result};

use crate::aws::transfer::TransferClient;
use crate::schema::pagination::Page;
use crate::schema::transfer::types::{TransferServer, TransferUser};

#[derive(Default)]
pub struct TransferQuery;

#[Object]
impl TransferQuery {
    /// Lists Transfer Family servers, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn transfer_servers(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TransferServer>> {
        let client = ctx.data::<TransferClient>()?;
        let (servers, token) = client.list_servers(limit, next_token).await?;
        Ok(Page {
            items: servers.into_iter().map(TransferServer::from).collect(),
            next_token: token,
        })
    }

    /// Lists users for a server, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn transfer_users(
        &self,
        ctx: &Context<'_>,
        server_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TransferUser>> {
        let client = ctx.data::<TransferClient>()?;
        let (users, token) = client.list_users(&server_id, limit, next_token).await?;
        Ok(Page {
            items: users
                .into_iter()
                .map(|u| TransferUser::from_listed(u, server_id.clone()))
                .collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::aws::transfer::TransferClient;
    use crate::schema::test_util::build_query_schema;

    use super::TransferQuery;

    const BASE: &str = "https://transfer.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn transfer_servers_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"Servers":[{"Arn":"arn:s1","Domain":"S3","IdentityProviderType":"SERVICE_MANAGED","EndpointType":"PUBLIC","LoggingRole":"arn:role1","ServerId":"s1","State":"ONLINE","UserCount":3},{"Arn":"arn:s2","ServerId":"s2"}]}"#,
            ),
        )]);
        let schema = build_query_schema(TransferQuery)
            .data(TransferClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ transferServers { items { serverId arn state protocols endpointType identityProviderType domain userCount } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let servers = json["transferServers"]["items"].as_array().unwrap();
        assert_eq!(servers.len(), 2);
        let s1 = &servers[0];
        assert_eq!(s1["serverId"], "s1");
        assert_eq!(s1["arn"], "arn:s1");
        assert_eq!(s1["state"], "ONLINE");
        assert_eq!(s1["protocols"], serde_json::json!([]));
        assert_eq!(s1["endpointType"], "PUBLIC");
        assert_eq!(s1["identityProviderType"], "SERVICE_MANAGED");
        assert_eq!(s1["domain"], "S3");
        assert_eq!(s1["userCount"], 3);

        let s2 = &servers[1];
        assert_eq!(s2["serverId"], "s2");
        assert_eq!(s2["state"], serde_json::Value::Null);
        assert_eq!(s2["domain"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn transfer_users_maps_fields_and_forwards_server_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"ServerId":"s-123"}"#),
            json_response(
                200,
                r#"{"ServerId":"s-123","Users":[{"Arn":"arn:u1","HomeDirectory":"/bucket/home","HomeDirectoryType":"PATH","Role":"arn:role1","SshPublicKeyCount":2,"UserName":"alice"},{"Arn":"arn:u2","UserName":"bob"}]}"#,
            ),
        )]);
        let schema = build_query_schema(TransferQuery)
            .data(TransferClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ transferUsers(serverId: "s-123") { items { userName arn serverId role homeDirectory homeDirectoryType } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let users = json["transferUsers"]["items"].as_array().unwrap();
        assert_eq!(users.len(), 2);
        let u1 = &users[0];
        assert_eq!(u1["userName"], "alice");
        assert_eq!(u1["arn"], "arn:u1");
        assert_eq!(u1["serverId"], "s-123");
        assert_eq!(u1["role"], "arn:role1");
        assert_eq!(u1["homeDirectory"], "/bucket/home");
        assert_eq!(u1["homeDirectoryType"], "PATH");

        let u2 = &users[1];
        assert_eq!(u2["userName"], "bob");
        assert_eq!(u2["serverId"], "s-123");
        assert_eq!(u2["role"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }
}
