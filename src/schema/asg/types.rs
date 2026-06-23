use async_graphql::{Enum, SimpleObject};

use crate::schema::ec2::types::Tag;

// === Enums ===

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ScalingActivityStatus {
    Pending,
    WaitingForSpotInstanceRequestId,
    WaitingForSpotInstanceId,
    WaitingForInstanceId,
    PreInService,
    InProgress,
    WaitingForElbConnectionDraining,
    MidLifecycleAction,
    WaitingForInstanceWarmup,
    Successful,
    Failed,
    Cancelled,
    WaitingForConnectionDraining,
}

impl ScalingActivityStatus {
    pub fn from_sdk(s: &aws_sdk_autoscaling::types::ScalingActivityStatusCode) -> Self {
        match s {
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::PendingSpotBidPlacement => {
                Self::Pending
            }
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::WaitingForSpotInstanceRequestId => {
                Self::WaitingForSpotInstanceRequestId
            }
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::WaitingForSpotInstanceId => {
                Self::WaitingForSpotInstanceId
            }
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::WaitingForInstanceId => {
                Self::WaitingForInstanceId
            }
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::PreInService => {
                Self::PreInService
            }
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::InProgress => Self::InProgress,
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::WaitingForElbConnectionDraining => {
                Self::WaitingForElbConnectionDraining
            }
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::MidLifecycleAction => {
                Self::MidLifecycleAction
            }
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::WaitingForInstanceWarmup => {
                Self::WaitingForInstanceWarmup
            }
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::Successful => Self::Successful,
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::Failed => Self::Failed,
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::Cancelled => Self::Cancelled,
            aws_sdk_autoscaling::types::ScalingActivityStatusCode::WaitingForConnectionDraining => {
                Self::WaitingForConnectionDraining
            }
            _ => Self::InProgress,
        }
    }
}

// === Output Types ===

#[derive(SimpleObject, Clone)]
pub struct LaunchTemplateRef {
    pub id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
}

