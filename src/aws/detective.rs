use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct DetectiveClient {
    inner: aws_sdk_detective::Client,
}

#[derive(Debug)]
pub struct DatasourcePackageInfo {
    pub datasource_package: Option<String>,
    pub ingest_state: Option<String>,
}

impl DetectiveClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_detective::Client::new(config),
        }
    }

    /// Lists Detective behavior graphs the account administers, optionally
    /// capped at `limit` results (default unlimited) and resumed from
    /// `next_token`. `ListGraphs` has no SDK paginator (confirmed: no
    /// `paginator.rs` under `aws-sdk-detective` 1.103.0's generated
    /// `operation/list_graphs/`), so the loop is hand-rolled, but `limit` is
    /// handed to AWS via `ListGraphsInput::max_results` so a capped page
    /// boundary lands exactly on the returned token (kinesis.rs pattern).
    pub async fn list_graphs(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_detective::types::Graph>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_graphs();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.graph_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists member accounts of `graph_arn`, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListMembers` has no SDK paginator (confirmed against the generated
    /// SDK source), same as `list_graphs`.
    pub async fn list_members(
        &self,
        graph_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_detective::types::MemberDetail>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_members().graph_arn(&graph_arn);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.member_details.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists data source packages enabled for `graph_arn`, optionally capped
    /// at `limit` results (default unlimited) and resumed from `next_token`.
    /// `ListDatasourcePackages` has a generated paginator, but it hides the
    /// token, so this is hand-rolled like the other two ops in this file;
    /// `limit` caps via `ListDatasourcePackagesInput::max_results`.
    pub async fn list_datasource_packages(
        &self,
        graph_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DatasourcePackageInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_datasource_packages().graph_arn(&graph_arn);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            if let Some(map) = output.datasource_packages() {
                items.extend(map.iter().map(|(pkg, details)| {
                    DatasourcePackageInfo {
                        datasource_package: Some(pkg.as_str().to_string()),
                        ingest_state: details
                            .datasource_package_ingest_state()
                            .map(|s| s.as_str().to_string()),
                    }
                }));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const LIST_GRAPHS: &str = "https://api.detective.us-east-1.amazonaws.com/graphs/list";
    const LIST_MEMBERS: &str = "https://api.detective.us-east-1.amazonaws.com/graph/members/list";
    const LIST_DATASOURCE_PACKAGES: &str =
        "https://api.detective.us-east-1.amazonaws.com/graph/datasources/list";

    #[tokio::test]
    async fn list_graphs_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_GRAPHS, "{}"),
            json_response(
                200,
                r#"{"GraphList":[{"Arn":"arn:aws:detective:us-east-1:1:graph:abc"}]}"#,
            ),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let (graphs, token) = client.list_graphs(None, None).await.unwrap();

        assert_eq!(graphs.len(), 1);
        assert_eq!(
            graphs[0].arn(),
            Some("arn:aws:detective:us-east-1:1:graph:abc")
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_graphs_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_GRAPHS, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"GraphList":[{"Arn":"arn3"}]}"#),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let (graphs, token) = client
            .list_graphs(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(graphs.len(), 1);
        assert_eq!(graphs[0].arn(), Some("arn3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_graphs_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_GRAPHS, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"GraphList":[{"Arn":"arn1"},{"Arn":"arn2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let (graphs, token) = client.list_graphs(Some(2), None).await.unwrap();

        assert_eq!(graphs.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_graphs_exhausts_all_pages_until_no_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(LIST_GRAPHS, "{}"),
                json_response(200, r#"{"GraphList":[{"Arn":"arn1"}],"NextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(LIST_GRAPHS, r#"{"NextToken":"page2"}"#),
                json_response(200, r#"{"GraphList":[{"Arn":"arn2"}]}"#),
            ),
        ]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let (graphs, token) = client.list_graphs(None, None).await.unwrap();

        assert_eq!(graphs.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_graphs_propagates_service_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_GRAPHS, "{}"),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let err = client.list_graphs(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_members_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_MEMBERS, r#"{"GraphArn":"arn:graph:1"}"#),
            json_response(
                200,
                r#"{"MemberDetails":[{"AccountId":"111111111111","GraphArn":"arn:graph:1"}]}"#,
            ),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let (members, token) = client
            .list_members("arn:graph:1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(members.len(), 1);
        assert_eq!(members[0].account_id(), Some("111111111111"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_members_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_MEMBERS, r#"{"GraphArn":"arn:graph:1","MaxResults":1}"#),
            json_response(
                200,
                r#"{"MemberDetails":[{"AccountId":"222222222222"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let (members, token) = client
            .list_members("arn:graph:1".to_string(), Some(1), None)
            .await
            .unwrap();

        assert_eq!(members.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_datasource_packages_maps_package_states() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_DATASOURCE_PACKAGES, r#"{"GraphArn":"arn:graph:1"}"#),
            json_response(
                200,
                r#"{"DatasourcePackages":{"DETECTIVE_CORE":{"DatasourcePackageIngestState":"STARTED"},"EKS_AUDIT":{"DatasourcePackageIngestState":"DISABLED"}}}"#,
            ),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let (packages, token) = client
            .list_datasource_packages("arn:graph:1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(packages.len(), 2);
        let core = packages
            .iter()
            .find(|p| p.datasource_package.as_deref() == Some("DETECTIVE_CORE"))
            .expect("DETECTIVE_CORE entry present");
        assert_eq!(core.ingest_state.as_deref(), Some("STARTED"));
        let eks = packages
            .iter()
            .find(|p| p.datasource_package.as_deref() == Some("EKS_AUDIT"))
            .expect("EKS_AUDIT entry present");
        assert_eq!(eks.ingest_state.as_deref(), Some("DISABLED"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_datasource_packages_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                LIST_DATASOURCE_PACKAGES,
                r#"{"GraphArn":"arn:graph:1","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"DatasourcePackages":{"DETECTIVE_CORE":{"DatasourcePackageIngestState":"STARTED"}},"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let (packages, token) = client
            .list_datasource_packages("arn:graph:1".to_string(), Some(1), None)
            .await
            .unwrap();

        assert_eq!(packages.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_datasource_packages_propagates_resource_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_DATASOURCE_PACKAGES, r#"{"GraphArn":"arn:missing"}"#),
            json_error_response("ResourceNotFoundException", "graph not found"),
        )]);
        let client = DetectiveClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_datasource_packages("arn:missing".to_string(), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "graph not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
