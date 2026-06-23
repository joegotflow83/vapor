use async_graphql::SimpleObject;
use aws_sdk_codedeploy::types::{
    ApplicationInfo as SdkApplicationInfo, DeploymentGroupInfo as SdkDeploymentGroupInfo,
    DeploymentInfo as SdkDeploymentInfo,
};

#[derive(SimpleObject, Clone)]
pub struct DeployApplication {
    pub id: Option<String>,
    pub name: String,
    pub compute_platform: Option<String>,
    pub created_at: Option<String>,
}

impl From<SdkApplicationInfo> for DeployApplication {
    fn from(a: SdkApplicationInfo) -> Self {
        Self {
            id: a.application_id().map(|v| v.to_string()),
            name: a.application_name().unwrap_or_default().to_string(),
            compute_platform: a.compute_platform().map(|v| v.as_str().to_string()),
            created_at: a.create_time().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DeploymentGroup {
    pub id: Option<String>,
    pub name: String,
    pub application_name: Option<String>,
    pub deployment_config_name: Option<String>,
    pub deployment_style: Option<String>,
    pub service_role_arn: Option<String>,
    pub target_revision_type: Option<String>,
}

impl From<SdkDeploymentGroupInfo> for DeploymentGroup {
    fn from(g: SdkDeploymentGroupInfo) -> Self {
        let deployment_style = g.deployment_style().and_then(|s| s.deployment_type()).map(|t| t.as_str().to_string());
        let target_revision_type = g.target_revision().and_then(|r| r.revision_type()).map(|t| t.as_str().to_string());
        Self {
            id: g.deployment_group_id().map(|v| v.to_string()),
            name: g.deployment_group_name().unwrap_or_default().to_string(),
            application_name: g.application_name().map(|v| v.to_string()),
            deployment_config_name: g.deployment_config_name().map(|v| v.to_string()),
            deployment_style,
            service_role_arn: g.service_role_arn().map(|v| v.to_string()),
            target_revision_type,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Deployment {
    pub deployment_id: String,
    pub application_name: Option<String>,
    pub deployment_group_name: Option<String>,
    pub status: Option<String>,
    pub create_time: Option<String>,
    pub start_time: Option<String>,
    pub complete_time: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl From<SdkDeploymentInfo> for Deployment {
    fn from(d: SdkDeploymentInfo) -> Self {
        let (error_code, error_message) = match d.error_information() {
            Some(e) => (
                e.code().map(|c| c.as_str().to_string()),
                e.message().map(|m| m.to_string()),
            ),
            None => (None, None),
        };
        Self {
            deployment_id: d.deployment_id().unwrap_or_default().to_string(),
            application_name: d.application_name().map(|v| v.to_string()),
            deployment_group_name: d.deployment_group_name().map(|v| v.to_string()),
            status: d.status().map(|s| s.as_str().to_string()),
            create_time: d.create_time().map(|t| t.to_string()),
            start_time: d.start_time().map(|t| t.to_string()),
            complete_time: d.complete_time().map(|t| t.to_string()),
            error_code,
            error_message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_application_from_sdk() {
        let app = SdkApplicationInfo::builder()
            .application_id("app-123")
            .application_name("my-app")
            .compute_platform(aws_sdk_codedeploy::types::ComputePlatform::Server)
            .build();
        let da = DeployApplication::from(app);
        assert_eq!(da.id, Some("app-123".to_string()));
        assert_eq!(da.name, "my-app");
        assert_eq!(da.compute_platform, Some("Server".to_string()));
    }

    #[test]
    fn test_deploy_application_minimal() {
        let app = SdkApplicationInfo::builder().build();
        let da = DeployApplication::from(app);
        assert_eq!(da.name, "");
        assert!(da.id.is_none());
        assert!(da.compute_platform.is_none());
        assert!(da.created_at.is_none());
    }

    #[test]
    fn test_deployment_group_from_sdk() {
        let group = SdkDeploymentGroupInfo::builder()
            .deployment_group_id("dg-123")
            .deployment_group_name("prod-group")
            .application_name("my-app")
            .deployment_config_name("CodeDeployDefault.OneAtATime")
            .service_role_arn("arn:aws:iam::123456789012:role/deploy-role")
            .build();
        let dg = DeploymentGroup::from(group);
        assert_eq!(dg.id, Some("dg-123".to_string()));
        assert_eq!(dg.name, "prod-group");
        assert_eq!(dg.application_name, Some("my-app".to_string()));
        assert_eq!(dg.deployment_config_name, Some("CodeDeployDefault.OneAtATime".to_string()));
        assert_eq!(dg.service_role_arn, Some("arn:aws:iam::123456789012:role/deploy-role".to_string()));
    }

    #[test]
    fn test_deployment_group_minimal() {
        let group = SdkDeploymentGroupInfo::builder().build();
        let dg = DeploymentGroup::from(group);
        assert_eq!(dg.name, "");
        assert!(dg.id.is_none());
        assert!(dg.deployment_style.is_none());
        assert!(dg.target_revision_type.is_none());
    }

    #[test]
    fn test_deployment_from_sdk() {
        let dep = SdkDeploymentInfo::builder()
            .deployment_id("d-ABC123")
            .application_name("my-app")
            .deployment_group_name("prod-group")
            .status(aws_sdk_codedeploy::types::DeploymentStatus::Succeeded)
            .build();
        let d = Deployment::from(dep);
        assert_eq!(d.deployment_id, "d-ABC123");
        assert_eq!(d.application_name, Some("my-app".to_string()));
        assert_eq!(d.deployment_group_name, Some("prod-group".to_string()));
        assert_eq!(d.status, Some("Succeeded".to_string()));
        assert!(d.error_code.is_none());
        assert!(d.error_message.is_none());
    }

    #[test]
    fn test_deployment_minimal() {
        let dep = SdkDeploymentInfo::builder().build();
        let d = Deployment::from(dep);
        assert_eq!(d.deployment_id, "");
        assert!(d.application_name.is_none());
        assert!(d.status.is_none());
        assert!(d.create_time.is_none());
    }
}
