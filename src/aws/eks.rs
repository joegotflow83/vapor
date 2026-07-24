#[cfg(feature = "eks")]
use aws_config::SdkConfig;

#[cfg(feature = "eks")]
use crate::error::VaporError;

#[cfg(feature = "eks")]
pub struct EksClient {
    inner: aws_sdk_eks::Client,
}

impl EksClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_eks::Client::new(config),
        }
    }

    /// Lists EKS cluster names, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListClusters` has both
    /// `max_results` and `next_token` (verified against pinned `aws-sdk-eks`
    /// 1.136.0's `operation/list_clusters/_list_clusters_input.rs`) — dropped
    /// `.into_paginator()` in favor of a caller-driven `loop{}` so the token
    /// round-trips correctly (kinesis/translate precedent).
    pub async fn list_clusters(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_clusters();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - names.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            names.extend(output.clusters.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if names.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((names, token))
    }

    /// Describe a single EKS cluster by name. Returns None if not found.
    pub async fn describe_cluster(
        &self,
        name: &str,
    ) -> Result<Option<aws_sdk_eks::types::Cluster>, VaporError> {
        let result = self.inner.describe_cluster().name(name).send().await;
        match result {
            Ok(output) => Ok(output.cluster().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_resource_not_found_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }

    /// Lists nodegroup names for a cluster, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListNodegroups` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-eks` 1.136.0), same pattern as `list_clusters`.
    pub async fn list_nodegroups(
        &self,
        cluster: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_nodegroups().cluster_name(cluster);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - names.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            names.extend(output.nodegroups.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if names.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((names, token))
    }

    /// Describe a single nodegroup. Returns None if not found.
    pub async fn describe_nodegroup(
        &self,
        cluster: &str,
        nodegroup: &str,
    ) -> Result<Option<aws_sdk_eks::types::Nodegroup>, VaporError> {
        let result = self
            .inner
            .describe_nodegroup()
            .cluster_name(cluster)
            .nodegroup_name(nodegroup)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.nodegroup().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_resource_not_found_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }

    /// Lists Fargate profile names for a cluster, optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `ListFargateProfiles` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-eks` 1.136.0), same pattern as
    /// `list_clusters`.
    pub async fn list_fargate_profiles(
        &self,
        cluster: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_fargate_profiles().cluster_name(cluster);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - names.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            names.extend(output.fargate_profile_names.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if names.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((names, token))
    }

    /// Describe a single Fargate profile. Returns None if not found.
    pub async fn describe_fargate_profile(
        &self,
        cluster: &str,
        profile: &str,
    ) -> Result<Option<aws_sdk_eks::types::FargateProfile>, VaporError> {
        let result = self
            .inner
            .describe_fargate_profile()
            .cluster_name(cluster)
            .fargate_profile_name(profile)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.fargate_profile().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_resource_not_found_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }

    /// Lists addon names for a cluster, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListAddons` has
    /// both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-eks` 1.136.0), same pattern as `list_clusters`.
    pub async fn list_addons(
        &self,
        cluster: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_addons().cluster_name(cluster);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - names.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            names.extend(output.addons.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if names.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((names, token))
    }

    /// Describe a single addon. Returns None if not found.
    pub async fn describe_addon(
        &self,
        cluster: &str,
        addon: &str,
    ) -> Result<Option<aws_sdk_eks::types::Addon>, VaporError> {
        let result = self
            .inner
            .describe_addon()
            .cluster_name(cluster)
            .addon_name(addon)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.addon().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_resource_not_found_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const CLUSTERS: &str = "https://eks.us-east-1.amazonaws.com/clusters";
    const CLUSTER: &str = "https://eks.us-east-1.amazonaws.com/clusters/demo";
    const NODEGROUPS: &str = "https://eks.us-east-1.amazonaws.com/clusters/demo/node-groups";
    const NODEGROUP: &str = "https://eks.us-east-1.amazonaws.com/clusters/demo/node-groups/ng-1";
    const FARGATE_PROFILES: &str =
        "https://eks.us-east-1.amazonaws.com/clusters/demo/fargate-profiles";
    const FARGATE_PROFILE: &str =
        "https://eks.us-east-1.amazonaws.com/clusters/demo/fargate-profiles/fp-1";
    const ADDONS: &str = "https://eks.us-east-1.amazonaws.com/clusters/demo/addons";
    const ADDON: &str = "https://eks.us-east-1.amazonaws.com/clusters/demo/addons/vpc-cni";

    // --- list_clusters ---

    #[tokio::test]
    async fn list_clusters_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(CLUSTERS, ""),
            json_response(200, r#"{"clusters":["c1","c2"]}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_clusters(None, None).await.unwrap();

        assert_eq!(names, vec!["c1".to_string(), "c2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{CLUSTERS}?nextToken=cursor-a"), ""),
            json_response(200, r#"{"clusters":["c3"]}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_clusters(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(names, vec!["c3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{CLUSTERS}?maxResults=2"), ""),
            json_response(200, r#"{"clusters":["c1","c2"],"nextToken":"page2"}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_clusters(Some(2), None).await.unwrap();

        assert_eq!(names, vec!["c1".to_string(), "c2".to_string()]);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{CLUSTERS}?maxResults=10"), ""),
                json_response(200, r#"{"clusters":["c1","c2"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{CLUSTERS}?maxResults=8&nextToken=page2"), ""),
                json_response(200, r#"{"clusters":["c3"]}"#),
            ),
        ]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_clusters(Some(10), None).await.unwrap();

        assert_eq!(
            names,
            vec!["c1".to_string(), "c2".to_string(), "c3".to_string()]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_propagates_errors() {
        // `ClientException` rather than a throttling-classified code (e.g.
        // `TooManyRequestsException`) — those are on the SDK's built-in
        // retry-classifier denylist and would consume a second (nonexistent)
        // replay event instead of exercising `sdk_err`'s mapping path.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(CLUSTERS, ""),
            json_error_response("ClientException", "invalid request"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let err = client.list_clusters(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ClientException".to_string()));
                assert_eq!(message, "invalid request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- describe_cluster ---

    #[tokio::test]
    async fn describe_cluster_returns_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(CLUSTER, ""),
            json_response(
                200,
                r#"{"cluster":{"name":"demo","status":"ACTIVE","version":"1.29"}}"#,
            ),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let cluster = client.describe_cluster("demo").await.unwrap().unwrap();

        assert_eq!(cluster.name(), Some("demo"));
        assert_eq!(cluster.version(), Some("1.29"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(CLUSTER, ""),
            json_error_response("ResourceNotFoundException", "cluster not found"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let cluster = client.describe_cluster("demo").await.unwrap();

        assert_eq!(cluster, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(CLUSTER, ""),
            json_error_response("ServerException", "internal failure"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_cluster("demo").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ServerException".to_string()));
                assert_eq!(message, "internal failure");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- list_nodegroups ---

    #[tokio::test]
    async fn list_nodegroups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(NODEGROUPS, ""),
            json_response(200, r#"{"nodegroups":["ng-1","ng-2"]}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_nodegroups("demo", None, None).await.unwrap();

        assert_eq!(names, vec!["ng-1".to_string(), "ng-2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodegroups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{NODEGROUPS}?nextToken=cursor-a"), ""),
            json_response(200, r#"{"nodegroups":["ng-3"]}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_nodegroups("demo", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(names, vec!["ng-3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodegroups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{NODEGROUPS}?maxResults=1"), ""),
            json_response(200, r#"{"nodegroups":["ng-1"],"nextToken":"page2"}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_nodegroups("demo", Some(1), None).await.unwrap();

        assert_eq!(names, vec!["ng-1".to_string()]);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodegroups_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{NODEGROUPS}?maxResults=10"), ""),
                json_response(200, r#"{"nodegroups":["ng-1","ng-2"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{NODEGROUPS}?maxResults=8&nextToken=page2"), ""),
                json_response(200, r#"{"nodegroups":["ng-3"]}"#),
            ),
        ]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_nodegroups("demo", Some(10), None)
            .await
            .unwrap();

        assert_eq!(
            names,
            vec!["ng-1".to_string(), "ng-2".to_string(), "ng-3".to_string()]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodegroups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(NODEGROUPS, ""),
            json_error_response("ResourceNotFoundException", "cluster not found"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_nodegroups("demo", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "cluster not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- describe_nodegroup ---

    #[tokio::test]
    async fn describe_nodegroup_returns_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(NODEGROUP, ""),
            json_response(
                200,
                r#"{"nodegroup":{"nodegroupName":"ng-1","clusterName":"demo","status":"ACTIVE"}}"#,
            ),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let nodegroup = client
            .describe_nodegroup("demo", "ng-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(nodegroup.nodegroup_name(), Some("ng-1"));
        assert_eq!(nodegroup.cluster_name(), Some("demo"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_nodegroup_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(NODEGROUP, ""),
            json_error_response("ResourceNotFoundException", "nodegroup not found"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let nodegroup = client.describe_nodegroup("demo", "ng-1").await.unwrap();

        assert_eq!(nodegroup, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_nodegroup_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(NODEGROUP, ""),
            json_error_response("InvalidParameterException", "bad parameter"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_nodegroup("demo", "ng-1").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- list_fargate_profiles ---

    #[tokio::test]
    async fn list_fargate_profiles_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FARGATE_PROFILES, ""),
            json_response(200, r#"{"fargateProfileNames":["fp-1","fp-2"]}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_fargate_profiles("demo", None, None)
            .await
            .unwrap();

        assert_eq!(names, vec!["fp-1".to_string(), "fp-2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_fargate_profiles_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FARGATE_PROFILES}?nextToken=cursor-a"), ""),
            json_response(200, r#"{"fargateProfileNames":["fp-3"]}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_fargate_profiles("demo", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(names, vec!["fp-3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_fargate_profiles_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FARGATE_PROFILES}?maxResults=1"), ""),
            json_response(
                200,
                r#"{"fargateProfileNames":["fp-1"],"nextToken":"page2"}"#,
            ),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_fargate_profiles("demo", Some(1), None)
            .await
            .unwrap();

        assert_eq!(names, vec!["fp-1".to_string()]);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_fargate_profiles_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{FARGATE_PROFILES}?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"fargateProfileNames":["fp-1","fp-2"],"nextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{FARGATE_PROFILES}?maxResults=8&nextToken=page2"),
                    "",
                ),
                json_response(200, r#"{"fargateProfileNames":["fp-3"]}"#),
            ),
        ]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_fargate_profiles("demo", Some(10), None)
            .await
            .unwrap();

        assert_eq!(
            names,
            vec!["fp-1".to_string(), "fp-2".to_string(), "fp-3".to_string()]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_fargate_profiles_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FARGATE_PROFILES, ""),
            json_error_response("ResourceNotFoundException", "cluster not found"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_fargate_profiles("demo", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "cluster not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- describe_fargate_profile ---

    #[tokio::test]
    async fn describe_fargate_profile_returns_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FARGATE_PROFILE, ""),
            json_response(
                200,
                r#"{"fargateProfile":{"fargateProfileName":"fp-1","clusterName":"demo","status":"ACTIVE"}}"#,
            ),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let profile = client
            .describe_fargate_profile("demo", "fp-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(profile.fargate_profile_name(), Some("fp-1"));
        assert_eq!(profile.cluster_name(), Some("demo"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_fargate_profile_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FARGATE_PROFILE, ""),
            json_error_response("ResourceNotFoundException", "fargate profile not found"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let profile = client
            .describe_fargate_profile("demo", "fp-1")
            .await
            .unwrap();

        assert_eq!(profile, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_fargate_profile_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FARGATE_PROFILE, ""),
            json_error_response("ClientException", "invalid request"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_fargate_profile("demo", "fp-1")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ClientException".to_string()));
                assert_eq!(message, "invalid request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- list_addons ---

    #[tokio::test]
    async fn list_addons_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ADDONS, ""),
            json_response(200, r#"{"addons":["vpc-cni","coredns"]}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_addons("demo", None, None).await.unwrap();

        assert_eq!(names, vec!["vpc-cni".to_string(), "coredns".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_addons_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{ADDONS}?nextToken=cursor-a"), ""),
            json_response(200, r#"{"addons":["kube-proxy"]}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_addons("demo", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(names, vec!["kube-proxy".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_addons_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{ADDONS}?maxResults=1"), ""),
            json_response(200, r#"{"addons":["vpc-cni"],"nextToken":"page2"}"#),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_addons("demo", Some(1), None).await.unwrap();

        assert_eq!(names, vec!["vpc-cni".to_string()]);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_addons_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{ADDONS}?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"addons":["vpc-cni","coredns"],"nextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{ADDONS}?maxResults=8&nextToken=page2"), ""),
                json_response(200, r#"{"addons":["kube-proxy"]}"#),
            ),
        ]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_addons("demo", Some(10), None).await.unwrap();

        assert_eq!(
            names,
            vec![
                "vpc-cni".to_string(),
                "coredns".to_string(),
                "kube-proxy".to_string()
            ]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_addons_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ADDONS, ""),
            json_error_response("ResourceNotFoundException", "cluster not found"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let err = client.list_addons("demo", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "cluster not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- describe_addon ---

    #[tokio::test]
    async fn describe_addon_returns_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ADDON, ""),
            json_response(
                200,
                r#"{"addon":{"addonName":"vpc-cni","clusterName":"demo","status":"ACTIVE","addonVersion":"v1.18.0"}}"#,
            ),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let addon = client
            .describe_addon("demo", "vpc-cni")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(addon.addon_name(), Some("vpc-cni"));
        assert_eq!(addon.addon_version(), Some("v1.18.0"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_addon_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ADDON, ""),
            json_error_response("ResourceNotFoundException", "addon not found"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let addon = client.describe_addon("demo", "vpc-cni").await.unwrap();

        assert_eq!(addon, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_addon_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ADDON, ""),
            json_error_response("InvalidRequestException", "invalid request"),
        )]);
        let client = EksClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_addon("demo", "vpc-cni").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "invalid request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
