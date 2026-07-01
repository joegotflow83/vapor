use async_graphql::{Enum, SimpleObject};

use crate::schema::common::types::Tag;

// === Enums ===

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum EcsLaunchType {
    Ec2,
    Fargate,
    External,
}

impl EcsLaunchType {
    pub fn from_sdk(s: &aws_sdk_ecs::types::LaunchType) -> Self {
        match s {
            aws_sdk_ecs::types::LaunchType::Ec2 => Self::Ec2,
            aws_sdk_ecs::types::LaunchType::Fargate => Self::Fargate,
            aws_sdk_ecs::types::LaunchType::External => Self::External,
            _ => Self::Ec2,
        }
    }
}

// === Output Types ===

#[derive(SimpleObject, Clone)]
pub struct KeyValue {
    pub name: Option<String>,
    pub value: Option<String>,
}

impl From<aws_sdk_ecs::types::KeyValuePair> for KeyValue {
    fn from(kv: aws_sdk_ecs::types::KeyValuePair) -> Self {
        Self {
            name: kv.name().map(|s| s.to_string()),
            value: kv.value().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct PortMapping {
    pub container_port: Option<i32>,
    pub host_port: Option<i32>,
    pub protocol: Option<String>,
}

impl From<aws_sdk_ecs::types::PortMapping> for PortMapping {
    fn from(pm: aws_sdk_ecs::types::PortMapping) -> Self {
        Self {
            container_port: pm.container_port(),
            host_port: pm.host_port(),
            protocol: pm.protocol().map(|p| p.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ContainerDefinition {
    pub name: Option<String>,
    pub image: Option<String>,
    pub cpu: Option<i32>,
    pub memory: Option<i32>,
    pub memory_reservation: Option<i32>,
    pub essential: Option<bool>,
    pub environment: Vec<KeyValue>,
    pub port_mappings: Vec<PortMapping>,
}

impl From<aws_sdk_ecs::types::ContainerDefinition> for ContainerDefinition {
    fn from(cd: aws_sdk_ecs::types::ContainerDefinition) -> Self {
        Self {
            name: cd.name().map(|s| s.to_string()),
            image: cd.image().map(|s| s.to_string()),
            cpu: Some(cd.cpu()).filter(|&c| c != 0),
            memory: cd.memory(),
            memory_reservation: cd.memory_reservation(),
            essential: cd.essential(),
            environment: cd
                .environment()
                .iter()
                .map(|kv| KeyValue::from(kv.clone()))
                .collect(),
            port_mappings: cd
                .port_mappings()
                .iter()
                .map(|p| PortMapping::from(p.clone()))
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TaskDefinition {
    pub family: Option<String>,
    pub arn: Option<String>,
    pub revision: Option<i32>,
    pub status: Option<String>,
    pub network_mode: Option<String>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub requires_compatibilities: Vec<String>,
    pub container_definitions: Vec<ContainerDefinition>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ecs::types::TaskDefinition> for TaskDefinition {
    fn from(td: aws_sdk_ecs::types::TaskDefinition) -> Self {
        Self {
            family: td.family().map(|s| s.to_string()),
            arn: td.task_definition_arn().map(|s| s.to_string()),
            revision: Some(td.revision()),
            status: td.status().map(|s| s.as_str().to_string()),
            network_mode: td.network_mode().map(|m| m.as_str().to_string()),
            cpu: td.cpu().map(|s| s.to_string()),
            memory: td.memory().map(|s| s.to_string()),
            requires_compatibilities: td
                .requires_compatibilities()
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
            container_definitions: td
                .container_definitions()
                .iter()
                .map(|c| ContainerDefinition::from(c.clone()))
                .collect(),
            // `TaskDefinition` carries no tags; DescribeTaskDefinition returns
            // them as a sibling field, not on the definition itself.
            tags: Vec::new(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Container {
    pub container_arn: Option<String>,
    pub name: Option<String>,
    pub image: Option<String>,
    pub last_status: Option<String>,
    pub exit_code: Option<i32>,
    pub reason: Option<String>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

impl From<aws_sdk_ecs::types::Container> for Container {
    fn from(c: aws_sdk_ecs::types::Container) -> Self {
        Self {
            container_arn: c.container_arn().map(|s| s.to_string()),
            name: c.name().map(|s| s.to_string()),
            image: c.image().map(|s| s.to_string()),
            last_status: c.last_status().map(|s| s.to_string()),
            exit_code: c.exit_code(),
            reason: c.reason().map(|s| s.to_string()),
            cpu: c.cpu().map(|s| s.to_string()),
            memory: c.memory().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Task {
    pub task_arn: Option<String>,
    pub cluster_arn: Option<String>,
    pub task_definition_arn: Option<String>,
    pub last_status: Option<String>,
    pub desired_status: Option<String>,
    pub launch_type: Option<EcsLaunchType>,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
    pub stopped_reason: Option<String>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub tags: Vec<Tag>,
    pub containers: Vec<Container>,
}

impl From<aws_sdk_ecs::types::Task> for Task {
    fn from(t: aws_sdk_ecs::types::Task) -> Self {
        Self {
            task_arn: t.task_arn().map(|s| s.to_string()),
            cluster_arn: t.cluster_arn().map(|s| s.to_string()),
            task_definition_arn: t.task_definition_arn().map(|s| s.to_string()),
            last_status: t.last_status().map(|s| s.to_string()),
            desired_status: t.desired_status().map(|s| s.to_string()),
            launch_type: t.launch_type().map(EcsLaunchType::from_sdk),
            started_at: t.started_at().map(|d| d.to_string()),
            stopped_at: t.stopped_at().map(|d| d.to_string()),
            stopped_reason: t.stopped_reason().map(|s| s.to_string()),
            cpu: t.cpu().map(|s| s.to_string()),
            memory: t.memory().map(|s| s.to_string()),
            tags: t
                .tags()
                .iter()
                .map(|tag| Tag {
                    key: tag.key().unwrap_or_default().to_string(),
                    value: tag.value().unwrap_or_default().to_string(),
                })
                .collect(),
            containers: t
                .containers()
                .iter()
                .map(|c| Container::from(c.clone()))
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AwsVpcConfiguration {
    pub subnets: Vec<String>,
    pub security_groups: Vec<String>,
    pub assign_public_ip: Option<String>,
}

impl From<aws_sdk_ecs::types::AwsVpcConfiguration> for AwsVpcConfiguration {
    fn from(vpc: aws_sdk_ecs::types::AwsVpcConfiguration) -> Self {
        Self {
            subnets: vpc.subnets().iter().map(|s| s.to_string()).collect(),
            security_groups: vpc
                .security_groups()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            assign_public_ip: vpc.assign_public_ip().map(|a| a.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ServiceLoadBalancer {
    pub target_group_arn: Option<String>,
    pub load_balancer_name: Option<String>,
    pub container_name: Option<String>,
    pub container_port: Option<i32>,
}

impl From<aws_sdk_ecs::types::LoadBalancer> for ServiceLoadBalancer {
    fn from(lb: aws_sdk_ecs::types::LoadBalancer) -> Self {
        Self {
            target_group_arn: lb.target_group_arn().map(|s| s.to_string()),
            load_balancer_name: lb.load_balancer_name().map(|s| s.to_string()),
            container_name: lb.container_name().map(|s| s.to_string()),
            container_port: lb.container_port(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Service {
    pub name: String,
    pub arn: Option<String>,
    pub cluster_arn: Option<String>,
    pub status: Option<String>,
    pub desired_count: i32,
    pub running_count: i32,
    pub pending_count: i32,
    pub launch_type: Option<EcsLaunchType>,
    pub task_definition: Option<String>,
    pub created_at: Option<String>,
    pub load_balancers: Vec<ServiceLoadBalancer>,
    pub network_configuration: Option<AwsVpcConfiguration>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ecs::types::Service> for Service {
    fn from(s: aws_sdk_ecs::types::Service) -> Self {
        Self {
            name: s.service_name().unwrap_or_default().to_string(),
            arn: s.service_arn().map(|v| v.to_string()),
            cluster_arn: s.cluster_arn().map(|v| v.to_string()),
            status: s.status().map(|v| v.to_string()),
            desired_count: s.desired_count(),
            running_count: s.running_count(),
            pending_count: s.pending_count(),
            launch_type: s.launch_type().map(EcsLaunchType::from_sdk),
            task_definition: s.task_definition().map(|v| v.to_string()),
            created_at: s.created_at().map(|d| d.to_string()),
            load_balancers: s
                .load_balancers()
                .iter()
                .map(|lb| ServiceLoadBalancer::from(lb.clone()))
                .collect(),
            network_configuration: s
                .network_configuration()
                .and_then(|n| n.awsvpc_configuration())
                .map(|c| AwsVpcConfiguration::from(c.clone())),
            tags: s
                .tags()
                .iter()
                .map(|t| Tag {
                    key: t.key().unwrap_or_default().to_string(),
                    value: t.value().unwrap_or_default().to_string(),
                })
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Cluster {
    pub name: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub running_tasks_count: Option<i32>,
    pub pending_tasks_count: Option<i32>,
    pub active_services_count: Option<i32>,
    pub registered_container_instances_count: Option<i32>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ecs::types::Cluster> for Cluster {
    fn from(c: aws_sdk_ecs::types::Cluster) -> Self {
        Self {
            name: c.cluster_name().unwrap_or_default().to_string(),
            arn: c.cluster_arn().map(|s| s.to_string()),
            status: c.status().map(|s| s.to_string()),
            running_tasks_count: Some(c.running_tasks_count()),
            pending_tasks_count: Some(c.pending_tasks_count()),
            active_services_count: Some(c.active_services_count()),
            registered_container_instances_count: Some(
                c.registered_container_instances_count(),
            ),
            tags: c
                .tags()
                .iter()
                .map(|t| Tag {
                    key: t.key().unwrap_or_default().to_string(),
                    value: t.value().unwrap_or_default().to_string(),
                })
                .collect(),
        }
    }
}

// === Unit Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecs_launch_type_all_variants() {
        assert_eq!(
            EcsLaunchType::from_sdk(&aws_sdk_ecs::types::LaunchType::Ec2),
            EcsLaunchType::Ec2
        );
        assert_eq!(
            EcsLaunchType::from_sdk(&aws_sdk_ecs::types::LaunchType::Fargate),
            EcsLaunchType::Fargate
        );
        assert_eq!(
            EcsLaunchType::from_sdk(&aws_sdk_ecs::types::LaunchType::External),
            EcsLaunchType::External
        );
        // Unknown variant fallback
        assert_eq!(
            EcsLaunchType::from_sdk(&aws_sdk_ecs::types::LaunchType::from("other")),
            EcsLaunchType::Ec2
        );
    }

    #[test]
    fn test_key_value_from_sdk() {
        let kv = aws_sdk_ecs::types::KeyValuePair::builder()
            .name("MY_VAR")
            .value("my_value")
            .build();
        let result = KeyValue::from(kv);
        assert_eq!(result.name, Some("MY_VAR".to_string()));
        assert_eq!(result.value, Some("my_value".to_string()));
    }

    #[test]
    fn test_port_mapping_from_sdk() {
        let pm = aws_sdk_ecs::types::PortMapping::builder()
            .container_port(8080)
            .host_port(8080)
            .protocol(aws_sdk_ecs::types::TransportProtocol::Tcp)
            .build();
        let result = PortMapping::from(pm);
        assert_eq!(result.container_port, Some(8080));
        assert_eq!(result.host_port, Some(8080));
        assert_eq!(result.protocol, Some("tcp".to_string()));
    }

    #[test]
    fn test_container_definition_from_sdk() {
        let kv = aws_sdk_ecs::types::KeyValuePair::builder()
            .name("ENV_VAR")
            .value("env_val")
            .build();
        let pm = aws_sdk_ecs::types::PortMapping::builder()
            .container_port(80)
            .build();
        let cd = aws_sdk_ecs::types::ContainerDefinition::builder()
            .name("my-container")
            .image("nginx:latest")
            .cpu(256)
            .memory(512)
            .essential(true)
            .environment(kv)
            .port_mappings(pm)
            .build();
        let result = ContainerDefinition::from(cd);
        assert_eq!(result.name, Some("my-container".to_string()));
        assert_eq!(result.image, Some("nginx:latest".to_string()));
        assert_eq!(result.cpu, Some(256));
        assert_eq!(result.memory, Some(512));
        assert_eq!(result.essential, Some(true));
        assert_eq!(result.environment.len(), 1);
        assert_eq!(result.environment[0].name, Some("ENV_VAR".to_string()));
        assert_eq!(result.port_mappings.len(), 1);
        assert_eq!(result.port_mappings[0].container_port, Some(80));
    }

    #[test]
    fn test_container_definition_from_sdk_empty() {
        let cd = aws_sdk_ecs::types::ContainerDefinition::builder().build();
        let result = ContainerDefinition::from(cd);
        assert_eq!(result.name, None);
        assert_eq!(result.image, None);
        assert_eq!(result.cpu, None);
        assert_eq!(result.memory, None);
        assert_eq!(result.essential, None);
        assert!(result.environment.is_empty());
        assert!(result.port_mappings.is_empty());
    }

    #[test]
    fn test_container_from_sdk() {
        // Running container
        let running = aws_sdk_ecs::types::Container::builder()
            .container_arn("arn:aws:ecs:us-east-1:123:container/abc")
            .name("my-container")
            .image("nginx:latest")
            .last_status("RUNNING")
            .build();
        let result = Container::from(running);
        assert_eq!(
            result.container_arn,
            Some("arn:aws:ecs:us-east-1:123:container/abc".to_string())
        );
        assert_eq!(result.name, Some("my-container".to_string()));
        assert_eq!(result.last_status, Some("RUNNING".to_string()));
        assert_eq!(result.exit_code, None);

        // Stopped container with exit code
        let stopped = aws_sdk_ecs::types::Container::builder()
            .name("my-container")
            .last_status("STOPPED")
            .exit_code(1)
            .reason("Essential container exited")
            .build();
        let result = Container::from(stopped);
        assert_eq!(result.last_status, Some("STOPPED".to_string()));
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(
            result.reason,
            Some("Essential container exited".to_string())
        );
    }

    #[test]
    fn test_task_from_sdk() {
        let tag = aws_sdk_ecs::types::Tag::builder()
            .key("Env")
            .value("prod")
            .build();
        let container = aws_sdk_ecs::types::Container::builder()
            .name("web")
            .last_status("RUNNING")
            .build();
        let task = aws_sdk_ecs::types::Task::builder()
            .task_arn("arn:aws:ecs:us-east-1:123:task/abc")
            .cluster_arn("arn:aws:ecs:us-east-1:123:cluster/my-cluster")
            .task_definition_arn("arn:aws:ecs:us-east-1:123:task-definition/my-td:1")
            .last_status("RUNNING")
            .desired_status("RUNNING")
            .launch_type(aws_sdk_ecs::types::LaunchType::Fargate)
            .cpu("256")
            .memory("512")
            .tags(tag)
            .containers(container)
            .build();
        let result = Task::from(task);
        assert_eq!(
            result.task_arn,
            Some("arn:aws:ecs:us-east-1:123:task/abc".to_string())
        );
        assert_eq!(result.last_status, Some("RUNNING".to_string()));
        assert_eq!(result.launch_type, Some(EcsLaunchType::Fargate));
        assert_eq!(result.cpu, Some("256".to_string()));
        assert_eq!(result.memory, Some("512".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Env");
        assert_eq!(result.tags[0].value, "prod");
        assert_eq!(result.containers.len(), 1);
        assert_eq!(result.containers[0].name, Some("web".to_string()));
    }

    #[test]
    fn test_task_from_sdk_empty() {
        let task = aws_sdk_ecs::types::Task::builder().build();
        let result = Task::from(task);
        assert_eq!(result.task_arn, None);
        assert_eq!(result.cluster_arn, None);
        assert_eq!(result.last_status, None);
        assert_eq!(result.launch_type, None);
        assert!(result.tags.is_empty());
        assert!(result.containers.is_empty());
    }

    #[test]
    fn test_aws_vpc_configuration_from_sdk() {
        let vpc = aws_sdk_ecs::types::AwsVpcConfiguration::builder()
            .subnets("subnet-abc123")
            .subnets("subnet-def456")
            .security_groups("sg-xyz789")
            .assign_public_ip(aws_sdk_ecs::types::AssignPublicIp::Enabled)
            .build()
            .unwrap();
        let result = AwsVpcConfiguration::from(vpc);
        assert_eq!(result.subnets.len(), 2);
        assert_eq!(result.subnets[0], "subnet-abc123");
        assert_eq!(result.security_groups.len(), 1);
        assert_eq!(result.security_groups[0], "sg-xyz789");
        assert_eq!(result.assign_public_ip, Some("ENABLED".to_string()));
    }

    #[test]
    fn test_service_load_balancer_from_sdk() {
        let lb = aws_sdk_ecs::types::LoadBalancer::builder()
            .target_group_arn("arn:aws:elasticloadbalancing:us-east-1:123:targetgroup/my-tg/abc")
            .container_name("web")
            .container_port(80)
            .build();
        let result = ServiceLoadBalancer::from(lb);
        assert_eq!(
            result.target_group_arn,
            Some(
                "arn:aws:elasticloadbalancing:us-east-1:123:targetgroup/my-tg/abc".to_string()
            )
        );
        assert_eq!(result.container_name, Some("web".to_string()));
        assert_eq!(result.container_port, Some(80));
    }

    #[test]
    fn test_service_from_sdk() {
        let lb = aws_sdk_ecs::types::LoadBalancer::builder()
            .target_group_arn("arn:aws:elasticloadbalancing:us-east-1:123:targetgroup/my-tg/abc")
            .container_name("web")
            .container_port(80)
            .build();
        let vpc_config = aws_sdk_ecs::types::AwsVpcConfiguration::builder()
            .subnets("subnet-abc123")
            .build()
            .unwrap();
        let net_config = aws_sdk_ecs::types::NetworkConfiguration::builder()
            .awsvpc_configuration(vpc_config)
            .build();
        let tag = aws_sdk_ecs::types::Tag::builder()
            .key("Team")
            .value("platform")
            .build();
        let service = aws_sdk_ecs::types::Service::builder()
            .service_name("my-service")
            .service_arn("arn:aws:ecs:us-east-1:123:service/my-cluster/my-service")
            .cluster_arn("arn:aws:ecs:us-east-1:123:cluster/my-cluster")
            .status("ACTIVE")
            .desired_count(2)
            .running_count(2)
            .pending_count(0)
            .launch_type(aws_sdk_ecs::types::LaunchType::Fargate)
            .task_definition("arn:aws:ecs:us-east-1:123:task-definition/my-td:1")
            .load_balancers(lb)
            .network_configuration(net_config)
            .tags(tag)
            .build();
        let result = Service::from(service);
        assert_eq!(result.name, "my-service");
        assert_eq!(
            result.arn,
            Some("arn:aws:ecs:us-east-1:123:service/my-cluster/my-service".to_string())
        );
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.desired_count, 2);
        assert_eq!(result.running_count, 2);
        assert_eq!(result.pending_count, 0);
        assert_eq!(result.launch_type, Some(EcsLaunchType::Fargate));
        assert_eq!(result.load_balancers.len(), 1);
        assert!(result.network_configuration.is_some());
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Team");
    }

    #[test]
    fn test_service_from_sdk_empty() {
        let service = aws_sdk_ecs::types::Service::builder().build();
        let result = Service::from(service);
        assert_eq!(result.name, "");
        assert_eq!(result.arn, None);
        assert_eq!(result.status, None);
        assert_eq!(result.desired_count, 0);
        assert_eq!(result.running_count, 0);
        assert_eq!(result.pending_count, 0);
        assert_eq!(result.launch_type, None);
        assert!(result.load_balancers.is_empty());
        assert!(result.network_configuration.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_cluster_from_sdk() {
        let tag = aws_sdk_ecs::types::Tag::builder()
            .key("Name")
            .value("production")
            .build();
        let cluster = aws_sdk_ecs::types::Cluster::builder()
            .cluster_name("my-cluster")
            .cluster_arn("arn:aws:ecs:us-east-1:123:cluster/my-cluster")
            .status("ACTIVE")
            .running_tasks_count(5)
            .pending_tasks_count(0)
            .active_services_count(3)
            .registered_container_instances_count(10)
            .tags(tag)
            .build();
        let result = Cluster::from(cluster);
        assert_eq!(result.name, "my-cluster");
        assert_eq!(
            result.arn,
            Some("arn:aws:ecs:us-east-1:123:cluster/my-cluster".to_string())
        );
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.running_tasks_count, Some(5));
        assert_eq!(result.pending_tasks_count, Some(0));
        assert_eq!(result.active_services_count, Some(3));
        assert_eq!(result.registered_container_instances_count, Some(10));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Name");
        assert_eq!(result.tags[0].value, "production");
    }

    #[test]
    fn test_cluster_from_sdk_empty() {
        let cluster = aws_sdk_ecs::types::Cluster::builder().build();
        let result = Cluster::from(cluster);
        assert_eq!(result.name, "");
        assert_eq!(result.arn, None);
        assert_eq!(result.status, None);
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_task_definition_from_sdk() {
        let container = aws_sdk_ecs::types::ContainerDefinition::builder()
            .name("web")
            .image("nginx:latest")
            .cpu(256)
            .memory(512)
            .essential(true)
            .build();
        let td = aws_sdk_ecs::types::TaskDefinition::builder()
            .family("my-family")
            .task_definition_arn("arn:aws:ecs:us-east-1:123:task-definition/my-family:1")
            .revision(1)
            .status(aws_sdk_ecs::types::TaskDefinitionStatus::Active)
            .network_mode(aws_sdk_ecs::types::NetworkMode::Awsvpc)
            .cpu("256")
            .memory("512")
            .requires_compatibilities(aws_sdk_ecs::types::Compatibility::Fargate)
            .container_definitions(container)
            .build();
        let result = TaskDefinition::from(td);
        assert_eq!(result.family, Some("my-family".to_string()));
        assert_eq!(
            result.arn,
            Some("arn:aws:ecs:us-east-1:123:task-definition/my-family:1".to_string())
        );
        assert_eq!(result.revision, Some(1));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.network_mode, Some("awsvpc".to_string()));
        assert_eq!(result.cpu, Some("256".to_string()));
        assert_eq!(result.memory, Some("512".to_string()));
        assert_eq!(result.requires_compatibilities.len(), 1);
        assert_eq!(result.requires_compatibilities[0], "FARGATE");
        assert_eq!(result.container_definitions.len(), 1);
        // `TaskDefinition` carries no tags (returned separately by the API).
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_task_definition_from_sdk_empty() {
        let td = aws_sdk_ecs::types::TaskDefinition::builder().build();
        let result = TaskDefinition::from(td);
        assert_eq!(result.family, None);
        assert_eq!(result.arn, None);
        assert_eq!(result.status, None);
        assert!(result.requires_compatibilities.is_empty());
        assert!(result.container_definitions.is_empty());
        assert!(result.tags.is_empty());
    }
}