impl From<&aws_sdk_autoscaling::types::LaunchTemplateSpecification> for LaunchTemplateRef {
    fn from(lt: &aws_sdk_autoscaling::types::LaunchTemplateSpecification) -> Self {
        Self {
            id: lt.launch_template_id().map(|s| s.to_string()),
            name: lt.launch_template_name().map(|s| s.to_string()),
            version: lt.version().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AsgInstance {
    pub instance_id: String,
    pub instance_type: Option<String>,
    pub availability_zone: String,
    pub health_status: String,
    pub lifecycle_state: String,
    pub protected_from_scale_in: bool,
}

impl From<&aws_sdk_autoscaling::types::Instance> for AsgInstance {
    fn from(inst: &aws_sdk_autoscaling::types::Instance) -> Self {
        Self {
            instance_id: inst.instance_id().unwrap_or_default().to_string(),
            instance_type: inst.instance_type().map(|t| t.to_string()),
            availability_zone: inst.availability_zone().unwrap_or_default().to_string(),
            health_status: inst.health_status().unwrap_or_default().to_string(),
            lifecycle_state: inst
                .lifecycle_state()
                .map(|s| s.as_str().to_string())
                .unwrap_or_default(),
            protected_from_scale_in: inst.protected_from_scale_in().unwrap_or(false),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AutoScalingGroup {
    pub name: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub desired_capacity: i32,
    pub min_size: i32,
    pub max_size: i32,
    pub default_cooldown: i32,
    pub health_check_type: String,
    pub vpc_zone_identifier: Option<String>,
    pub launch_template: Option<LaunchTemplateRef>,
    pub launch_configuration_name: Option<String>,
    pub instances: Vec<AsgInstance>,
    pub tags: Vec<Tag>,
    pub created_time: Option<String>,
}

impl From<aws_sdk_autoscaling::types::AutoScalingGroup> for AutoScalingGroup {
    fn from(asg: aws_sdk_autoscaling::types::AutoScalingGroup) -> Self {
        Self {
            name: asg
                .auto_scaling_group_name()
                .unwrap_or_default()
                .to_string(),
            arn: asg.auto_scaling_group_arn().map(|s| s.to_string()),
            status: asg.status().map(|s| s.to_string()),
            desired_capacity: asg.desired_capacity().unwrap_or(0),
            min_size: asg.min_size().unwrap_or(0),
            max_size: asg.max_size().unwrap_or(0),
            default_cooldown: asg.default_cooldown().unwrap_or(0),
            health_check_type: asg.health_check_type().unwrap_or_default().to_string(),
            vpc_zone_identifier: asg.vpc_zone_identifier().map(|s| s.to_string()),
            launch_template: asg.launch_template().map(LaunchTemplateRef::from),
            launch_configuration_name: asg
                .launch_configuration_name()
                .map(|s| s.to_string()),
            instances: asg.instances().iter().map(AsgInstance::from).collect(),
            tags: asg
                .tags()
                .iter()
                .map(|t| Tag {
                    key: t.key().unwrap_or_default().to_string(),
                    value: t.value().unwrap_or_default().to_string(),
                })
                .collect(),
            created_time: asg.created_time().map(|d| d.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ScalingActivity {
    pub activity_id: String,
    pub auto_scaling_group_name: String,
    pub description: Option<String>,
    pub status_code: ScalingActivityStatus,
    pub status_message: Option<String>,
    pub cause: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub progress: Option<i32>,
}

impl From<aws_sdk_autoscaling::types::Activity> for ScalingActivity {
    fn from(a: aws_sdk_autoscaling::types::Activity) -> Self {
        Self {
            activity_id: a.activity_id().unwrap_or_default().to_string(),
            auto_scaling_group_name: a
                .auto_scaling_group_name()
                .unwrap_or_default()
                .to_string(),
            description: a.description().map(|s| s.to_string()),
            status_code: a
                .status_code()
                .map(ScalingActivityStatus::from_sdk)
                .unwrap_or(ScalingActivityStatus::InProgress),
            status_message: a.status_message().map(|s| s.to_string()),
            cause: a.cause().map(|s| s.to_string()),
            start_time: a.start_time().map(|d| d.to_string()),
            end_time: a.end_time().map(|d| d.to_string()),
            progress: a.progress(),
        }
    }
}

// === Unit Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_autoscaling::types::ScalingActivityStatusCode;

    #[test]
    fn test_scaling_activity_status_all_variants() {
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::PendingSpotBidPlacement),
            ScalingActivityStatus::Pending
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(
                &ScalingActivityStatusCode::WaitingForSpotInstanceRequestId
            ),
            ScalingActivityStatus::WaitingForSpotInstanceRequestId
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::WaitingForSpotInstanceId),
            ScalingActivityStatus::WaitingForSpotInstanceId
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::WaitingForInstanceId),
            ScalingActivityStatus::WaitingForInstanceId
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::PreInService),
            ScalingActivityStatus::PreInService
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::InProgress),
            ScalingActivityStatus::InProgress
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(
                &ScalingActivityStatusCode::WaitingForElbConnectionDraining
            ),
            ScalingActivityStatus::WaitingForElbConnectionDraining
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::MidLifecycleAction),
            ScalingActivityStatus::MidLifecycleAction
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::WaitingForInstanceWarmup),
            ScalingActivityStatus::WaitingForInstanceWarmup
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::Successful),
            ScalingActivityStatus::Successful
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::Failed),
            ScalingActivityStatus::Failed
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::Cancelled),
            ScalingActivityStatus::Cancelled
        );
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::WaitingForConnectionDraining),
            ScalingActivityStatus::WaitingForConnectionDraining
        );
        // Unknown variant fallback
        assert_eq!(
            ScalingActivityStatus::from_sdk(&ScalingActivityStatusCode::from("other")),
            ScalingActivityStatus::InProgress
        );
    }

    #[test]
    fn test_launch_template_ref_from_sdk() {
        let lt = aws_sdk_autoscaling::types::LaunchTemplateSpecification::builder()
            .launch_template_id("lt-12345")
            .launch_template_name("my-template")
            .version("$Latest")
            .build();
        let result = LaunchTemplateRef::from(&lt);
        assert_eq!(result.id, Some("lt-12345".to_string()));
        assert_eq!(result.name, Some("my-template".to_string()));
        assert_eq!(result.version, Some("$Latest".to_string()));
    }

    #[test]
    fn test_launch_template_ref_empty() {
        let lt = aws_sdk_autoscaling::types::LaunchTemplateSpecification::builder().build();
        let result = LaunchTemplateRef::from(&lt);
        assert_eq!(result.id, None);
        assert_eq!(result.name, None);
        assert_eq!(result.version, None);
    }

    #[test]
    fn test_asg_instance_from_sdk() {
        let inst = aws_sdk_autoscaling::types::Instance::builder()
            .instance_id("i-12345")
            .availability_zone("us-east-1a")
            .health_status("Healthy")
            .lifecycle_state(aws_sdk_autoscaling::types::LifecycleState::InService)
            .protected_from_scale_in(false)
            .build();
        let result = AsgInstance::from(&inst);
        assert_eq!(result.instance_id, "i-12345");
        assert_eq!(result.availability_zone, "us-east-1a");
        assert_eq!(result.health_status, "Healthy");
        assert_eq!(result.lifecycle_state, "InService");
        assert!(!result.protected_from_scale_in);
    }

    #[test]
    fn test_asg_instance_empty() {
        let inst = aws_sdk_autoscaling::types::Instance::builder().build();
        let result = AsgInstance::from(&inst);
        assert_eq!(result.instance_id, "");
        assert_eq!(result.availability_zone, "");
        assert_eq!(result.health_status, "");
        assert_eq!(result.lifecycle_state, "");
        assert!(!result.protected_from_scale_in);
    }

    #[test]
    fn test_auto_scaling_group_from_sdk() {
        let lt = aws_sdk_autoscaling::types::LaunchTemplateSpecification::builder()
            .launch_template_id("lt-abc")
            .build();
        let asg = aws_sdk_autoscaling::types::AutoScalingGroup::builder()
            .auto_scaling_group_name("my-asg")
            .auto_scaling_group_arn("arn:aws:autoscaling:us-east-1:123:autoScalingGroup:my-asg")
            .desired_capacity(2)
            .min_size(1)
            .max_size(5)
            .default_cooldown(300)
            .health_check_type("EC2")
            .launch_template(lt)
            .build();
        let result = AutoScalingGroup::from(asg);
        assert_eq!(result.name, "my-asg");
        assert!(result.arn.is_some());
        assert_eq!(result.desired_capacity, 2);
        assert_eq!(result.min_size, 1);
        assert_eq!(result.max_size, 5);
        assert_eq!(result.default_cooldown, 300);
        assert_eq!(result.health_check_type, "EC2");
        assert!(result.launch_template.is_some());
        assert_eq!(result.launch_template.unwrap().id, Some("lt-abc".to_string()));
    }

    #[test]
    fn test_auto_scaling_group_from_sdk_empty() {
        let asg = aws_sdk_autoscaling::types::AutoScalingGroup::builder().build();
        let result = AutoScalingGroup::from(asg);
        assert_eq!(result.name, "");
        assert_eq!(result.desired_capacity, 0);
        assert_eq!(result.min_size, 0);
        assert_eq!(result.max_size, 0);
        assert_eq!(result.default_cooldown, 0);
        assert_eq!(result.health_check_type, "");
        assert!(result.instances.is_empty());
        assert!(result.tags.is_empty());
        assert!(result.launch_template.is_none());
    }

    #[test]
    fn test_scaling_activity_from_sdk() {
        let activity = aws_sdk_autoscaling::types::Activity::builder()
            .activity_id("act-123")
            .auto_scaling_group_name("my-asg")
            .status_code(ScalingActivityStatusCode::Successful)
            .cause("At 2024-01-01T00:00:00Z an instance was started.")
            .description("Launching a new EC2 instance")
            .build();
        let result = ScalingActivity::from(activity);
        assert_eq!(result.activity_id, "act-123");
        assert_eq!(result.auto_scaling_group_name, "my-asg");
        assert_eq!(result.status_code, ScalingActivityStatus::Successful);
        assert_eq!(
            result.cause,
            Some("At 2024-01-01T00:00:00Z an instance was started.".to_string())
        );
        assert_eq!(
            result.description,
            Some("Launching a new EC2 instance".to_string())
        );
    }

    #[test]
    fn test_scaling_activity_empty() {
        let activity = aws_sdk_autoscaling::types::Activity::builder().build();
        let result = ScalingActivity::from(activity);
        assert_eq!(result.activity_id, "");
        assert_eq!(result.auto_scaling_group_name, "");
        assert_eq!(result.status_code, ScalingActivityStatus::InProgress);
        assert!(result.description.is_none());
        assert!(result.cause.is_none());
        assert!(result.start_time.is_none());
        assert!(result.end_time.is_none());
        assert!(result.progress.is_none());
    }
}
