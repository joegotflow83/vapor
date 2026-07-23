use aws_config::SdkConfig;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;

use crate::error::VaporError;

#[derive(Debug)]
pub struct MqBrokerInstanceInfo {
    pub console_url: Option<String>,
    pub endpoints: Vec<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug)]
pub struct MqBrokerInfo {
    pub broker_id: Option<String>,
    pub broker_arn: Option<String>,
    pub broker_name: Option<String>,
    pub broker_state: Option<String>,
    pub engine_type: Option<String>,
    pub engine_version: Option<String>,
    pub deployment_mode: Option<String>,
    pub host_instance_type: Option<String>,
    pub publicly_accessible: Option<bool>,
    pub broker_instances: Vec<MqBrokerInstanceInfo>,
    pub subnet_ids: Vec<String>,
    pub security_groups: Vec<String>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct MqConfigurationInfo {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub engine_type: Option<String>,
    pub engine_version: Option<String>,
    pub description: Option<String>,
    pub latest_revision: Option<i32>,
    pub created: Option<aws_smithy_types::DateTime>,
    pub tags: Vec<(String, String)>,
}

pub struct MqClient {
    inner: aws_sdk_mq::Client,
}

impl MqClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_mq::Client::new(config),
        }
    }

    /// Lists brokers, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `ListBrokers` has both `max_results` and
    /// `next_token` (verified against pinned `aws-sdk-mq` 1.107.0's
    /// `operation/list_brokers/_list_brokers_input.rs`), so `limit` is capped
    /// to the remaining budget on the request itself, matching `kinesis.rs`'s
    /// `list_streams` pattern. The N+1 `describe_broker` fan-out only covers
    /// the single page of ids collected this call, not the whole collection —
    /// same as before, just now bounded to one page instead of looping until
    /// `limit` or exhaustion internally.
    pub async fn list_brokers(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<MqBrokerInfo>, Option<String>), VaporError> {
        let mut ids: Vec<String> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_brokers();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - ids.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for summary in output.broker_summaries() {
                if let Some(id) = summary.broker_id() {
                    ids.push(id.to_string());
                }
            }
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if ids.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut brokers = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(broker) = self.describe_broker(&id).await? {
                brokers.push(broker);
            }
        }
        Ok((brokers, token))
    }

    pub async fn describe_broker(&self, broker_id: &str) -> Result<Option<MqBrokerInfo>, VaporError> {
        let output = match self.inner.describe_broker().broker_id(broker_id).send().await {
            Ok(o) => o,
            Err(e) => {
                if matches!(e.code(), Some("NotFoundException") | Some("ResourceNotFoundException")) {
                    return Ok(None);
                }
                return Err(crate::error::sdk_err(e));
            }
        };

        let broker_instances = output
            .broker_instances()
            .iter()
            .map(|bi| MqBrokerInstanceInfo {
                console_url: bi.console_url().map(|s| s.to_string()),
                endpoints: bi.endpoints().to_vec(),
                ip_address: bi.ip_address().map(|s| s.to_string()),
            })
            .collect();

        let tags = output
            .tags()
            .map(|t| t.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        Ok(Some(MqBrokerInfo {
            broker_id: output.broker_id().map(|s| s.to_string()),
            broker_arn: output.broker_arn().map(|s| s.to_string()),
            broker_name: output.broker_name().map(|s| s.to_string()),
            broker_state: output.broker_state().map(|s| s.as_str().to_string()),
            engine_type: output.engine_type().map(|s| s.as_str().to_string()),
            engine_version: output.engine_version().map(|s| s.to_string()),
            deployment_mode: output.deployment_mode().map(|s| s.as_str().to_string()),
            host_instance_type: output.host_instance_type().map(|s| s.to_string()),
            publicly_accessible: output.publicly_accessible(),
            broker_instances,
            subnet_ids: output.subnet_ids().to_vec(),
            security_groups: output.security_groups().to_vec(),
            tags,
        }))
    }

    /// Lists configurations, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListConfigurations` has both
    /// `max_results` and `next_token` (verified against pinned `aws-sdk-mq`
    /// 1.107.0's `operation/list_configurations/_list_configurations_input.rs`
    /// — the earlier claim of "no SDK paginator" only meant no *generated
    /// paginator*, the request-level `max_results` field was there all along),
    /// so `limit` is capped to the remaining budget on the request itself,
    /// matching `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_configurations(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<MqConfigurationInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_configurations();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for cfg in output.configurations() {
                let tags = cfg
                    .tags()
                    .map(|t| t.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                items.push(MqConfigurationInfo {
                    id: cfg.id().map(|s| s.to_string()),
                    arn: cfg.arn().map(|s| s.to_string()),
                    name: cfg.name().map(|s| s.to_string()),
                    engine_type: cfg.engine_type().map(|s| s.as_str().to_string()),
                    engine_version: cfg.engine_version().map(|s| s.to_string()),
                    description: cfg.description().map(|s| s.to_string()),
                    latest_revision: cfg.latest_revision().and_then(|r| r.revision()),
                    created: cfg.created().cloned(),
                    tags,
                });
            }
            token = output.next_token().map(|t| t.to_string());

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
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const BASE: &str = "https://mq.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_brokers_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers"), ""),
                json_response(
                    200,
                    r#"{"brokerSummaries":[{"brokerId":"b1"},{"brokerId":"b2"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b1"), ""),
                json_response(
                    200,
                    r#"{"brokerId":"b1","brokerArn":"arn:b1","brokerName":"broker-one","brokerState":"RUNNING","engineType":"ACTIVEMQ","engineVersion":"5.17.6","deploymentMode":"SINGLE_INSTANCE","hostInstanceType":"mq.t3.micro","publiclyAccessible":true,"brokerInstances":[{"consoleURL":"https://console/b1","endpoints":["ssl://ep1:61617"],"ipAddress":"10.0.0.1"}],"subnetIds":["subnet-1"],"securityGroups":["sg-1"],"tags":{"env":"prod"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b2"), ""),
                json_response(200, r#"{"brokerId":"b2"}"#),
            ),
        ]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_brokers(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let b1 = &items[0];
        assert_eq!(b1.broker_id, Some("b1".to_string()));
        assert_eq!(b1.broker_arn, Some("arn:b1".to_string()));
        assert_eq!(b1.broker_name, Some("broker-one".to_string()));
        assert_eq!(b1.broker_state, Some("RUNNING".to_string()));
        assert_eq!(b1.engine_type, Some("ACTIVEMQ".to_string()));
        assert_eq!(b1.engine_version, Some("5.17.6".to_string()));
        assert_eq!(b1.deployment_mode, Some("SINGLE_INSTANCE".to_string()));
        assert_eq!(b1.host_instance_type, Some("mq.t3.micro".to_string()));
        assert_eq!(b1.publicly_accessible, Some(true));
        assert_eq!(b1.broker_instances.len(), 1);
        assert_eq!(
            b1.broker_instances[0].console_url,
            Some("https://console/b1".to_string())
        );
        assert_eq!(b1.broker_instances[0].endpoints, vec!["ssl://ep1:61617".to_string()]);
        assert_eq!(b1.broker_instances[0].ip_address, Some("10.0.0.1".to_string()));
        assert_eq!(b1.subnet_ids, vec!["subnet-1".to_string()]);
        assert_eq!(b1.security_groups, vec!["sg-1".to_string()]);
        assert_eq!(b1.tags, vec![("env".to_string(), "prod".to_string())]);

        let b2 = &items[1];
        assert_eq!(b2.broker_id, Some("b2".to_string()));
        assert_eq!(b2.broker_arn, None);
        assert!(b2.broker_instances.is_empty());
        assert!(b2.tags.is_empty());

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_brokers_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers?nextToken=cursor-a"), ""),
                json_response(200, r#"{"brokerSummaries":[{"brokerId":"b3"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b3"), ""),
                json_response(200, r#"{"brokerId":"b3"}"#),
            ),
        ]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_brokers(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_brokers_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers?maxResults=2"), ""),
                json_response(
                    200,
                    r#"{"brokerSummaries":[{"brokerId":"b1"},{"brokerId":"b2"}],"nextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b1"), ""),
                json_response(200, r#"{"brokerId":"b1"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b2"), ""),
                json_response(200, r#"{"brokerId":"b2"}"#),
            ),
        ]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_brokers(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_brokers_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"brokerSummaries":[{"brokerId":"b1"},{"brokerId":"b2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers?maxResults=8&nextToken=p2"), ""),
                json_response(200, r#"{"brokerSummaries":[{"brokerId":"b3"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b1"), ""),
                json_response(200, r#"{"brokerId":"b1"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b2"), ""),
                json_response(200, r#"{"brokerId":"b2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b3"), ""),
                json_response(200, r#"{"brokerId":"b3"}"#),
            ),
        ]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_brokers(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_brokers_propagates_errors() {
        // `BadRequestException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/brokers"), ""),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let err = client.list_brokers(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_brokers_propagates_describe_broker_errors() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers"), ""),
                json_response(200, r#"{"brokerSummaries":[{"brokerId":"b1"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b1"), ""),
                json_error_response("ForbiddenException", "not allowed"),
            ),
        ]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let err = client.list_brokers(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ForbiddenException".to_string()));
                assert_eq!(message, "not allowed");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_brokers_skips_broker_when_describe_broker_reports_not_found() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers"), ""),
                json_response(200, r#"{"brokerSummaries":[{"brokerId":"b1"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b1"), ""),
                json_error_response("NotFoundException", "broker not found"),
            ),
        ]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_brokers(None, None).await.unwrap();

        assert!(items.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_broker_returns_none_when_resource_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/brokers/missing"), ""),
            json_error_response("ResourceNotFoundException", "no such broker"),
        )]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let broker = client.describe_broker("missing").await.unwrap();

        assert!(broker.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_broker_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/brokers/b1"), ""),
            json_error_response("InternalServerErrorException", "boom"),
        )]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_broker("b1").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InternalServerErrorException".to_string()));
                assert_eq!(message, "boom");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configurations_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/configurations"), ""),
            json_response(
                200,
                r#"{"configurations":[{"id":"c1","arn":"arn:c1","name":"cfg-one","engineType":"RABBITMQ","engineVersion":"3.11.20","description":"first config","latestRevision":{"revision":3,"created":"2024-01-01T00:00:00Z"},"tags":{"team":"platform"}},{"id":"c2"}]}"#,
            ),
        )]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_configurations(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let c1 = &items[0];
        assert_eq!(c1.id, Some("c1".to_string()));
        assert_eq!(c1.arn, Some("arn:c1".to_string()));
        assert_eq!(c1.name, Some("cfg-one".to_string()));
        assert_eq!(c1.engine_type, Some("RABBITMQ".to_string()));
        assert_eq!(c1.engine_version, Some("3.11.20".to_string()));
        assert_eq!(c1.description, Some("first config".to_string()));
        assert_eq!(c1.latest_revision, Some(3));
        assert_eq!(c1.tags, vec![("team".to_string(), "platform".to_string())]);

        let c2 = &items[1];
        assert_eq!(c2.id, Some("c2".to_string()));
        // `configuration_correct_errors` in the pinned SDK default-fills
        // omitted fields rather than leaving them `None` (extends the
        // memory's gotcha 14/16 to a case where the accessor keeps its
        // `Option` wrapper but the *value* is defaulted) — `latest_revision`
        // comes back `Some` with a defaulted `ConfigurationRevision`
        // (revision 0), not `None`.
        assert_eq!(c2.latest_revision, Some(0));
        assert_eq!(c2.arn, Some(String::new()));

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configurations_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/configurations?nextToken=cursor-a"), ""),
            json_response(200, r#"{"configurations":[{"id":"c3"}]}"#),
        )]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_configurations(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configurations_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/configurations?maxResults=2"), ""),
            json_response(
                200,
                r#"{"configurations":[{"id":"c1"},{"id":"c2"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_configurations(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configurations_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/configurations?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"configurations":[{"id":"c1"},{"id":"c2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/configurations?maxResults=8&nextToken=p2"), ""),
                json_response(200, r#"{"configurations":[{"id":"c3"}]}"#),
            ),
        ]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_configurations(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configurations_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/configurations"), ""),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = MqClient::new(&sdk_config(http_client.clone()));

        let err = client.list_configurations(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

