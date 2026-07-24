use async_graphql::{Context, Object, Result};

use crate::aws::workspaces::WorkspacesClient;
use crate::schema::pagination::Page;
use crate::schema::workspaces::types::{Workspace, WorkspaceBundle, WorkspaceDirectory};

#[derive(Default)]
pub struct WorkspacesQuery;

#[Object]
impl WorkspacesQuery {
    /// Lists WorkSpaces, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn workspaces(
        &self,
        ctx: &Context<'_>,
        directory_id: Option<String>,
        user_name: Option<String>,
        bundle_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Workspace>> {
        let client = ctx.data::<WorkspacesClient>()?;
        let (items, token) = client
            .describe_workspaces(directory_id, user_name, bundle_id, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(Workspace::from).collect(),
            next_token: token,
        })
    }

    /// Lists WorkSpace directories, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn workspace_directories(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<WorkspaceDirectory>> {
        let client = ctx.data::<WorkspacesClient>()?;
        let (items, token) = client
            .describe_workspace_directories(limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(WorkspaceDirectory::from).collect(),
            next_token: token,
        })
    }

    /// Lists WorkSpace bundles, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn workspace_bundles(
        &self,
        ctx: &Context<'_>,
        owner: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<WorkspaceBundle>> {
        let client = ctx.data::<WorkspacesClient>()?;
        let (items, token) = client
            .describe_workspace_bundles(owner, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(WorkspaceBundle::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::aws::workspaces::WorkspacesClient;
    use crate::schema::test_util::build_query_schema;

    use super::WorkspacesQuery;

    const BASE: &str = "https://workspaces.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn workspaces_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Workspaces":[{"WorkspaceId":"ws-1","DirectoryId":"d-1","UserName":"alice","IpAddress":"10.0.0.1","State":"AVAILABLE","BundleId":"wsb-1","SubnetId":"subnet-1","ComputerName":"COMPUTER1","VolumeEncryptionKey":"key-1","WorkspaceProperties":{"UserVolumeSizeGib":50,"RootVolumeSizeGib":80}},{"WorkspaceId":"ws-2","State":"PENDING"}]}"#,
            ),
        )]);
        let schema = build_query_schema(WorkspacesQuery)
            .data(WorkspacesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ workspaces { items { workspaceId directoryId userName ipAddress state bundleId subnetId computerName volumeEncryptionKey userVolumeSizeGib rootVolumeSizeGib } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["workspaces"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let w1 = &items[0];
        assert_eq!(w1["workspaceId"], "ws-1");
        assert_eq!(w1["directoryId"], "d-1");
        assert_eq!(w1["userName"], "alice");
        assert_eq!(w1["ipAddress"], "10.0.0.1");
        assert_eq!(w1["state"], "AVAILABLE");
        assert_eq!(w1["bundleId"], "wsb-1");
        assert_eq!(w1["subnetId"], "subnet-1");
        assert_eq!(w1["computerName"], "COMPUTER1");
        assert_eq!(w1["volumeEncryptionKey"], "key-1");
        assert_eq!(w1["userVolumeSizeGib"], 50);
        assert_eq!(w1["rootVolumeSizeGib"], 80);

        let w2 = &items[1];
        assert_eq!(w2["workspaceId"], "ws-2");
        assert_eq!(w2["state"], "PENDING");
        assert_eq!(w2["directoryId"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn workspaces_forwards_filter_args() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"DirectoryId":"d-1","UserName":"alice","BundleId":"wsb-1"}"#,
            ),
            json_response(200, r#"{"Workspaces":[]}"#),
        )]);
        let schema = build_query_schema(WorkspacesQuery)
            .data(WorkspacesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ workspaces(directoryId: "d-1", userName: "alice", bundleId: "wsb-1") { items { workspaceId } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["workspaces"]["items"].as_array().unwrap().is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn workspace_directories_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Directories":[{"DirectoryId":"d-1","DirectoryName":"corp.example.com","DirectoryType":"SIMPLE_AD","DnsIpAddresses":["10.0.0.1","10.0.0.2"],"Alias":"alias-1","State":"REGISTERED","WorkspaceCreationProperties":{"EnableInternetAccess":true,"EnableMaintenanceMode":false,"UserEnabledAsLocalAdministrator":true}},{"DirectoryId":"d-2"}]}"#,
            ),
        )]);
        let schema = build_query_schema(WorkspacesQuery)
            .data(WorkspacesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ workspaceDirectories { items { directoryId directoryName directoryType dnsIpAddresses alias state workspaceCreationProperties { enableInternetAccess enableMaintenanceMode userEnabledAsLocalAdministrator } } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["workspaceDirectories"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let d1 = &items[0];
        assert_eq!(d1["directoryId"], "d-1");
        assert_eq!(d1["directoryName"], "corp.example.com");
        assert_eq!(d1["directoryType"], "SIMPLE_AD");
        assert_eq!(
            d1["dnsIpAddresses"],
            serde_json::json!(["10.0.0.1", "10.0.0.2"])
        );
        assert_eq!(d1["alias"], "alias-1");
        assert_eq!(d1["state"], "REGISTERED");
        let props = &d1["workspaceCreationProperties"];
        assert_eq!(props["enableInternetAccess"], true);
        assert_eq!(props["enableMaintenanceMode"], false);
        assert_eq!(props["userEnabledAsLocalAdministrator"], true);

        let d2 = &items[1];
        assert_eq!(d2["directoryId"], "d-2");
        assert_eq!(d2["workspaceCreationProperties"], serde_json::Value::Null);
        assert_eq!(d2["dnsIpAddresses"], serde_json::json!([]));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn workspace_bundles_maps_fields_and_forwards_owner() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Owner":"AMAZON"}"#),
            json_response(
                200,
                r#"{"Bundles":[{"BundleId":"wsb-1","Name":"Standard","Owner":"AMAZON","Description":"desc-1","ImageId":"wsi-1","RootStorage":{"Capacity":"80"},"UserStorage":{"Capacity":"50"},"ComputeType":{"Name":"STANDARD"}},{"BundleId":"wsb-2"}]}"#,
            ),
        )]);
        let schema = build_query_schema(WorkspacesQuery)
            .data(WorkspacesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ workspaceBundles(owner: "AMAZON") { items { bundleId name owner description imageId rootStorage userStorage computeType } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["workspaceBundles"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let b1 = &items[0];
        assert_eq!(b1["bundleId"], "wsb-1");
        assert_eq!(b1["name"], "Standard");
        assert_eq!(b1["owner"], "AMAZON");
        assert_eq!(b1["description"], "desc-1");
        assert_eq!(b1["imageId"], "wsi-1");
        assert_eq!(b1["rootStorage"], "80");
        assert_eq!(b1["userStorage"], "50");
        assert_eq!(b1["computeType"], "STANDARD");

        let b2 = &items[1];
        assert_eq!(b2["bundleId"], "wsb-2");
        assert_eq!(b2["rootStorage"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }
}
