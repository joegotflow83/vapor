use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct EcsClient {
    inner: aws_sdk_ecs::Client,
}

impl EcsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ecs::Client::new(config),
        }
    }

    /// Describes clusters. If `cluster_arns` is given (caller-supplied IDs),
    /// describes exactly those with no pagination (`limit`/`next_token` are
    /// no-ops, token is always `None`). Otherwise discovers cluster ARNs via
    /// `ListClusters`, paginating that discovery call with `limit`/
    /// `next_token`, before the describe fan-out over the discovered page.
    pub async fn describe_clusters(
        &self,
        cluster_arns: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ecs::types::Cluster>, Option<String>), VaporError> {
        let (arns, token) = match cluster_arns {
            Some(arns) => (arns, None),
            None => {
                let mut all_arns: Vec<String> = Vec::new();
                let mut token = next_token;
                loop {
                    let mut req = self.inner.list_clusters();
                    if let Some(ref t) = token {
                        req = req.next_token(t);
                    }
                    if let Some(l) = limit {
                        req = req.max_results(l - all_arns.len() as i32);
                    }
                    let output = req.send().await.map_err(crate::error::sdk_err)?;
                    token = output.next_token;
                    all_arns.extend(output.cluster_arns.unwrap_or_default());

                    match (&token, limit) {
                        (None, _) => break,
                        (_, Some(l)) if all_arns.len() as i32 >= l => break,
                        _ => continue,
                    }
                }
                (all_arns, token)
            }
        };

        if arns.is_empty() {
            return Ok((Vec::new(), token));
        }

        let mut results: Vec<aws_sdk_ecs::types::Cluster> = Vec::new();
        for chunk in arns.chunks(100) {
            let output = self
                .inner
                .describe_clusters()
                .set_clusters(Some(chunk.to_vec()))
                .include(aws_sdk_ecs::types::ClusterField::Tags)
                .include(aws_sdk_ecs::types::ClusterField::Statistics)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            results.extend(output.clusters().iter().cloned());
        }
        Ok((results, token))
    }

    /// Describes services in a cluster. If `service_arns` is given
    /// (caller-supplied IDs), describes exactly those with no pagination
    /// (`limit`/`next_token` are no-ops, token is always `None`). Otherwise
    /// discovers service ARNs via `ListServices`, paginating that discovery
    /// call with `limit`/`next_token`, before the describe fan-out over the
    /// discovered page.
    pub async fn describe_services(
        &self,
        cluster_arn: &str,
        service_arns: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ecs::types::Service>, Option<String>), VaporError> {
        let (arns, token) = match service_arns {
            Some(arns) => (arns, None),
            None => {
                let mut all_arns: Vec<String> = Vec::new();
                let mut token = next_token;
                loop {
                    let mut req = self.inner.list_services().cluster(cluster_arn);
                    if let Some(ref t) = token {
                        req = req.next_token(t);
                    }
                    if let Some(l) = limit {
                        req = req.max_results(l - all_arns.len() as i32);
                    }
                    let output = req.send().await.map_err(crate::error::sdk_err)?;
                    token = output.next_token;
                    all_arns.extend(output.service_arns.unwrap_or_default());

                    match (&token, limit) {
                        (None, _) => break,
                        (_, Some(l)) if all_arns.len() as i32 >= l => break,
                        _ => continue,
                    }
                }
                (all_arns, token)
            }
        };

        if arns.is_empty() {
            return Ok((Vec::new(), token));
        }

        let mut results: Vec<aws_sdk_ecs::types::Service> = Vec::new();
        for chunk in arns.chunks(10) {
            let output = self
                .inner
                .describe_services()
                .cluster(cluster_arn)
                .set_services(Some(chunk.to_vec()))
                .include(aws_sdk_ecs::types::ServiceField::Tags)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            results.extend(output.services().iter().cloned());
        }
        Ok((results, token))
    }

    /// Describes tasks in a cluster, optionally filtered by service and
    /// desired status. The task-ARN discovery (`ListTasks`) is paginated
    /// with `limit`/`next_token` before the describe fan-out over the
    /// discovered page.
    pub async fn describe_tasks(
        &self,
        cluster_arn: &str,
        service_arn: Option<String>,
        desired_status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ecs::types::Task>, Option<String>), VaporError> {
        let mut all_task_arns: Vec<String> = Vec::new();
        let mut token = next_token;
        loop {
            let mut req = self.inner.list_tasks().cluster(cluster_arn);
            if let Some(ref svc) = service_arn {
                req = req.service_name(svc);
            }
            if let Some(ref status) = desired_status {
                req = req.desired_status(aws_sdk_ecs::types::DesiredStatus::from(status.as_str()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - all_task_arns.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            all_task_arns.extend(output.task_arns.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all_task_arns.len() as i32 >= l => break,
                _ => continue,
            }
        }

        if all_task_arns.is_empty() {
            return Ok((Vec::new(), token));
        }

        let mut results: Vec<aws_sdk_ecs::types::Task> = Vec::new();
        for chunk in all_task_arns.chunks(100) {
            let output = self
                .inner
                .describe_tasks()
                .cluster(cluster_arn)
                .set_tasks(Some(chunk.to_vec()))
                .include(aws_sdk_ecs::types::TaskField::Tags)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            results.extend(output.tasks().iter().cloned());
        }
        Ok((results, token))
    }

    pub async fn describe_task_definition(
        &self,
        task_definition: &str,
    ) -> Result<Option<aws_sdk_ecs::types::TaskDefinition>, VaporError> {
        let result = self
            .inner
            .describe_task_definition()
            .task_definition(task_definition)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.task_definition().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_client_exception() || se.is_invalid_parameter_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }

    /// Lists task definition ARNs, optionally filtered by family prefix and
    /// status, paginated with `limit`/`next_token`.
    pub async fn list_task_definitions(
        &self,
        family_prefix: Option<String>,
        status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut all_arns: Vec<String> = Vec::new();
        let mut token = next_token;
        loop {
            let mut req = self.inner.list_task_definitions();
            if let Some(ref prefix) = family_prefix {
                req = req.family_prefix(prefix);
            }
            if let Some(ref s) = status {
                req = req.status(aws_sdk_ecs::types::TaskDefinitionStatus::from(s.as_str()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - all_arns.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            all_arns.extend(output.task_definition_arns.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if all_arns.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((all_arns, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://ecs.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_clusters_discovers_and_fans_out_with_include() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"clusterArns":["arn:cluster-a","arn:cluster-b"]}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"clusters":["arn:cluster-a","arn:cluster-b"],"include":["TAGS","STATISTICS"]}"#,
                ),
                json_response(
                    200,
                    r#"{"clusters":[{"clusterArn":"arn:cluster-a","clusterName":"a"},{"clusterArn":"arn:cluster-b","clusterName":"b"}]}"#,
                ),
            ),
        ]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (clusters, token) = client.describe_clusters(None, None, None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].cluster_name.as_deref(), Some("a"));
        assert_eq!(clusters[1].cluster_name.as_deref(), Some("b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_paginates_discovery_and_resumes_with_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"maxResults":1}"#),
                json_response(
                    200,
                    r#"{"clusterArns":["arn:cluster-a"],"nextToken":"cursor-a"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"clusters":["arn:cluster-a"],"include":["TAGS","STATISTICS"]}"#,
                ),
                json_response(
                    200,
                    r#"{"clusters":[{"clusterArn":"arn:cluster-a","clusterName":"a"}]}"#,
                ),
            ),
        ]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (clusters, token) = client.describe_clusters(None, Some(1), None).await.unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(token.as_deref(), Some("cursor-a"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_with_explicit_arns_skips_discovery() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"clusters":["arn:cluster-c"],"include":["TAGS","STATISTICS"]}"#,
            ),
            json_response(
                200,
                r#"{"clusters":[{"clusterArn":"arn:cluster-c","clusterName":"c"}]}"#,
            ),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (clusters, token) = client
            .describe_clusters(Some(vec!["arn:cluster-c".to_string()]), None, None)
            .await
            .unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_with_explicit_empty_arns_returns_empty_without_calls() {
        let http_client = StaticReplayClient::new(vec![]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (clusters, token) = client
            .describe_clusters(Some(vec![]), None, None)
            .await
            .unwrap();

        assert!(clusters.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_propagates_discovery_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidParameterException", "bad param"),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_clusters(None, None, None)
            .await
            .unwrap_err();

        assert!(matches!(err, VaporError::AwsSdk { .. }));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_propagates_describe_errors() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"clusterArns":["arn:cluster-a"]}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"clusters":["arn:cluster-a"],"include":["TAGS","STATISTICS"]}"#,
                ),
                json_error_response("ClusterNotFoundException", "no such cluster"),
            ),
        ]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_clusters(None, None, None)
            .await
            .unwrap_err();

        assert!(matches!(err, VaporError::AwsSdk { .. }));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_services_discovers_and_fans_out_with_include() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"cluster":"my-cluster"}"#),
                json_response(200, r#"{"serviceArns":["arn:svc-a"]}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"cluster":"my-cluster","services":["arn:svc-a"],"include":["TAGS"]}"#,
                ),
                json_response(
                    200,
                    r#"{"services":[{"serviceArn":"arn:svc-a","serviceName":"svc-a"}]}"#,
                ),
            ),
        ]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client
            .describe_services("my-cluster", None, None, None)
            .await
            .unwrap();

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].service_name.as_deref(), Some("svc-a"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_services_with_explicit_arns_skips_discovery() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"cluster":"my-cluster","services":["arn:svc-b"],"include":["TAGS"]}"#,
            ),
            json_response(
                200,
                r#"{"services":[{"serviceArn":"arn:svc-b","serviceName":"svc-b"}]}"#,
            ),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client
            .describe_services(
                "my-cluster",
                Some(vec!["arn:svc-b".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(services.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_services_chunks_arns_over_10_for_describe() {
        let arns: Vec<String> = (0..11).map(|i| format!("arn:svc-{i}")).collect();
        let first_chunk = arns[..10]
            .iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(",");
        let second_chunk = format!("\"{}\"", arns[10]);

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    format!(
                        r#"{{"cluster":"my-cluster","services":[{first_chunk}],"include":["TAGS"]}}"#
                    ),
                ),
                json_response(200, r#"{"services":[{"serviceArn":"arn:svc-0"}]}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    format!(
                        r#"{{"cluster":"my-cluster","services":[{second_chunk}],"include":["TAGS"]}}"#
                    ),
                ),
                json_response(200, r#"{"services":[{"serviceArn":"arn:svc-10"}]}"#),
            ),
        ]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (services, _token) = client
            .describe_services("my-cluster", Some(arns), None, None)
            .await
            .unwrap();

        assert_eq!(services.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_services_propagates_discovery_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"cluster":"my-cluster"}"#),
            json_error_response("ClusterNotFoundException", "no such cluster"),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_services("my-cluster", None, None, None)
            .await
            .unwrap_err();

        assert!(matches!(err, VaporError::AwsSdk { .. }));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_tasks_filters_by_service_and_status_with_include() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"cluster":"my-cluster","serviceName":"arn:svc-a","desiredStatus":"RUNNING"}"#,
                ),
                json_response(200, r#"{"taskArns":["arn:task-a"]}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"cluster":"my-cluster","tasks":["arn:task-a"],"include":["TAGS"]}"#,
                ),
                json_response(
                    200,
                    r#"{"tasks":[{"taskArn":"arn:task-a","lastStatus":"RUNNING"}]}"#,
                ),
            ),
        ]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (tasks, token) = client
            .describe_tasks(
                "my-cluster",
                Some("arn:svc-a".to_string()),
                Some("RUNNING".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].last_status.as_deref(), Some("RUNNING"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_tasks_paginates_discovery_and_resumes_with_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"cluster":"my-cluster","maxResults":1}"#),
                json_response(200, r#"{"taskArns":["arn:task-a"],"nextToken":"cursor-a"}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"cluster":"my-cluster","tasks":["arn:task-a"],"include":["TAGS"]}"#,
                ),
                json_response(200, r#"{"tasks":[{"taskArn":"arn:task-a"}]}"#),
            ),
        ]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (tasks, token) = client
            .describe_tasks("my-cluster", None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(token.as_deref(), Some("cursor-a"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_tasks_returns_empty_when_no_tasks_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"cluster":"my-cluster"}"#),
            json_response(200, r#"{"taskArns":[]}"#),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (tasks, token) = client
            .describe_tasks("my-cluster", None, None, None, None)
            .await
            .unwrap();

        assert!(tasks.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_tasks_propagates_discovery_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"cluster":"my-cluster"}"#),
            json_error_response("ClusterNotFoundException", "no such cluster"),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_tasks("my-cluster", None, None, None, None)
            .await
            .unwrap_err();

        assert!(matches!(err, VaporError::AwsSdk { .. }));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_task_definition_returns_found_definition() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"taskDefinition":"my-family:1"}"#),
            json_response(
                200,
                r#"{"taskDefinition":{"taskDefinitionArn":"arn:td-1","family":"my-family","revision":1}}"#,
            ),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let result = client
            .describe_task_definition("my-family:1")
            .await
            .unwrap();

        let td = result.expect("expected Some(TaskDefinition)");
        assert_eq!(td.family.as_deref(), Some("my-family"));
        assert_eq!(td.revision, 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_task_definition_returns_none_on_client_exception() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"taskDefinition":"missing:1"}"#),
            json_error_response("ClientException", "not found"),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let result = client.describe_task_definition("missing:1").await.unwrap();

        assert!(result.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_task_definition_returns_none_on_invalid_parameter_exception() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"taskDefinition":"bad:1"}"#),
            json_error_response("InvalidParameterException", "invalid"),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let result = client.describe_task_definition("bad:1").await.unwrap();

        assert!(result.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_task_definition_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"taskDefinition":"my-family:1"}"#),
            json_error_response("AccessDeniedException", "denied"),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_task_definition("my-family:1")
            .await
            .unwrap_err();

        assert!(matches!(err, VaporError::AwsSdk { .. }));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_task_definitions_filters_by_family_prefix_and_status() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"familyPrefix":"my-family","status":"ACTIVE"}"#,
            ),
            json_response(200, r#"{"taskDefinitionArns":["arn:td-1","arn:td-2"]}"#),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (arns, token) = client
            .list_task_definitions(
                Some("my-family".to_string()),
                Some("ACTIVE".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(arns, vec!["arn:td-1", "arn:td-2"]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_task_definitions_paginates_and_resumes_with_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"maxResults":2}"#),
                json_response(
                    200,
                    r#"{"taskDefinitionArns":["arn:td-1"],"nextToken":"cursor-a"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"cursor-a","maxResults":1}"#),
                json_response(200, r#"{"taskDefinitionArns":["arn:td-2"]}"#),
            ),
        ]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let (arns, token) = client
            .list_task_definitions(None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(arns, vec!["arn:td-1", "arn:td-2"]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_task_definitions_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidParameterException", "bad param"),
        )]);
        let client = EcsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_task_definitions(None, None, None, None)
            .await
            .unwrap_err();

        assert!(matches!(err, VaporError::AwsSdk { .. }));
        http_client.relaxed_requests_match();
    }
}
