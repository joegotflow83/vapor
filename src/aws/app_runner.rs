use aws_config::SdkConfig;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
use chrono::{DateTime, Utc};

use crate::error::VaporError;
use crate::schema::time::to_utc;

pub struct AppRunnerImageRepoInfo {
    pub image_identifier: String,
    pub image_repository_type: String,
}

pub struct AppRunnerCodeRepoInfo {
    pub repository_url: String,
    pub source_code_version: Option<String>,
}

pub struct AppRunnerSourceConfigInfo {
    pub image_repository: Option<AppRunnerImageRepoInfo>,
    pub code_repository: Option<AppRunnerCodeRepoInfo>,
    pub auto_deployments_enabled: Option<bool>,
}

pub struct AppRunnerInstanceConfigInfo {
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

pub struct AppRunnerServiceInfo {
    pub service_id: Option<String>,
    pub service_name: String,
    pub service_arn: String,
    pub service_url: Option<String>,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub source_configuration: Option<AppRunnerSourceConfigInfo>,
    pub instance_configuration: Option<AppRunnerInstanceConfigInfo>,
}

pub struct AppRunnerVpcConnectorInfo {
    pub vpc_connector_name: String,
    pub vpc_connector_arn: String,
    pub vpc_connector_revision: i32,
    pub vpc_id: Option<String>,
    pub subnets: Vec<String>,
    pub security_groups: Vec<String>,
    pub status: String,
}

pub struct AppRunnerConnectionInfo {
    pub connection_name: Option<String>,
    pub connection_arn: Option<String>,
    pub provider_type: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

pub struct AppRunnerObservabilityConfigurationInfo {
    pub observability_configuration_arn: Option<String>,
    pub observability_configuration_name: Option<String>,
    pub observability_configuration_revision: i32,
    pub tracing_vendor: Option<String>,
    pub latest: bool,
    pub status: Option<String>,
}

pub struct AppRunnerClient {
    inner: aws_sdk_apprunner::Client,
}

impl AppRunnerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_apprunner::Client::new(config),
        }
    }

    /// Lists App Runner services. `limit` caps the number of ARNs fetched
    /// (and thus the number of `describe_service` calls made) before the
    /// per-ARN detail fan-out, scoped to one page; `next_token` resumes from
    /// a prior page. Default unlimited.
    pub async fn list_services(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AppRunnerServiceInfo>, Option<String>), VaporError> {
        let mut arns: Vec<String> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_services();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - arns.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            arns.extend(
                output
                    .service_summary_list
                    .into_iter()
                    .filter_map(|svc| svc.service_arn),
            );

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if arns.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut services = Vec::with_capacity(arns.len());
        for arn in arns {
            if let Some(svc) = self.describe_service(&arn).await? {
                services.push(svc);
            }
        }
        Ok((services, token))
    }

    pub async fn describe_service(
        &self,
        service_arn: &str,
    ) -> Result<Option<AppRunnerServiceInfo>, VaporError> {
        let output = match self
            .inner
            .describe_service()
            .service_arn(service_arn)
            .send()
            .await
        {
            Ok(o) => o,
            Err(e) => {
                if matches!(e.code(), Some("InvalidRequestException") | Some("ResourceNotFoundException")) {
                    return Ok(None);
                }
                return Err(crate::error::sdk_err(e));
            }
        };

        let svc = match output.service() {
            Some(s) => s,
            None => return Ok(None),
        };

        let source_configuration = svc.source_configuration().map(|sc| {
            let image_repository = sc.image_repository().map(|ir| AppRunnerImageRepoInfo {
                image_identifier: ir.image_identifier().to_string(),
                image_repository_type: ir.image_repository_type().as_str().to_string(),
            });
            let code_repository = sc.code_repository().map(|cr| AppRunnerCodeRepoInfo {
                repository_url: cr.repository_url().to_string(),
                source_code_version: cr
                    .source_code_version()
                    .map(|scv| scv.value().to_string()),
            });
            AppRunnerSourceConfigInfo {
                image_repository,
                code_repository,
                auto_deployments_enabled: sc.auto_deployments_enabled(),
            }
        });

        let instance_configuration =
            svc.instance_configuration().map(|ic| AppRunnerInstanceConfigInfo {
                cpu: ic.cpu().map(|s| s.to_string()),
                memory: ic.memory().map(|s| s.to_string()),
            });

        Ok(Some(AppRunnerServiceInfo {
            service_id: Some(svc.service_id().to_string()),
            service_name: svc.service_name().to_string(),
            service_arn: svc.service_arn().to_string(),
            service_url: svc.service_url().map(|s| s.to_string()),
            status: svc.status().as_str().to_string(),
            created_at: to_utc(Some(svc.created_at())),
            updated_at: to_utc(Some(svc.updated_at())),
            source_configuration,
            instance_configuration,
        }))
    }

    /// Lists App Runner VPC connectors. `limit` caps the number of results
    /// returned (default unlimited); `next_token` resumes from a prior page.
    pub async fn list_vpc_connectors(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AppRunnerVpcConnectorInfo>, Option<String>), VaporError> {
        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_vpc_connectors();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            summaries.extend(output.vpc_connectors);

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let items = summaries
            .into_iter()
            .map(|vc| AppRunnerVpcConnectorInfo {
                vpc_connector_name: vc.vpc_connector_name.unwrap_or_default(),
                vpc_connector_arn: vc.vpc_connector_arn.unwrap_or_default(),
                vpc_connector_revision: vc.vpc_connector_revision,
                // VpcConnector exposes only subnets/security groups, not a vpc_id.
                vpc_id: None,
                subnets: vc.subnets.unwrap_or_default(),
                security_groups: vc.security_groups.unwrap_or_default(),
                status: vc.status.map(|s| s.as_str().to_string()).unwrap_or_default(),
            })
            .collect();

        Ok((items, token))
    }

    /// Lists App Runner connections. `limit` caps the number of results
    /// returned (default unlimited); `next_token` resumes from a prior page.
    pub async fn list_connections(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AppRunnerConnectionInfo>, Option<String>), VaporError> {
        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_connections();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            summaries.extend(output.connection_summary_list);

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let items = summaries
            .into_iter()
            .map(|c| AppRunnerConnectionInfo {
                connection_name: c.connection_name,
                connection_arn: c.connection_arn,
                provider_type: c.provider_type.map(|p| p.as_str().to_string()),
                status: c.status.map(|s| s.as_str().to_string()),
                created_at: to_utc(c.created_at.as_ref()),
            })
            .collect();

        Ok((items, token))
    }

    /// Lists App Runner observability configurations. The list API only
    /// returns identification fields (arn/name/revision), so each summary
    /// is fanned out through `describe_observability_configuration` for
    /// tracing vendor, latest flag, and status, scoped to one page. `limit`
    /// caps the number of summaries fetched (and thus describe calls made)
    /// before the fan-out; `next_token` resumes from a prior page. Default
    /// unlimited.
    pub async fn list_observability_configurations(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AppRunnerObservabilityConfigurationInfo>, Option<String>), VaporError> {
        let mut arns: Vec<String> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_observability_configurations();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - arns.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            arns.extend(
                output
                    .observability_configuration_summary_list
                    .into_iter()
                    .filter_map(|c| c.observability_configuration_arn),
            );

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if arns.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut items = Vec::with_capacity(arns.len());
        for arn in arns {
            if let Some(cfg) = self.describe_observability_configuration(&arn).await? {
                items.push(cfg);
            }
        }
        Ok((items, token))
    }

    /// Describes a single observability configuration by ARN, including
    /// tracing vendor, latest flag, and status (not present on the list
    /// summary type).
    pub async fn describe_observability_configuration(
        &self,
        observability_configuration_arn: &str,
    ) -> Result<Option<AppRunnerObservabilityConfigurationInfo>, VaporError> {
        let output = match self
            .inner
            .describe_observability_configuration()
            .observability_configuration_arn(observability_configuration_arn)
            .send()
            .await
        {
            Ok(o) => o,
            Err(e) => {
                if matches!(e.code(), Some("InvalidRequestException") | Some("ResourceNotFoundException")) {
                    return Ok(None);
                }
                return Err(crate::error::sdk_err(e));
            }
        };

        let cfg = match output.observability_configuration() {
            Some(c) => c,
            None => return Ok(None),
        };

        Ok(Some(AppRunnerObservabilityConfigurationInfo {
            observability_configuration_arn: cfg.observability_configuration_arn().map(|s| s.to_string()),
            observability_configuration_name: cfg.observability_configuration_name().map(|s| s.to_string()),
            observability_configuration_revision: cfg.observability_configuration_revision(),
            tracing_vendor: cfg
                .trace_configuration()
                .map(|t| t.vendor().as_str().to_string()),
            latest: cfg.latest(),
            status: cfg.status().map(|s| s.as_str().to_string()),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://apprunner.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_services_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{}"#),
                json_response(
                    200,
                    r#"{"ServiceSummaryList":[{"ServiceArn":"arn:aws:apprunner:us-east-1:1:service/svc1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ServiceArn":"arn:aws:apprunner:us-east-1:1:service/svc1"}"#),
                json_response(
                    200,
                    r#"{"Service":{"ServiceName":"svc1","ServiceId":"id1","ServiceArn":"arn:aws:apprunner:us-east-1:1:service/svc1","ServiceUrl":"svc1.example.com","Status":"RUNNING","CreatedAt":1700000000,"UpdatedAt":1700000100,"SourceConfiguration":{"ImageRepository":{"ImageIdentifier":"public.ecr.aws/x/y:latest","ImageRepositoryType":"ECR_PUBLIC"},"AutoDeploymentsEnabled":true},"InstanceConfiguration":{"Cpu":"1024","Memory":"2048"}}}"#,
                ),
            ),
        ]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client.list_services(None, None).await.unwrap();

        assert_eq!(services.len(), 1);
        let svc = &services[0];
        assert_eq!(svc.service_name, "svc1");
        assert_eq!(svc.service_id.as_deref(), Some("id1"));
        assert_eq!(svc.status, "RUNNING");
        assert!(svc.created_at.is_some());
        let src = svc.source_configuration.as_ref().unwrap();
        assert_eq!(
            src.image_repository.as_ref().unwrap().image_identifier,
            "public.ecr.aws/x/y:latest"
        );
        assert_eq!(src.auto_deployments_enabled, Some(true));
        assert_eq!(
            svc.instance_configuration.as_ref().unwrap().cpu.as_deref(),
            Some("1024")
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_services_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"ServiceSummaryList":[]}"#),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client
            .list_services(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(services.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_services_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":2}"#),
                json_response(
                    200,
                    r#"{"ServiceSummaryList":[{"ServiceArn":"arn-1"},{"ServiceArn":"arn-2"}],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ServiceArn":"arn-1"}"#),
                json_response(
                    200,
                    r#"{"Service":{"ServiceName":"svc1","ServiceId":"id1","ServiceArn":"arn-1","Status":"RUNNING","CreatedAt":1700000000,"UpdatedAt":1700000000}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ServiceArn":"arn-2"}"#),
                json_response(
                    200,
                    r#"{"Service":{"ServiceName":"svc2","ServiceId":"id2","ServiceArn":"arn-2","Status":"RUNNING","CreatedAt":1700000000,"UpdatedAt":1700000000}}"#,
                ),
            ),
        ]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client.list_services(Some(2), None).await.unwrap();

        assert_eq!(services.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_services_pages_through_until_exhausted_when_limit_not_reached() {
        // Same call ordering as acm_pca.rs's fan-out precedent: both list
        // pages are drained into `arns` first, THEN the N+1 describe_service
        // fan-out runs over the whole accumulated set.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"ServiceSummaryList":[{"ServiceArn":"arn-1"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":9}"#),
                json_response(200, r#"{"ServiceSummaryList":[{"ServiceArn":"arn-2"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ServiceArn":"arn-1"}"#),
                json_response(
                    200,
                    r#"{"Service":{"ServiceName":"svc1","ServiceId":"id1","ServiceArn":"arn-1","Status":"RUNNING","CreatedAt":1700000000,"UpdatedAt":1700000000}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ServiceArn":"arn-2"}"#),
                json_response(
                    200,
                    r#"{"Service":{"ServiceName":"svc2","ServiceId":"id2","ServiceArn":"arn-2","Status":"RUNNING","CreatedAt":1700000000,"UpdatedAt":1700000000}}"#,
                ),
            ),
        ]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client.list_services(Some(10), None).await.unwrap();

        assert_eq!(services.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_services_propagates_error_from_list_call() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        match client.list_services(None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code.as_deref(), Some("InvalidRequestException"));
                assert_eq!(message, "bad request");
            }
            Ok(_) => panic!("expected an error"),
            Err(e) => panic!("expected AwsSdk error, got {e:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_service_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ServiceArn":"arn-missing"}"#),
            json_error_response("ResourceNotFoundException", "no such service"),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let result = client.describe_service("arn-missing").await.unwrap();

        assert!(result.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_service_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ServiceArn":"arn-1"}"#),
            json_error_response("InternalServiceErrorException", "internal failure"),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        match client.describe_service("arn-1").await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code.as_deref(), Some("InternalServiceErrorException"));
                assert_eq!(message, "internal failure");
            }
            Ok(_) => panic!("expected an error"),
            Err(e) => panic!("expected AwsSdk error, got {e:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_vpc_connectors_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"VpcConnectors":[{"VpcConnectorName":"conn1","VpcConnectorArn":"arn-1","VpcConnectorRevision":1,"Subnets":["subnet-1"],"SecurityGroups":["sg-1"],"Status":"ACTIVE"}]}"#,
            ),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_vpc_connectors(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].vpc_connector_name, "conn1");
        assert_eq!(items[0].vpc_connector_revision, 1);
        assert_eq!(items[0].subnets, vec!["subnet-1".to_string()]);
        assert_eq!(items[0].security_groups, vec!["sg-1".to_string()]);
        assert_eq!(items[0].status, "ACTIVE");
        assert!(items[0].vpc_id.is_none());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_vpc_connectors_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"VpcConnectors":[{"VpcConnectorRevision":1}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_vpc_connectors(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_connections_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ConnectionSummaryList":[{"ConnectionName":"conn1","ConnectionArn":"arn-1","ProviderType":"GITHUB","Status":"AVAILABLE","CreatedAt":1700000000}]}"#,
            ),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_connections(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].connection_name.as_deref(), Some("conn1"));
        assert_eq!(items[0].provider_type.as_deref(), Some("GITHUB"));
        assert_eq!(items[0].status.as_deref(), Some("AVAILABLE"));
        assert!(items[0].created_at.is_some());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_connections_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"ConnectionSummaryList":[{"ConnectionName":"conn1"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_connections(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_observability_configurations_happy_path() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{}"#),
                json_response(
                    200,
                    r#"{"ObservabilityConfigurationSummaryList":[{"ObservabilityConfigurationArn":"arn-1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ObservabilityConfigurationArn":"arn-1"}"#),
                json_response(
                    200,
                    r#"{"ObservabilityConfiguration":{"ObservabilityConfigurationArn":"arn-1","ObservabilityConfigurationName":"cfg1","ObservabilityConfigurationRevision":1,"TraceConfiguration":{"Vendor":"AWSXRAY"},"Latest":true,"Status":"ACTIVE"}}"#,
                ),
            ),
        ]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_observability_configurations(None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].observability_configuration_name.as_deref(), Some("cfg1"));
        assert_eq!(items[0].tracing_vendor.as_deref(), Some("AWSXRAY"));
        assert!(items[0].latest);
        assert_eq!(items[0].status.as_deref(), Some("ACTIVE"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_observability_configurations_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"ObservabilityConfigurationSummaryList":[{"ObservabilityConfigurationArn":"arn-1"}],"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ObservabilityConfigurationArn":"arn-1"}"#),
                json_response(
                    200,
                    r#"{"ObservabilityConfiguration":{"ObservabilityConfigurationArn":"arn-1","ObservabilityConfigurationRevision":1,"Latest":false}}"#,
                ),
            ),
        ]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_observability_configurations(Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_observability_configuration_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ObservabilityConfigurationArn":"arn-missing"}"#),
            json_error_response("ResourceNotFoundException", "no such config"),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        let result = client
            .describe_observability_configuration("arn-missing")
            .await
            .unwrap();

        assert!(result.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_observability_configuration_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ObservabilityConfigurationArn":"arn-1"}"#),
            json_error_response("InternalServiceErrorException", "internal failure"),
        )]);
        let client = AppRunnerClient::new(&sdk_config(http_client.clone()));

        match client.describe_observability_configuration("arn-1").await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code.as_deref(), Some("InternalServiceErrorException"));
                assert_eq!(message, "internal failure");
            }
            Ok(_) => panic!("expected an error"),
            Err(e) => panic!("expected AwsSdk error, got {e:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

