use aws_config::SdkConfig;

use crate::error::VaporError;

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
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
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

pub struct AppRunnerClient {
    inner: aws_sdk_apprunner::Client,
}

impl AppRunnerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_apprunner::Client::new(config),
        }
    }

    pub async fn list_services(&self) -> Result<Vec<AppRunnerServiceInfo>, VaporError> {
        let mut arns: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_services();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for svc in output.service_summary_list() {
                if let Some(arn) = svc.service_arn() {
                    arns.push(arn.to_string());
                }
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        let mut services = Vec::with_capacity(arns.len());
        for arn in arns {
            if let Some(svc) = self.describe_service(&arn).await? {
                services.push(svc);
            }
        }
        Ok(services)
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
                let msg = e.to_string();
                if msg.contains("InvalidRequestException") || msg.contains("ResourceNotFoundException") {
                    return Ok(None);
                }
                return Err(VaporError::AwsSdk(msg));
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
            created_at: Some(svc.created_at().to_string()),
            updated_at: Some(svc.updated_at().to_string()),
            source_configuration,
            instance_configuration,
        }))
    }

    pub async fn list_vpc_connectors(&self) -> Result<Vec<AppRunnerVpcConnectorInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_vpc_connectors();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for vc in output.vpc_connectors() {
                items.push(AppRunnerVpcConnectorInfo {
                    vpc_connector_name: vc.vpc_connector_name().unwrap_or_default().to_string(),
                    vpc_connector_arn: vc.vpc_connector_arn().unwrap_or_default().to_string(),
                    vpc_connector_revision: vc.vpc_connector_revision(),
                    // VpcConnector exposes only subnets/security groups, not a vpc_id.
                    vpc_id: None,
                    subnets: vc.subnets().to_vec(),
                    security_groups: vc.security_groups().to_vec(),
                    status: vc.status().map(|s| s.as_str()).unwrap_or_default().to_string(),
                });
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
