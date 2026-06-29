use async_graphql::SimpleObject;

use crate::aws::app_runner::{
    AppRunnerCodeRepoInfo, AppRunnerImageRepoInfo, AppRunnerInstanceConfigInfo,
    AppRunnerServiceInfo, AppRunnerSourceConfigInfo, AppRunnerVpcConnectorInfo,
};

#[derive(SimpleObject, Clone)]
pub struct AppRunnerImageRepo {
    pub image_identifier: String,
    pub image_repository_type: String,
}

impl From<AppRunnerImageRepoInfo> for AppRunnerImageRepo {
    fn from(r: AppRunnerImageRepoInfo) -> Self {
        Self {
            image_identifier: r.image_identifier,
            image_repository_type: r.image_repository_type,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AppRunnerCodeRepo {
    pub repository_url: String,
    pub source_code_version: Option<String>,
}

impl From<AppRunnerCodeRepoInfo> for AppRunnerCodeRepo {
    fn from(r: AppRunnerCodeRepoInfo) -> Self {
        Self {
            repository_url: r.repository_url,
            source_code_version: r.source_code_version,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AppRunnerSourceConfig {
    pub image_repository: Option<AppRunnerImageRepo>,
    pub code_repository: Option<AppRunnerCodeRepo>,
    pub auto_deployments_enabled: Option<bool>,
}

impl From<AppRunnerSourceConfigInfo> for AppRunnerSourceConfig {
    fn from(s: AppRunnerSourceConfigInfo) -> Self {
        Self {
            image_repository: s.image_repository.map(AppRunnerImageRepo::from),
            code_repository: s.code_repository.map(AppRunnerCodeRepo::from),
            auto_deployments_enabled: s.auto_deployments_enabled,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AppRunnerInstanceConfig {
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

impl From<AppRunnerInstanceConfigInfo> for AppRunnerInstanceConfig {
    fn from(i: AppRunnerInstanceConfigInfo) -> Self {
        Self {
            cpu: i.cpu,
            memory: i.memory,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AppRunnerService {
    pub service_id: Option<String>,
    pub service_name: String,
    pub service_arn: String,
    pub service_url: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub source_configuration: Option<AppRunnerSourceConfig>,
    pub instance_configuration: Option<AppRunnerInstanceConfig>,
}

impl From<AppRunnerServiceInfo> for AppRunnerService {
    fn from(s: AppRunnerServiceInfo) -> Self {
        Self {
            service_id: s.service_id,
            service_name: s.service_name,
            service_arn: s.service_arn,
            service_url: s.service_url,
            status: s.status,
            created_at: s.created_at,
            updated_at: s.updated_at,
            source_configuration: s.source_configuration.map(AppRunnerSourceConfig::from),
            instance_configuration: s.instance_configuration.map(AppRunnerInstanceConfig::from),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AppRunnerVpcConnector {
    pub vpc_connector_name: String,
    pub vpc_connector_arn: String,
    pub vpc_connector_revision: i32,
    pub vpc_id: Option<String>,
    pub subnets: Vec<String>,
    pub security_groups: Vec<String>,
    pub status: String,
}

impl From<AppRunnerVpcConnectorInfo> for AppRunnerVpcConnector {
    fn from(v: AppRunnerVpcConnectorInfo) -> Self {
        Self {
            vpc_connector_name: v.vpc_connector_name,
            vpc_connector_arn: v.vpc_connector_arn,
            vpc_connector_revision: v.vpc_connector_revision,
            vpc_id: v.vpc_id,
            subnets: v.subnets,
            security_groups: v.security_groups,
            status: v.status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::app_runner::{
        AppRunnerCodeRepoInfo, AppRunnerImageRepoInfo, AppRunnerInstanceConfigInfo,
        AppRunnerServiceInfo, AppRunnerSourceConfigInfo, AppRunnerVpcConnectorInfo,
    };

    #[test]
    fn test_app_runner_service_from_minimal() {
        let info = AppRunnerServiceInfo {
            service_id: None,
            service_name: "my-service".to_string(),
            service_arn: "arn:aws:apprunner:us-east-1:123456789012:service/my-service/abc123".to_string(),
            service_url: None,
            status: "RUNNING".to_string(),
            created_at: None,
            updated_at: None,
            source_configuration: None,
            instance_configuration: None,
        };
        let result = AppRunnerService::from(info);
        assert_eq!(result.service_name, "my-service");
        assert_eq!(result.status, "RUNNING");
        assert!(result.service_id.is_none());
        assert!(result.source_configuration.is_none());
        assert!(result.instance_configuration.is_none());
    }

    #[test]
    fn test_app_runner_service_from_full() {
        let info = AppRunnerServiceInfo {
            service_id: Some("abc123".to_string()),
            service_name: "my-api".to_string(),
            service_arn: "arn:aws:apprunner:us-east-1:123456789012:service/my-api/abc123".to_string(),
            service_url: Some("abc123.us-east-1.awsapprunner.com".to_string()),
            status: "RUNNING".to_string(),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            updated_at: Some("2024-06-01T00:00:00Z".to_string()),
            source_configuration: Some(AppRunnerSourceConfigInfo {
                image_repository: Some(AppRunnerImageRepoInfo {
                    image_identifier: "public.ecr.aws/my-org/my-image:latest".to_string(),
                    image_repository_type: "ECR_PUBLIC".to_string(),
                }),
                code_repository: None,
                auto_deployments_enabled: Some(true),
            }),
            instance_configuration: Some(AppRunnerInstanceConfigInfo {
                cpu: Some("1024".to_string()),
                memory: Some("2048".to_string()),
            }),
        };
        let result = AppRunnerService::from(info);
        assert_eq!(result.service_id, Some("abc123".to_string()));
        assert_eq!(result.service_url, Some("abc123.us-east-1.awsapprunner.com".to_string()));
        let src = result.source_configuration.unwrap();
        let img = src.image_repository.unwrap();
        assert_eq!(img.image_repository_type, "ECR_PUBLIC");
        assert_eq!(src.auto_deployments_enabled, Some(true));
        assert!(src.code_repository.is_none());
        let ic = result.instance_configuration.unwrap();
        assert_eq!(ic.cpu, Some("1024".to_string()));
        assert_eq!(ic.memory, Some("2048".to_string()));
    }

    #[test]
    fn test_app_runner_service_with_code_repo() {
        let info = AppRunnerServiceInfo {
            service_id: Some("def456".to_string()),
            service_name: "code-service".to_string(),
            service_arn: "arn:aws:apprunner:us-east-1:123456789012:service/code-service/def456".to_string(),
            service_url: None,
            status: "CREATE_FAILED".to_string(),
            created_at: None,
            updated_at: None,
            source_configuration: Some(AppRunnerSourceConfigInfo {
                image_repository: None,
                code_repository: Some(AppRunnerCodeRepoInfo {
                    repository_url: "https://github.com/myorg/myrepo".to_string(),
                    source_code_version: Some("main".to_string()),
                }),
                auto_deployments_enabled: Some(false),
            }),
            instance_configuration: None,
        };
        let result = AppRunnerService::from(info);
        assert_eq!(result.status, "CREATE_FAILED");
        let src = result.source_configuration.unwrap();
        assert!(src.image_repository.is_none());
        let cr = src.code_repository.unwrap();
        assert_eq!(cr.repository_url, "https://github.com/myorg/myrepo");
        assert_eq!(cr.source_code_version, Some("main".to_string()));
    }

    #[test]
    fn test_app_runner_vpc_connector_from() {
        let info = AppRunnerVpcConnectorInfo {
            vpc_connector_name: "my-connector".to_string(),
            vpc_connector_arn: "arn:aws:apprunner:us-east-1:123456789012:vpcconnector/my-connector/1/abc123".to_string(),
            vpc_connector_revision: 1,
            vpc_id: Some("vpc-12345678".to_string()),
            subnets: vec!["subnet-aaa".to_string(), "subnet-bbb".to_string()],
            security_groups: vec!["sg-xyz".to_string()],
            status: "ACTIVE".to_string(),
        };
        let result = AppRunnerVpcConnector::from(info);
        assert_eq!(result.vpc_connector_name, "my-connector");
        assert_eq!(result.vpc_connector_revision, 1);
        assert_eq!(result.vpc_id, Some("vpc-12345678".to_string()));
        assert_eq!(result.subnets, vec!["subnet-aaa", "subnet-bbb"]);
        assert_eq!(result.security_groups, vec!["sg-xyz"]);
        assert_eq!(result.status, "ACTIVE");
    }

    #[test]
    fn test_app_runner_vpc_connector_no_vpc_id() {
        let info = AppRunnerVpcConnectorInfo {
            vpc_connector_name: "orphan-connector".to_string(),
            vpc_connector_arn: "arn:aws:apprunner:us-east-1:123456789012:vpcconnector/orphan/1/xyz".to_string(),
            vpc_connector_revision: 2,
            vpc_id: None,
            subnets: vec![],
            security_groups: vec![],
            status: "INACTIVE".to_string(),
        };
        let result = AppRunnerVpcConnector::from(info);
        assert!(result.vpc_id.is_none());
        assert!(result.subnets.is_empty());
        assert!(result.security_groups.is_empty());
        assert_eq!(result.vpc_connector_revision, 2);
    }
}
