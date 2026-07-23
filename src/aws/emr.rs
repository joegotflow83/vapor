use aws_config::SdkConfig;
use aws_sdk_emr::types::{Cluster, ClusterState, ClusterSummary, StepSummary};

use crate::aws::pagination::apply_limit;
use crate::error::VaporError;

pub struct EmrClient {
    inner: aws_sdk_emr::Client,
}

impl EmrClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_emr::Client::new(config),
        }
    }

    /// Lists clusters, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListClustersInput` has no
    /// `max_results`-equivalent field at all (verified against pinned
    /// `aws-sdk-emr` 1.114.0's `_list_clusters_input.rs` — only `marker`
    /// exists), same caveat class as `cost_explorer.rs::get_cost_and_usage`/
    /// `polly.rs::describe_voices`: `limit` can only be enforced via
    /// client-side `apply_limit` truncation, so when that trips mid-page the
    /// returned `next_token` is still AWS's *next*-page token, permanently
    /// skipping whatever was truncated off the current page.
    pub async fn list_clusters(
        &self,
        states: Option<Vec<ClusterState>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ClusterSummary>, Option<String>), VaporError> {
        let mut clusters = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_clusters();
            if let Some(ref s) = states {
                req = req.set_cluster_states(Some(s.clone()));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            clusters.extend(output.clusters().to_vec());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if apply_limit(&mut clusters, limit) {
                break;
            }
            if marker.is_none() {
                break;
            }
        }

        Ok((clusters, marker))
    }

    pub async fn describe_cluster(&self, cluster_id: &str) -> Result<Cluster, VaporError> {
        let output = self
            .inner
            .describe_cluster()
            .cluster_id(cluster_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        output
            .cluster
            .ok_or_else(|| VaporError::AwsSdk { code: None, message: format!("No cluster returned for id {cluster_id}") })
    }

    /// Lists steps for a cluster, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListStepsInput`
    /// has no `max_results`-equivalent field at all (verified against
    /// pinned `aws-sdk-emr` 1.114.0's `_list_steps_input.rs` — only
    /// `marker` exists), same caveat class as `list_clusters` above.
    pub async fn list_steps(
        &self,
        cluster_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<StepSummary>, Option<String>), VaporError> {
        let mut steps = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_steps().cluster_id(cluster_id);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            steps.extend(output.steps().to_vec());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if apply_limit(&mut steps, limit) {
                break;
            }
            if marker.is_none() {
                break;
            }
        }

        Ok((steps, marker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::error::VaporError;

    const ENDPOINT: &str = "https://elasticmapreduce.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_clusters_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Clusters":[{"Id":"j-1","Name":"cluster-1","Status":{"State":"WAITING"}},{"Id":"j-2","Name":"cluster-2","Status":{"State":"RUNNING"}}]}"#,
            ),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.list_clusters(None, None, None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].id(), Some("j-1"));
        assert_eq!(clusters[1].id(), Some("j-2"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_passes_states_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterStates":["RUNNING","WAITING"]}"#),
            json_response(
                200,
                r#"{"Clusters":[{"Id":"j-1","Status":{"State":"RUNNING"}}]}"#,
            ),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (clusters, _marker) = client
            .list_clusters(
                Some(vec![ClusterState::Running, ClusterState::Waiting]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].id(), Some("j-1"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Marker":"cursor-a"}"#),
            json_response(200, r#"{"Clusters":[{"Id":"j-3"}]}"#),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client
            .list_clusters(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].id(), Some("j-3"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_stops_at_limit_and_surfaces_resume_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Clusters":[{"Id":"a"},{"Id":"b"},{"Id":"c"}],"Marker":"page2"}"#,
            ),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.list_clusters(None, Some(2), None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].id(), Some("a"));
        assert_eq!(clusters[1].id(), Some("b"));
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"Clusters":[{"Id":"a"}],"Marker":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Marker":"p2"}"#),
                json_response(200, r#"{"Clusters":[{"Id":"b"}]}"#),
            ),
        ]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.list_clusters(None, None, None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].id(), Some("a"));
        assert_eq!(clusters[1].id(), Some("b"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidRequestException", "bad marker"),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let err = client.list_clusters(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidRequestException"));
                assert_eq!(message, "bad marker");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterId":"j-1"}"#),
            json_response(
                200,
                r#"{"Cluster":{"Id":"j-1","Name":"my-cluster","Status":{"State":"RUNNING"}}}"#,
            ),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let cluster = client.describe_cluster("j-1").await.unwrap();

        assert_eq!(cluster.id(), Some("j-1"));
        assert_eq!(cluster.name(), Some("my-cluster"));
        assert_eq!(
            cluster.status().and_then(|s| s.state()),
            Some(&ClusterState::Running)
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_errors_when_cluster_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterId":"j-missing"}"#),
            json_response(200, "{}"),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_cluster("j-missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, None);
                assert!(message.contains("j-missing"));
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterId":"j-1"}"#),
            json_error_response("InvalidRequestException", "no such cluster"),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_cluster("j-1").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidRequestException"));
                assert_eq!(message, "no such cluster");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_steps_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterId":"j-1"}"#),
            json_response(
                200,
                r#"{"Steps":[{"Id":"s-1","Name":"step-1","Status":{"State":"RUNNING"}}]}"#,
            ),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (steps, marker) = client.list_steps("j-1", None, None).await.unwrap();

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id(), Some("s-1"));
        assert_eq!(steps[0].name(), Some("step-1"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_steps_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterId":"j-1","Marker":"cursor-a"}"#),
            json_response(200, r#"{"Steps":[{"Id":"s-2"}]}"#),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (steps, marker) = client
            .list_steps("j-1", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id(), Some("s-2"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_steps_stops_at_limit_and_surfaces_resume_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterId":"j-1"}"#),
            json_response(
                200,
                r#"{"Steps":[{"Id":"a"},{"Id":"b"},{"Id":"c"}],"Marker":"page2"}"#,
            ),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (steps, marker) = client.list_steps("j-1", Some(2), None).await.unwrap();

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].id(), Some("a"));
        assert_eq!(steps[1].id(), Some("b"));
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_steps_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ClusterId":"j-1"}"#),
                json_response(200, r#"{"Steps":[{"Id":"a"}],"Marker":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ClusterId":"j-1","Marker":"p2"}"#),
                json_response(200, r#"{"Steps":[{"Id":"b"}]}"#),
            ),
        ]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let (steps, marker) = client.list_steps("j-1", None, None).await.unwrap();

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].id(), Some("a"));
        assert_eq!(steps[1].id(), Some("b"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_steps_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterId":"j-1"}"#),
            json_error_response("InvalidRequestException", "no such cluster"),
        )]);
        let client = EmrClient::new(&sdk_config(http_client.clone()));

        let err = client.list_steps("j-1", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidRequestException"));
                assert_eq!(message, "no such cluster");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
