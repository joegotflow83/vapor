use async_graphql::SimpleObject;

use crate::schema::ec2::types::Tag;

fn tags_from_map(map: Option<&std::collections::HashMap<String, String>>) -> Vec<Tag> {
    map.into_iter()
        .flat_map(|m| m.iter().map(|(k, v)| Tag { key: k.clone(), value: v.clone() }))
        .collect()
}

fn labels_from_map(map: Option<&std::collections::HashMap<String, String>>) -> Vec<Tag> {
    tags_from_map(map)
}

/// Kubernetes network configuration for an EKS cluster.
#[derive(SimpleObject, Clone)]
pub struct EksKubernetesNetworkConfig {
    pub service_ipv4_cidr: Option<String>,
    pub service_ipv6_cidr: Option<String>,
    /// ipv4 | ipv6
    pub ip_family: Option<String>,
}

impl From<&aws_sdk_eks::types::KubernetesNetworkConfigResponse>
    for EksKubernetesNetworkConfig
{
    fn from(knc: &aws_sdk_eks::types::KubernetesNetworkConfigResponse) -> Self {
        Self {
            service_ipv4_cidr: knc.service_ipv4_cidr().map(|s| s.to_string()),
            service_ipv6_cidr: knc.service_ipv6_cidr().map(|s| s.to_string()),
            ip_family: knc.ip_family().map(|f| f.as_str().to_string()),
        }
    }
}

/// VPC configuration for an EKS cluster.
#[derive(SimpleObject, Clone)]
pub struct EksVpcConfig {
    pub subnet_ids: Vec<String>,
    pub security_group_ids: Vec<String>,
    pub cluster_security_group_id: Option<String>,
    pub vpc_id: Option<String>,
    pub endpoint_public_access: Option<bool>,
    pub endpoint_private_access: Option<bool>,
    pub public_access_cidrs: Vec<String>,
}

