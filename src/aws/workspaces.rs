use aws_config::SdkConfig;

use crate::error::VaporError;
use crate::aws::pagination::apply_limit;

#[derive(Debug)]
pub struct WorkspaceInfo {
    pub workspace_id: Option<String>,
    pub directory_id: Option<String>,
    pub user_name: Option<String>,
    pub ip_address: Option<String>,
    pub state: Option<String>,
    pub bundle_id: Option<String>,
    pub subnet_id: Option<String>,
    pub error_message: Option<String>,
    pub computer_name: Option<String>,
    pub volume_encryption_key: Option<String>,
    pub user_volume_size_gib: Option<i32>,
    pub root_volume_size_gib: Option<i32>,
}

#[derive(Debug)]
pub struct WorkspaceCreationPropsInfo {
    pub enable_internet_access: Option<bool>,
    pub enable_maintenance_mode: Option<bool>,
    pub user_enabled_as_local_administrator: Option<bool>,
}

#[derive(Debug)]
pub struct WorkspaceDirectoryInfo {
    pub directory_id: Option<String>,
    pub directory_name: Option<String>,
    pub directory_type: Option<String>,
    pub dns_ip_addresses: Vec<String>,
    pub alias: Option<String>,
    pub state: Option<String>,
    pub workspace_creation_properties: Option<WorkspaceCreationPropsInfo>,
}

#[derive(Debug)]
pub struct WorkspaceBundleInfo {
    pub bundle_id: Option<String>,
    pub name: Option<String>,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub image_id: Option<String>,
    pub root_storage: Option<String>,
    pub user_storage: Option<String>,
    pub compute_type: Option<String>,
}

pub struct WorkspacesClient {
    inner: aws_sdk_workspaces::Client,
}