impl From<&aws_sdk_eks::types::VpcConfigResponse> for EksVpcConfig {
    fn from(vpc: &aws_sdk_eks::types::VpcConfigResponse) -> Self {
        Self {
            subnet_ids: vpc.subnet_ids().iter().map(|s| s.to_string()).collect(),
            security_group_ids: vpc.security_group_ids().iter().map(|s| s.to_string()).collect(),
            cluster_security_group_id: vpc.cluster_security_group_id().map(|s| s.to_string()),
            vpc_id: vpc.vpc_id().map(|s| s.to_string()),
            endpoint_public_access: Some(vpc.endpoint_public_access()),
            endpoint_private_access: Some(vpc.endpoint_private_access()),
            public_access_cidrs: vpc.public_access_cidrs().iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// A single log type configuration for an EKS cluster.
#[derive(SimpleObject, Clone)]
pub struct EksLogSetup {
    /// api | audit | authenticator | controllerManager | scheduler
    pub types: Vec<String>,
    pub enabled: Option<bool>,
}

impl From<&aws_sdk_eks::types::LogSetup> for EksLogSetup {
    fn from(ls: &aws_sdk_eks::types::LogSetup) -> Self {
        Self {
            types: ls.types().iter().map(|t| t.as_str().to_string()).collect(),
            enabled: ls.enabled(),
        }
    }
}

/// Logging configuration for an EKS cluster.
#[derive(SimpleObject, Clone)]
pub struct EksLogging {
    pub cluster_logging: Vec<EksLogSetup>,
}

impl From<&aws_sdk_eks::types::Logging> for EksLogging {
    fn from(logging: &aws_sdk_eks::types::Logging) -> Self {
        Self {
            cluster_logging: logging.cluster_logging().iter().map(EksLogSetup::from).collect(),
        }
    }
}

/// An Amazon EKS cluster.
#[derive(SimpleObject, Clone)]
pub struct EksCluster {
    pub name: String,
    pub arn: Option<String>,
    /// CREATING | ACTIVE | DELETING | FAILED | UPDATING | PENDING
    pub status: Option<String>,
    pub version: Option<String>,
    pub endpoint: Option<String>,
    pub role_arn: Option<String>,
    pub kubernetes_network_config: Option<EksKubernetesNetworkConfig>,
    pub resources_vpc_config: Option<EksVpcConfig>,
    pub logging: Option<EksLogging>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_eks::types::Cluster> for EksCluster {
    fn from(c: aws_sdk_eks::types::Cluster) -> Self {
        Self {
            name: c.name().map(|s| s.to_string()).unwrap_or_default(),
            arn: c.arn().map(|s| s.to_string()),
            status: c.status().map(|s| s.as_str().to_string()),
            version: c.version().map(|s| s.to_string()),
            endpoint: c.endpoint().map(|s| s.to_string()),
            role_arn: c.role_arn().map(|s| s.to_string()),
            kubernetes_network_config: c
                .kubernetes_network_config()
                .map(EksKubernetesNetworkConfig::from),
            resources_vpc_config: c.resources_vpc_config().map(EksVpcConfig::from),
            logging: c.logging().map(EksLogging::from),
            tags: tags_from_map(c.tags()),
        }
    }
}

/// Auto-scaling configuration for an EKS nodegroup.
#[derive(SimpleObject, Clone)]
pub struct EksScalingConfig {
    pub min_size: Option<i32>,
    pub max_size: Option<i32>,
    pub desired_size: Option<i32>,
}

impl From<&aws_sdk_eks::types::NodegroupScalingConfig> for EksScalingConfig {
    fn from(sc: &aws_sdk_eks::types::NodegroupScalingConfig) -> Self {
        Self {
            min_size: sc.min_size(),
            max_size: sc.max_size(),
            desired_size: sc.desired_size(),
        }
    }
}

/// An EKS managed nodegroup.
#[derive(SimpleObject, Clone)]
pub struct EksNodegroup {
    pub name: String,
    pub arn: Option<String>,
    pub cluster_name: Option<String>,
    /// CREATING | ACTIVE | UPDATING | DELETING | CREATE_FAILED | DELETE_FAILED | DEGRADED
    pub status: Option<String>,
    /// ON_DEMAND | SPOT
    pub capacity_type: Option<String>,
    pub ami_type: Option<String>,
    pub release_version: Option<String>,
    pub instance_types: Vec<String>,
    pub disk_size: Option<i32>,
    pub scaling_config: Option<EksScalingConfig>,
    pub subnet_ids: Vec<String>,
    pub node_role: Option<String>,
    /// Kubernetes labels as key/value pairs.
    pub labels: Vec<Tag>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_eks::types::Nodegroup> for EksNodegroup {
    fn from(ng: aws_sdk_eks::types::Nodegroup) -> Self {
        Self {
            name: ng.nodegroup_name().map(|s| s.to_string()).unwrap_or_default(),
            arn: ng.nodegroup_arn().map(|s| s.to_string()),
            cluster_name: ng.cluster_name().map(|s| s.to_string()),
            status: ng.status().map(|s| s.as_str().to_string()),
            capacity_type: ng.capacity_type().map(|s| s.as_str().to_string()),
            ami_type: ng.ami_type().map(|s| s.as_str().to_string()),
            release_version: ng.release_version().map(|s| s.to_string()),
            instance_types: ng.instance_types().iter().map(|s| s.to_string()).collect(),
            disk_size: ng.disk_size(),
            scaling_config: ng.scaling_config().map(EksScalingConfig::from),
            subnet_ids: ng.subnets().iter().map(|s| s.to_string()).collect(),
            node_role: ng.node_role().map(|s| s.to_string()),
            labels: labels_from_map(ng.labels()),
            created_at: ng.created_at().map(|d| d.to_string()),
            modified_at: ng.modified_at().map(|d| d.to_string()),
            tags: tags_from_map(ng.tags()),
        }
    }
}

/// A Fargate selector for an EKS Fargate profile.
#[derive(SimpleObject, Clone)]
pub struct EksFargateSelector {
    pub namespace: Option<String>,
    /// Kubernetes labels as key/value pairs.
    pub labels: Vec<Tag>,
}

impl From<&aws_sdk_eks::types::FargateProfileSelector> for EksFargateSelector {
    fn from(sel: &aws_sdk_eks::types::FargateProfileSelector) -> Self {
        Self {
            namespace: sel.namespace().map(|s| s.to_string()),
            labels: labels_from_map(sel.labels()),
        }
    }
}

/// An EKS Fargate profile.
#[derive(SimpleObject, Clone)]
pub struct EksFargateProfile {
    pub name: String,
    pub arn: Option<String>,
    pub cluster_name: Option<String>,
    pub status: Option<String>,
    pub pod_execution_role_arn: Option<String>,
    pub subnet_ids: Vec<String>,
    pub selectors: Vec<EksFargateSelector>,
    pub created_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_eks::types::FargateProfile> for EksFargateProfile {
    fn from(fp: aws_sdk_eks::types::FargateProfile) -> Self {
        Self {
            name: fp.fargate_profile_name().map(|s| s.to_string()).unwrap_or_default(),
            arn: fp.fargate_profile_arn().map(|s| s.to_string()),
            cluster_name: fp.cluster_name().map(|s| s.to_string()),
            status: fp.status().map(|s| s.as_str().to_string()),
            pod_execution_role_arn: fp.pod_execution_role_arn().map(|s| s.to_string()),
            subnet_ids: fp.subnets().iter().map(|s| s.to_string()).collect(),
            selectors: fp.selectors().iter().map(EksFargateSelector::from).collect(),
            created_at: fp.created_at().map(|d| d.to_string()),
            tags: tags_from_map(fp.tags()),
        }
    }
}

/// An EKS add-on.
#[derive(SimpleObject, Clone)]
pub struct EksAddon {
    pub name: String,
    pub arn: Option<String>,
    pub cluster_name: Option<String>,
    pub status: Option<String>,
    pub addon_version: Option<String>,
    pub service_account_role_arn: Option<String>,
    pub marketplace_version: Option<String>,
    pub configuration_values: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_eks::types::Addon> for EksAddon {
    fn from(addon: aws_sdk_eks::types::Addon) -> Self {
        Self {
            name: addon.addon_name().map(|s| s.to_string()).unwrap_or_default(),
            arn: addon.addon_arn().map(|s| s.to_string()),
            cluster_name: addon.cluster_name().map(|s| s.to_string()),
            status: addon.status().map(|s| s.as_str().to_string()),
            addon_version: addon.addon_version().map(|s| s.to_string()),
            service_account_role_arn: addon.service_account_role_arn().map(|s| s.to_string()),
            // No `marketplace_version` accessor; the SDK exposes marketplace
            // details via `marketplace_information` (product id/url only).
            marketplace_version: None,
            configuration_values: addon.configuration_values().map(|s| s.to_string()),
            created_at: addon.created_at().map(|d| d.to_string()),
            modified_at: addon.modified_at().map(|d| d.to_string()),
            tags: tags_from_map(addon.tags()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eks_kubernetes_network_config_from() {
        let knc = aws_sdk_eks::types::KubernetesNetworkConfigResponse::builder()
            .service_ipv4_cidr("10.100.0.0/16")
            .build();
        let result = EksKubernetesNetworkConfig::from(&knc);
        assert_eq!(result.service_ipv4_cidr, Some("10.100.0.0/16".to_string()));
        assert_eq!(result.service_ipv6_cidr, None);
        assert_eq!(result.ip_family, None);
    }

    #[test]
    fn test_eks_kubernetes_network_config_empty() {
        let knc = aws_sdk_eks::types::KubernetesNetworkConfigResponse::builder().build();
        let result = EksKubernetesNetworkConfig::from(&knc);
        assert_eq!(result.service_ipv4_cidr, None);
        assert_eq!(result.service_ipv6_cidr, None);
        assert_eq!(result.ip_family, None);
    }

    #[test]
    fn test_eks_vpc_config_from() {
        let vpc = aws_sdk_eks::types::VpcConfigResponse::builder()
            .subnet_ids("subnet-aaa")
            .subnet_ids("subnet-bbb")
            .security_group_ids("sg-001")
            .cluster_security_group_id("sg-cluster")
            .vpc_id("vpc-12345")
            .endpoint_public_access(true)
            .endpoint_private_access(false)
            .public_access_cidrs("0.0.0.0/0")
            .build();
        let result = EksVpcConfig::from(&vpc);
        assert_eq!(result.subnet_ids.len(), 2);
        assert!(result.subnet_ids.contains(&"subnet-aaa".to_string()));
        assert_eq!(result.security_group_ids, vec!["sg-001".to_string()]);
        assert_eq!(result.cluster_security_group_id, Some("sg-cluster".to_string()));
        assert_eq!(result.vpc_id, Some("vpc-12345".to_string()));
        assert_eq!(result.endpoint_public_access, Some(true));
        assert_eq!(result.endpoint_private_access, Some(false));
        assert_eq!(result.public_access_cidrs, vec!["0.0.0.0/0".to_string()]);
    }

    #[test]
    fn test_eks_vpc_config_empty() {
        let vpc = aws_sdk_eks::types::VpcConfigResponse::builder().build();
        let result = EksVpcConfig::from(&vpc);
        assert!(result.subnet_ids.is_empty());
        assert!(result.security_group_ids.is_empty());
        assert_eq!(result.cluster_security_group_id, None);
        assert_eq!(result.vpc_id, None);
        // The SDK accessors return a plain `bool` (default false), so an unset
        // config maps to Some(false) rather than None.
        assert_eq!(result.endpoint_public_access, Some(false));
        assert_eq!(result.endpoint_private_access, Some(false));
        assert!(result.public_access_cidrs.is_empty());
    }

    #[test]
    fn test_eks_log_setup_from() {
        let ls = aws_sdk_eks::types::LogSetup::builder()
            .types(aws_sdk_eks::types::LogType::Api)
            .types(aws_sdk_eks::types::LogType::Audit)
            .enabled(true)
            .build();
        let result = EksLogSetup::from(&ls);
        assert_eq!(result.types.len(), 2);
        assert!(result.types.contains(&"api".to_string()));
        assert!(result.types.contains(&"audit".to_string()));
        assert_eq!(result.enabled, Some(true));
    }

    #[test]
    fn test_eks_scaling_config_from() {
        let sc = aws_sdk_eks::types::NodegroupScalingConfig::builder()
            .min_size(1)
            .max_size(5)
            .desired_size(3)
            .build();
        let result = EksScalingConfig::from(&sc);
        assert_eq!(result.min_size, Some(1));
        assert_eq!(result.max_size, Some(5));
        assert_eq!(result.desired_size, Some(3));
    }

    #[test]
    fn test_eks_fargate_selector_from() {
        let mut labels = std::collections::HashMap::new();
        labels.insert("app".to_string(), "nginx".to_string());
        let sel = aws_sdk_eks::types::FargateProfileSelector::builder()
            .namespace("default")
            .set_labels(Some(labels))
            .build();
        let result = EksFargateSelector::from(&sel);
        assert_eq!(result.namespace, Some("default".to_string()));
        assert_eq!(result.labels.len(), 1);
        assert_eq!(result.labels[0].key, "app");
        assert_eq!(result.labels[0].value, "nginx");
    }

    #[test]
    fn test_eks_cluster_from() {
        let cluster = aws_sdk_eks::types::Cluster::builder()
            .name("my-cluster")
            .arn("arn:aws:eks:us-east-1:123456789:cluster/my-cluster")
            .status(aws_sdk_eks::types::ClusterStatus::Active)
            .version("1.28")
            .endpoint("https://ABCD.gr7.us-east-1.eks.amazonaws.com")
            .role_arn("arn:aws:iam::123456789:role/eks-role")
            .build();
        let result = EksCluster::from(cluster);
        assert_eq!(result.name, "my-cluster");
        assert_eq!(
            result.arn,
            Some("arn:aws:eks:us-east-1:123456789:cluster/my-cluster".to_string())
        );
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.version, Some("1.28".to_string()));
        assert_eq!(
            result.endpoint,
            Some("https://ABCD.gr7.us-east-1.eks.amazonaws.com".to_string())
        );
        assert_eq!(
            result.role_arn,
            Some("arn:aws:iam::123456789:role/eks-role".to_string())
        );
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_eks_cluster_empty() {
        let cluster = aws_sdk_eks::types::Cluster::builder().build();
        let result = EksCluster::from(cluster);
        assert_eq!(result.name, "");
        assert_eq!(result.arn, None);
        assert_eq!(result.status, None);
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_eks_cluster_tags() {
        let mut tags = std::collections::HashMap::new();
        tags.insert("env".to_string(), "prod".to_string());
        let cluster = aws_sdk_eks::types::Cluster::builder()
            .name("tagged-cluster")
            .set_tags(Some(tags))
            .build();
        let result = EksCluster::from(cluster);
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_eks_nodegroup_from() {
        let ng = aws_sdk_eks::types::Nodegroup::builder()
            .nodegroup_name("my-ng")
            .nodegroup_arn("arn:aws:eks:us-east-1:123:nodegroup/my-cluster/my-ng/abc")
            .cluster_name("my-cluster")
            .status(aws_sdk_eks::types::NodegroupStatus::Active)
            .instance_types("t3.medium")
            .disk_size(20)
            .subnets("subnet-aaa")
            .node_role("arn:aws:iam::123:role/node-role")
            .build();
        let result = EksNodegroup::from(ng);
        assert_eq!(result.name, "my-ng");
        assert_eq!(result.cluster_name, Some("my-cluster".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.instance_types, vec!["t3.medium".to_string()]);
        assert_eq!(result.disk_size, Some(20));
        assert_eq!(result.subnet_ids, vec!["subnet-aaa".to_string()]);
        assert_eq!(
            result.node_role,
            Some("arn:aws:iam::123:role/node-role".to_string())
        );
        assert!(result.labels.is_empty());
    }

    #[test]
    fn test_eks_fargate_profile_from() {
        let fp = aws_sdk_eks::types::FargateProfile::builder()
            .fargate_profile_name("my-fp")
            .fargate_profile_arn("arn:aws:eks:us-east-1:123:fargateprofile/my-cluster/my-fp/abc")
            .cluster_name("my-cluster")
            .status(aws_sdk_eks::types::FargateProfileStatus::Active)
            .pod_execution_role_arn("arn:aws:iam::123:role/fargate-role")
            .subnets("subnet-aaa")
            .build();
        let result = EksFargateProfile::from(fp);
        assert_eq!(result.name, "my-fp");
        assert_eq!(result.cluster_name, Some("my-cluster".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(
            result.pod_execution_role_arn,
            Some("arn:aws:iam::123:role/fargate-role".to_string())
        );
        assert_eq!(result.subnet_ids, vec!["subnet-aaa".to_string()]);
        assert!(result.selectors.is_empty());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_eks_addon_from() {
        let addon = aws_sdk_eks::types::Addon::builder()
            .addon_name("vpc-cni")
            .addon_arn("arn:aws:eks:us-east-1:123:addon/my-cluster/vpc-cni/abc")
            .cluster_name("my-cluster")
            .status(aws_sdk_eks::types::AddonStatus::Active)
            .addon_version("v1.12.0-eksbuild.1")
            .service_account_role_arn("arn:aws:iam::123:role/addon-role")
            .build();
        let result = EksAddon::from(addon);
        assert_eq!(result.name, "vpc-cni");
        assert_eq!(result.cluster_name, Some("my-cluster".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.addon_version, Some("v1.12.0-eksbuild.1".to_string()));
        assert_eq!(
            result.service_account_role_arn,
            Some("arn:aws:iam::123:role/addon-role".to_string())
        );
        assert!(result.tags.is_empty());
    }
}