impl WorkspacesClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_workspaces::Client::new(config),
        }
    }

    /// Lists WorkSpaces, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeWorkspaces` has
    /// both `limit` and `next_token` (verified against pinned
    /// `aws-sdk-workspaces` 1.124.0's
    /// `operation/describe_workspaces/_describe_workspaces_input.rs`), so
    /// `limit` is handed to AWS on the request itself (kinesis::list_streams
    /// pattern).
    pub async fn describe_workspaces(
        &self,
        directory_id: Option<String>,
        user_name: Option<String>,
        bundle_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<WorkspaceInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_workspaces();
            if let Some(ref d) = directory_id {
                req = req.directory_id(d);
            }
            if let Some(ref u) = user_name {
                req = req.user_name(u);
            }
            if let Some(ref b) = bundle_id {
                req = req.bundle_id(b);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for ws in output.workspaces.unwrap_or_default() {
                items.push(WorkspaceInfo {
                    workspace_id: ws.workspace_id,
                    directory_id: ws.directory_id,
                    user_name: ws.user_name,
                    ip_address: ws.ip_address,
                    state: ws.state.map(|s| s.as_str().to_string()),
                    bundle_id: ws.bundle_id,
                    subnet_id: ws.subnet_id,
                    error_message: ws.error_message,
                    computer_name: ws.computer_name,
                    volume_encryption_key: ws.volume_encryption_key,
                    user_volume_size_gib: ws
                        .workspace_properties
                        .as_ref()
                        .and_then(|p| p.user_volume_size_gib),
                    root_volume_size_gib: ws
                        .workspace_properties
                        .as_ref()
                        .and_then(|p| p.root_volume_size_gib),
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists WorkSpace directories, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `DescribeWorkspaceDirectories` has both `limit` and `next_token`
    /// (verified against pinned `aws-sdk-workspaces` 1.124.0's
    /// `operation/describe_workspace_directories/
    /// _describe_workspace_directories_input.rs`), same shape as
    /// `describe_workspaces` above.
    pub async fn describe_workspace_directories(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<WorkspaceDirectoryInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_workspace_directories();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for dir in output.directories.unwrap_or_default() {
                let creation_props =
                    dir.workspace_creation_properties.map(|p| WorkspaceCreationPropsInfo {
                        enable_internet_access: p.enable_internet_access,
                        enable_maintenance_mode: p.enable_maintenance_mode,
                        user_enabled_as_local_administrator: p.user_enabled_as_local_administrator,
                    });
                items.push(WorkspaceDirectoryInfo {
                    directory_id: dir.directory_id,
                    directory_name: dir.directory_name,
                    directory_type: dir.directory_type.map(|t| t.as_str().to_string()),
                    dns_ip_addresses: dir.dns_ip_addresses.unwrap_or_default(),
                    alias: dir.alias,
                    state: dir.state.map(|s| s.as_str().to_string()),
                    workspace_creation_properties: creation_props,
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists WorkSpace bundles, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeWorkspaceBundles`
    /// has no `max_results`-equivalent input field (only a bare
    /// `next_token`, verified against pinned `aws-sdk-workspaces` 1.124.0's
    /// `operation/describe_workspace_bundles/
    /// _describe_workspace_bundles_input.rs`), so `limit` can only be
    /// enforced via client-side `apply_limit` truncation (xray::get_groups
    /// pattern) — when truncation trips mid-page, the returned `next_token`
    /// is still AWS's *next*-page token, permanently skipping whatever was
    /// truncated off the current page.
    pub async fn describe_workspace_bundles(
        &self,
        owner: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<WorkspaceBundleInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_workspace_bundles();
            if let Some(ref o) = owner {
                req = req.owner(o);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for bundle in output.bundles.unwrap_or_default() {
                items.push(WorkspaceBundleInfo {
                    bundle_id: bundle.bundle_id,
                    name: bundle.name,
                    owner: bundle.owner,
                    description: bundle.description,
                    image_id: bundle.image_id,
                    root_storage: bundle.root_storage.map(|r| r.capacity),
                    user_storage: bundle.user_storage.map(|u| u.capacity),
                    compute_type: bundle
                        .compute_type
                        .and_then(|c| c.name)
                        .map(|n| n.as_str().to_string()),
                });
            }
            token = match output.next_token {
                Some(tok) if !tok.is_empty() => Some(tok),
                _ => None,
            };

            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://workspaces.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_workspaces_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Workspaces":[{"WorkspaceId":"ws-1","DirectoryId":"d-1","UserName":"alice","IpAddress":"10.0.0.1","State":"AVAILABLE","BundleId":"wsb-1","SubnetId":"subnet-1","ComputerName":"COMPUTER1","VolumeEncryptionKey":"key-1","WorkspaceProperties":{"UserVolumeSizeGib":50,"RootVolumeSizeGib":80}},{"WorkspaceId":"ws-2","State":"PENDING"}]}"#,
            ),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspaces(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].workspace_id, Some("ws-1".to_string()));
        assert_eq!(items[0].directory_id, Some("d-1".to_string()));
        assert_eq!(items[0].user_name, Some("alice".to_string()));
        assert_eq!(items[0].ip_address, Some("10.0.0.1".to_string()));
        assert_eq!(items[0].state, Some("AVAILABLE".to_string()));
        assert_eq!(items[0].bundle_id, Some("wsb-1".to_string()));
        assert_eq!(items[0].subnet_id, Some("subnet-1".to_string()));
        assert_eq!(items[0].computer_name, Some("COMPUTER1".to_string()));
        assert_eq!(items[0].volume_encryption_key, Some("key-1".to_string()));
        assert_eq!(items[0].user_volume_size_gib, Some(50));
        assert_eq!(items[0].root_volume_size_gib, Some(80));
        assert_eq!(items[1].workspace_id, Some("ws-2".to_string()));
        assert_eq!(items[1].user_volume_size_gib, None);
        assert_eq!(items[1].root_volume_size_gib, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspaces_passes_filter_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"DirectoryId":"d-1","UserName":"alice","BundleId":"wsb-1"}"#,
            ),
            json_response(200, r#"{"Workspaces":[]}"#),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .describe_workspaces(
                Some("d-1".to_string()),
                Some("alice".to_string()),
                Some("wsb-1".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert!(items.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspaces_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Workspaces":[{"WorkspaceId":"ws-3"}]}"#),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspaces(None, None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspaces_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Limit":2}"#),
            json_response(
                200,
                r#"{"Workspaces":[{"WorkspaceId":"ws-1"},{"WorkspaceId":"ws-2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspaces(None, None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspaces_propagates_errors() {
        // `InvalidParameterValuesException`, not a throttling-classified code
        // (see memory gotcha 1: those get retried and exhaust the single
        // replay event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidParameterValuesException", "bad parameter"),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_workspaces(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValuesException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_directories_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Directories":[{"DirectoryId":"d-1","DirectoryName":"corp.example.com","DirectoryType":"SIMPLE_AD","DnsIpAddresses":["10.0.0.1","10.0.0.2"],"Alias":"alias-1","State":"REGISTERED","WorkspaceCreationProperties":{"EnableInternetAccess":true,"EnableMaintenanceMode":false,"UserEnabledAsLocalAdministrator":true}},{"DirectoryId":"d-2"}]}"#,
            ),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspace_directories(None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].directory_id, Some("d-1".to_string()));
        assert_eq!(items[0].directory_name, Some("corp.example.com".to_string()));
        assert_eq!(items[0].directory_type, Some("SIMPLE_AD".to_string()));
        assert_eq!(
            items[0].dns_ip_addresses,
            vec!["10.0.0.1".to_string(), "10.0.0.2".to_string()]
        );
        assert_eq!(items[0].alias, Some("alias-1".to_string()));
        assert_eq!(items[0].state, Some("REGISTERED".to_string()));
        let props = items[0].workspace_creation_properties.as_ref().unwrap();
        assert_eq!(props.enable_internet_access, Some(true));
        assert_eq!(props.enable_maintenance_mode, Some(false));
        assert_eq!(props.user_enabled_as_local_administrator, Some(true));
        assert_eq!(items[1].directory_id, Some("d-2".to_string()));
        assert!(items[1].dns_ip_addresses.is_empty());
        assert!(items[1].workspace_creation_properties.is_none());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_directories_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-b"}"#),
            json_response(200, r#"{"Directories":[{"DirectoryId":"d-3"}]}"#),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspace_directories(None, Some("cursor-b".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_directories_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Limit":1}"#),
            json_response(
                200,
                r#"{"Directories":[{"DirectoryId":"d-1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspace_directories(Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_directories_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidParameterValuesException", "bad directory filter"),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_workspace_directories(None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValuesException".to_string()));
                assert_eq!(message, "bad directory filter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_bundles_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Bundles":[{"BundleId":"wsb-1","Name":"Standard","Owner":"AMAZON","Description":"desc-1","ImageId":"wsi-1","RootStorage":{"Capacity":"80"},"UserStorage":{"Capacity":"50"},"ComputeType":{"Name":"STANDARD"}},{"BundleId":"wsb-2"}]}"#,
            ),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspace_bundles(None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].bundle_id, Some("wsb-1".to_string()));
        assert_eq!(items[0].name, Some("Standard".to_string()));
        assert_eq!(items[0].owner, Some("AMAZON".to_string()));
        assert_eq!(items[0].description, Some("desc-1".to_string()));
        assert_eq!(items[0].image_id, Some("wsi-1".to_string()));
        assert_eq!(items[0].root_storage, Some("80".to_string()));
        assert_eq!(items[0].user_storage, Some("50".to_string()));
        assert_eq!(items[0].compute_type, Some("STANDARD".to_string()));
        assert_eq!(items[1].bundle_id, Some("wsb-2".to_string()));
        assert_eq!(items[1].root_storage, None);
        assert_eq!(items[1].compute_type, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_bundles_passes_owner_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Owner":"AMAZON"}"#),
            json_response(200, r#"{"Bundles":[]}"#),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .describe_workspace_bundles(Some("AMAZON".to_string()), None, None)
            .await
            .unwrap();

        assert!(items.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_bundles_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-c"}"#),
            json_response(200, r#"{"Bundles":[{"BundleId":"wsb-3"}]}"#),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspace_bundles(None, None, Some("cursor-c".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_bundles_stops_at_limit_truncates_client_side() {
        // `DescribeWorkspaceBundles` has no `Limit`/`MaxResults`-equivalent
        // input field (see the wrapper method's own doc comment), so the
        // request body never carries a limit and truncation happens purely
        // client-side via `apply_limit` — the canned response deliberately
        // returns *more* items than `limit` to exercise that (gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Bundles":[{"BundleId":"wsb-1"},{"BundleId":"wsb-2"},{"BundleId":"wsb-3"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspace_bundles(None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].bundle_id, Some("wsb-1".to_string()));
        assert_eq!(items[1].bundle_id, Some("wsb-2".to_string()));
        // Truncation trips mid-page, so the returned token is still AWS's
        // next-page token — `wsb-3` is permanently skipped, not resumable.
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_bundles_empty_next_token_treated_as_none() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Bundles":[{"BundleId":"wsb-1"}],"NextToken":""}"#,
            ),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_workspace_bundles(None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_workspace_bundles_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidParameterValuesException", "bad bundle id"),
        )]);
        let client = WorkspacesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_workspace_bundles(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValuesException".to_string()));
                assert_eq!(message, "bad bundle id");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
