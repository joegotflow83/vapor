use async_graphql::{Enum, InputObject, SimpleObject};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum InstanceState {
    Pending,
    Running,
    ShuttingDown,
    Terminated,
    Stopping,
    Stopped,
}

#[derive(SimpleObject, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(InputObject)]
pub struct TagFilter {
    pub key: String,
    pub value: String,
}

#[derive(InputObject)]
pub struct TagInput {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct SecurityGroupRef {
    pub id: String,
    pub name: String,
}

#[derive(SimpleObject, Clone)]
pub struct NetworkInterface {
    pub id: String,
    pub subnet_id: Option<String>,
    pub vpc_id: Option<String>,
    pub private_ip: Option<String>,
    pub public_ip: Option<String>,
    pub mac_address: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct BlockDevice {
    pub device_name: String,
    pub volume_id: Option<String>,
    pub status: Option<String>,
    pub delete_on_termination: bool,
}

/// EC2 instance metadata service (IMDS) configuration.
/// `http_tokens = "required"` means IMDSv2 is enforced — CIS AWS Benchmark 4.4.
/// `http_tokens = "optional"` means IMDSv1 is still allowed (SSRF credential-theft risk).
#[derive(SimpleObject, Clone)]
pub struct InstanceMetadataOptions {
    /// "optional" (IMDSv1 allowed) or "required" (IMDSv2 enforced)
    pub http_tokens: Option<String>,
    /// "enabled" or "disabled" — whether the IMDS endpoint is accessible at all
    pub http_endpoint: Option<String>,
    /// 1–64; controls how many network hops a PUT response can traverse
    pub http_put_response_hop_limit: Option<i32>,
    /// "enabled" or "disabled" for IPv6 metadata endpoint
    pub http_protocol_ipv6: Option<String>,
    /// "enabled" or "disabled" — whether instance tags are accessible via IMDS
    pub instance_metadata_tags: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct Instance {
    pub id: String,
    pub instance_type: Option<String>,
    pub state: InstanceState,
    pub az: Option<String>,
    pub public_ip: Option<String>,
    pub private_ip: Option<String>,
    pub vpc_id: Option<String>,
    pub subnet_id: Option<String>,
    pub key_name: Option<String>,
    pub launch_time: Option<String>,
    pub tags: Vec<Tag>,
    pub security_groups: Vec<SecurityGroupRef>,
    pub network_interfaces: Vec<NetworkInterface>,
    pub block_devices: Vec<BlockDevice>,
    /// IMDS configuration — check `http_tokens == "required"` for CIS 4.4 IMDSv2 compliance
    pub metadata_options: Option<InstanceMetadataOptions>,
}

#[derive(SimpleObject, Clone)]
pub struct Rule {
    pub protocol: Option<String>,
    pub from_port: Option<i32>,
    pub to_port: Option<i32>,
    pub cidr_ipv4: Option<String>,
    pub cidr_ipv6: Option<String>,
    pub description: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct SecurityGroup {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub vpc_id: Option<String>,
    pub ingress_rules: Vec<Rule>,
    pub egress_rules: Vec<Rule>,
    pub tags: Vec<Tag>,
}

fn ip_permission_to_rules(perm: &aws_sdk_ec2::types::IpPermission) -> Vec<Rule> {
    let protocol = perm.ip_protocol().map(|s| s.to_string());
    let from_port = perm.from_port();
    let to_port = perm.to_port();

    let mut rules: Vec<Rule> = Vec::new();

    for range in perm.ip_ranges() {
        rules.push(Rule {
            protocol: protocol.clone(),
            from_port,
            to_port,
            cidr_ipv4: range.cidr_ip().map(|s| s.to_string()),
            cidr_ipv6: None,
            description: range.description().map(|s| s.to_string()),
        });
    }

    for range in perm.ipv6_ranges() {
        rules.push(Rule {
            protocol: protocol.clone(),
            from_port,
            to_port,
            cidr_ipv4: None,
            cidr_ipv6: range.cidr_ipv6().map(|s| s.to_string()),
            description: range.description().map(|s| s.to_string()),
        });
    }

    if rules.is_empty() {
        rules.push(Rule {
            protocol,
            from_port,
            to_port,
            cidr_ipv4: None,
            cidr_ipv6: None,
            description: None,
        });
    }

    rules
}

impl From<aws_sdk_ec2::types::SecurityGroup> for SecurityGroup {
    fn from(sg: aws_sdk_ec2::types::SecurityGroup) -> Self {
        let tags = sg
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        let ingress_rules = sg
            .ip_permissions()
            .iter()
            .flat_map(ip_permission_to_rules)
            .collect();

        let egress_rules = sg
            .ip_permissions_egress()
            .iter()
            .flat_map(ip_permission_to_rules)
            .collect();

        SecurityGroup {
            id: sg.group_id().unwrap_or_default().to_string(),
            name: sg.group_name().map(|s| s.to_string()),
            description: sg.description().map(|s| s.to_string()),
            vpc_id: sg.vpc_id().map(|s| s.to_string()),
            ingress_rules,
            egress_rules,
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Vpc {
    pub id: String,
    pub cidr_block: Option<String>,
    pub state: Option<String>,
    pub is_default: bool,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::Vpc> for Vpc {
    fn from(v: aws_sdk_ec2::types::Vpc) -> Self {
        let tags = v
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        Vpc {
            id: v.vpc_id().unwrap_or_default().to_string(),
            cidr_block: v.cidr_block().map(|s| s.to_string()),
            state: v.state().map(|s| s.as_str().to_string()),
            is_default: v.is_default().unwrap_or(false),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Subnet {
    pub id: String,
    pub vpc_id: Option<String>,
    pub az: Option<String>,
    pub cidr_block: Option<String>,
    pub available_ips: Option<i32>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::Subnet> for Subnet {
    fn from(s: aws_sdk_ec2::types::Subnet) -> Self {
        let tags = s
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        Subnet {
            id: s.subnet_id().unwrap_or_default().to_string(),
            vpc_id: s.vpc_id().map(|s| s.to_string()),
            az: s.availability_zone().map(|s| s.to_string()),
            cidr_block: s.cidr_block().map(|s| s.to_string()),
            available_ips: s.available_ip_address_count(),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct VolumeAttachment {
    pub instance_id: Option<String>,
    pub device: Option<String>,
    pub state: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct Volume {
    pub id: String,
    pub size: Option<i32>,
    pub volume_type: Option<String>,
    pub state: Option<String>,
    pub az: Option<String>,
    pub encrypted: bool,
    pub attachments: Vec<VolumeAttachment>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::Volume> for Volume {
    fn from(v: aws_sdk_ec2::types::Volume) -> Self {
        let tags = v
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        let attachments = v
            .attachments()
            .iter()
            .map(|a| VolumeAttachment {
                instance_id: a.instance_id().map(|s| s.to_string()),
                device: a.device().map(|s| s.to_string()),
                state: a.state().map(|s| s.as_str().to_string()),
            })
            .collect();

        Volume {
            id: v.volume_id().unwrap_or_default().to_string(),
            size: v.size(),
            volume_type: v.volume_type().map(|t| t.as_str().to_string()),
            state: v.state().map(|s| s.as_str().to_string()),
            az: v.availability_zone().map(|s| s.to_string()),
            encrypted: v.encrypted().unwrap_or(false),
            attachments,
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct InstanceStateChange {
    pub instance_id: String,
    pub previous_state: InstanceState,
    pub current_state: InstanceState,
}

#[derive(SimpleObject, Clone)]
pub struct KeyPair {
    pub key_pair_id: String,
    pub name: Option<String>,
    pub key_type: Option<String>,
    pub fingerprint: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::KeyPairInfo> for KeyPair {
    fn from(kp: aws_sdk_ec2::types::KeyPairInfo) -> Self {
        let tags = kp
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        KeyPair {
            key_pair_id: kp.key_pair_id().unwrap_or_default().to_string(),
            name: kp.key_name().map(|s| s.to_string()),
            key_type: kp.key_type().map(|t| t.as_str().to_string()),
            fingerprint: kp.key_fingerprint().map(|s| s.to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Image {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub state: Option<String>,
    pub architecture: Option<String>,
    pub platform: Option<String>,
    pub owner_id: Option<String>,
    pub public: bool,
    pub creation_date: Option<String>,
    pub root_device_type: Option<String>,
    pub virtualization_type: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::Image> for Image {
    fn from(img: aws_sdk_ec2::types::Image) -> Self {
        let tags = img
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        Image {
            id: img.image_id().unwrap_or_default().to_string(),
            name: img.name().map(|s| s.to_string()),
            description: img.description().map(|s| s.to_string()),
            state: img.state().map(|s| s.as_str().to_string()),
            architecture: img.architecture().map(|a| a.as_str().to_string()),
            platform: img.platform_details().map(|s| s.to_string()),
            owner_id: img.owner_id().map(|s| s.to_string()),
            public: img.public().unwrap_or(false),
            creation_date: img.creation_date().map(|s| s.to_string()),
            root_device_type: img.root_device_type().map(|r| r.as_str().to_string()),
            virtualization_type: img.virtualization_type().map(|v| v.as_str().to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ElasticIp {
    pub allocation_id: Option<String>,
    pub public_ip: String,
    pub instance_id: Option<String>,
    pub network_interface_id: Option<String>,
    pub private_ip: Option<String>,
    pub domain: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::Address> for ElasticIp {
    fn from(a: aws_sdk_ec2::types::Address) -> Self {
        let tags = a
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        ElasticIp {
            allocation_id: a.allocation_id().map(|s| s.to_string()),
            public_ip: a.public_ip().unwrap_or_default().to_string(),
            instance_id: a.instance_id().map(|s| s.to_string()),
            network_interface_id: a.network_interface_id().map(|s| s.to_string()),
            private_ip: a.private_ip_address().map(|s| s.to_string()),
            domain: a.domain().map(|d| d.as_str().to_string()),
            tags,
        }
    }
}

#[derive(InputObject)]
pub struct RunInstancesInput {
    pub image_id: String,
    pub instance_type: String,
    pub min_count: i32,
    pub max_count: i32,
    pub key_name: Option<String>,
    pub security_group_ids: Option<Vec<String>>,
    pub subnet_id: Option<String>,
    pub tags: Option<Vec<TagInput>>,
}

#[derive(SimpleObject, Clone)]
pub struct LaunchTemplate {
    pub id: Option<String>,
    pub name: Option<String>,
    pub default_version: Option<i64>,
    pub latest_version: Option<i64>,
    pub created_by: Option<String>,
    pub create_time: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::LaunchTemplate> for LaunchTemplate {
    fn from(lt: aws_sdk_ec2::types::LaunchTemplate) -> Self {
        Self {
            id: lt.launch_template_id().map(|s| s.to_string()),
            name: lt.launch_template_name().map(|s| s.to_string()),
            default_version: lt.default_version_number(),
            latest_version: lt.latest_version_number(),
            created_by: lt.created_by().map(|s| s.to_string()),
            create_time: lt.create_time().map(|d| d.to_string()),
            tags: lt
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
pub struct LaunchTemplateVersion {
    pub launch_template_id: Option<String>,
    pub launch_template_name: Option<String>,
    pub version_number: Option<i64>,
    pub version_description: Option<String>,
    pub default_version: Option<bool>,
    pub created_by: Option<String>,
    pub create_time: Option<String>,
    pub image_id: Option<String>,
    pub instance_type: Option<String>,
    pub key_name: Option<String>,
    pub user_data: Option<String>,
    pub security_group_ids: Vec<String>,
    pub iam_instance_profile_arn: Option<String>,
    pub iam_instance_profile_name: Option<String>,
}

impl From<aws_sdk_ec2::types::LaunchTemplateVersion> for LaunchTemplateVersion {
    fn from(v: aws_sdk_ec2::types::LaunchTemplateVersion) -> Self {
        let data = v.launch_template_data();
        Self {
            launch_template_id: v.launch_template_id().map(|s| s.to_string()),
            launch_template_name: v.launch_template_name().map(|s| s.to_string()),
            version_number: v.version_number(),
            version_description: v.version_description().map(|s| s.to_string()),
            default_version: v.default_version(),
            created_by: v.created_by().map(|s| s.to_string()),
            create_time: v.create_time().map(|d| d.to_string()),
            image_id: data.and_then(|d| d.image_id()).map(|s| s.to_string()),
            instance_type: data
                .and_then(|d| d.instance_type())
                .map(|t| t.as_str().to_string()),
            key_name: data.and_then(|d| d.key_name()).map(|s| s.to_string()),
            user_data: data.and_then(|d| d.user_data()).map(|s| s.to_string()),
            security_group_ids: data
                .map(|d| d.security_group_ids().iter().map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            iam_instance_profile_arn: data
                .and_then(|d| d.iam_instance_profile())
                .and_then(|p| p.arn())
                .map(|s| s.to_string()),
            iam_instance_profile_name: data
                .and_then(|d| d.iam_instance_profile())
                .and_then(|p| p.name())
                .map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Snapshot {
    pub id: String,
    pub volume_id: Option<String>,
    pub volume_size: Option<i32>,
    pub state: Option<String>,
    pub progress: Option<String>,
    pub start_time: Option<String>,
    pub description: Option<String>,
    pub owner_id: Option<String>,
    pub encrypted: bool,
    pub kms_key_id: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::Snapshot> for Snapshot {
    fn from(s: aws_sdk_ec2::types::Snapshot) -> Self {
        let tags = s
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        Snapshot {
            id: s.snapshot_id().unwrap_or_default().to_string(),
            volume_id: s.volume_id().map(|v| v.to_string()),
            volume_size: s.volume_size(),
            state: s.state().map(|st| st.as_str().to_string()),
            progress: s.progress().map(|p| p.to_string()),
            start_time: s.start_time().map(|d| d.to_string()),
            description: s.description().map(|d| d.to_string()),
            owner_id: s.owner_id().map(|o| o.to_string()),
            encrypted: s.encrypted().unwrap_or(false),
            kms_key_id: s.kms_key_id().map(|k| k.to_string()),
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Vpc ---

    #[test]
    fn test_vpc_from_sdk_populated() {
        let sdk_vpc = aws_sdk_ec2::types::Vpc::builder()
            .vpc_id("vpc-abc123")
            .cidr_block("10.0.0.0/16")
            .is_default(true)
            .tags(
                aws_sdk_ec2::types::Tag::builder()
                    .key("Name")
                    .value("main")
                    .build(),
            )
            .build();

        let vpc: Vpc = sdk_vpc.into();
        assert_eq!(vpc.id, "vpc-abc123");
        assert_eq!(vpc.cidr_block, Some("10.0.0.0/16".to_string()));
        assert!(vpc.is_default);
        assert_eq!(vpc.tags.len(), 1);
        assert_eq!(vpc.tags[0].key, "Name");
        assert_eq!(vpc.tags[0].value, "main");
    }

    #[test]
    fn test_vpc_from_sdk_empty() {
        let sdk_vpc = aws_sdk_ec2::types::Vpc::builder().build();
        let vpc: Vpc = sdk_vpc.into();
        // unwrap_or_default yields empty string
        assert_eq!(vpc.id, "");
        assert_eq!(vpc.cidr_block, None);
        assert!(!vpc.is_default);
        assert!(vpc.tags.is_empty());
    }

    // --- Subnet ---

    #[test]
    fn test_subnet_from_sdk_populated() {
        let sdk_subnet = aws_sdk_ec2::types::Subnet::builder()
            .subnet_id("subnet-abc123")
            .vpc_id("vpc-abc123")
            .availability_zone("us-east-1a")
            .cidr_block("10.0.1.0/24")
            .available_ip_address_count(251)
            .build();

        let subnet: Subnet = sdk_subnet.into();
        assert_eq!(subnet.id, "subnet-abc123");
        assert_eq!(subnet.vpc_id, Some("vpc-abc123".to_string()));
        assert_eq!(subnet.az, Some("us-east-1a".to_string()));
        assert_eq!(subnet.cidr_block, Some("10.0.1.0/24".to_string()));
        assert_eq!(subnet.available_ips, Some(251));
        assert!(subnet.tags.is_empty());
    }

    #[test]
    fn test_subnet_from_sdk_empty() {
        let sdk_subnet = aws_sdk_ec2::types::Subnet::builder().build();
        let subnet: Subnet = sdk_subnet.into();
        assert_eq!(subnet.id, "");
        assert_eq!(subnet.vpc_id, None);
        assert_eq!(subnet.az, None);
        assert_eq!(subnet.available_ips, None);
    }

    // --- SecurityGroup ---

    #[test]
    fn test_security_group_from_sdk_basic() {
        let sdk_sg = aws_sdk_ec2::types::SecurityGroup::builder()
            .group_id("sg-abc123")
            .group_name("my-sg")
            .description("Test SG")
            .vpc_id("vpc-abc123")
            .build();

        let sg: SecurityGroup = sdk_sg.into();
        assert_eq!(sg.id, "sg-abc123");
        assert_eq!(sg.name, Some("my-sg".to_string()));
        assert_eq!(sg.description, Some("Test SG".to_string()));
        assert_eq!(sg.vpc_id, Some("vpc-abc123".to_string()));
        assert!(sg.ingress_rules.is_empty());
        assert!(sg.egress_rules.is_empty());
    }

    #[test]
    fn test_security_group_from_sdk_with_ingress_ipv4() {
        let ip_range = aws_sdk_ec2::types::IpRange::builder()
            .cidr_ip("0.0.0.0/0")
            .description("allow all")
            .build();
        let perm = aws_sdk_ec2::types::IpPermission::builder()
            .ip_protocol("tcp")
            .from_port(80)
            .to_port(80)
            .ip_ranges(ip_range)
            .build();
        let sdk_sg = aws_sdk_ec2::types::SecurityGroup::builder()
            .group_id("sg-abc123")
            .ip_permissions(perm)
            .build();

        let sg: SecurityGroup = sdk_sg.into();
        assert_eq!(sg.ingress_rules.len(), 1);
        let rule = &sg.ingress_rules[0];
        assert_eq!(rule.protocol, Some("tcp".to_string()));
        assert_eq!(rule.from_port, Some(80));
        assert_eq!(rule.to_port, Some(80));
        assert_eq!(rule.cidr_ipv4, Some("0.0.0.0/0".to_string()));
        assert_eq!(rule.cidr_ipv6, None);
        assert_eq!(rule.description, Some("allow all".to_string()));
    }

    // --- Volume ---

    #[test]
    fn test_volume_from_sdk_populated() {
        let sdk_volume = aws_sdk_ec2::types::Volume::builder()
            .volume_id("vol-abc123")
            .size(100)
            .encrypted(true)
            .availability_zone("us-east-1a")
            .build();

        let volume: Volume = sdk_volume.into();
        assert_eq!(volume.id, "vol-abc123");
        assert_eq!(volume.size, Some(100));
        assert!(volume.encrypted);
        assert_eq!(volume.az, Some("us-east-1a".to_string()));
        assert!(volume.attachments.is_empty());
        assert!(volume.tags.is_empty());
    }

    #[test]
    fn test_volume_from_sdk_with_attachment() {
        let attachment = aws_sdk_ec2::types::VolumeAttachment::builder()
            .instance_id("i-abc123")
            .device("/dev/xvda")
            .build();
        let sdk_volume = aws_sdk_ec2::types::Volume::builder()
            .volume_id("vol-abc123")
            .attachments(attachment)
            .build();

        let volume: Volume = sdk_volume.into();
        assert_eq!(volume.attachments.len(), 1);
        assert_eq!(
            volume.attachments[0].instance_id,
            Some("i-abc123".to_string())
        );
        assert_eq!(volume.attachments[0].device, Some("/dev/xvda".to_string()));
    }

    // --- Instance ---

    #[test]
    fn test_instance_from_sdk_minimal() {
        let sdk_instance = aws_sdk_ec2::types::Instance::builder()
            .instance_id("i-abc123")
            .build();

        let instance: Instance = sdk_instance.into();
        assert_eq!(instance.id, "i-abc123");
        // No state set → defaults to Running
        assert_eq!(instance.state, InstanceState::Running);
        assert!(instance.tags.is_empty());
        assert!(instance.security_groups.is_empty());
        assert!(instance.network_interfaces.is_empty());
        assert!(instance.block_devices.is_empty());
    }

    #[test]
    fn test_instance_state_stopped() {
        let sdk_state = aws_sdk_ec2::types::InstanceState::builder()
            .name(aws_sdk_ec2::types::InstanceStateName::Stopped)
            .build();
        let sdk_instance = aws_sdk_ec2::types::Instance::builder()
            .instance_id("i-abc123")
            .state(sdk_state)
            .build();

        let instance: Instance = sdk_instance.into();
        assert_eq!(instance.state, InstanceState::Stopped);
    }

    #[test]
    fn test_instance_state_all_variants() {
        use aws_sdk_ec2::types::InstanceStateName;

        let cases = [
            (InstanceStateName::Pending, InstanceState::Pending),
            (InstanceStateName::Running, InstanceState::Running),
            (InstanceStateName::ShuttingDown, InstanceState::ShuttingDown),
            (InstanceStateName::Terminated, InstanceState::Terminated),
            (InstanceStateName::Stopping, InstanceState::Stopping),
            (InstanceStateName::Stopped, InstanceState::Stopped),
        ];

        for (sdk_name, expected) in cases {
            let sdk_state = aws_sdk_ec2::types::InstanceState::builder()
                .name(sdk_name)
                .build();
            let sdk_instance = aws_sdk_ec2::types::Instance::builder()
                .instance_id("i-abc123")
                .state(sdk_state)
                .build();
            let instance: Instance = sdk_instance.into();
            assert_eq!(instance.state, expected);
        }
    }

    #[test]
    fn test_instance_with_tags() {
        let tag = aws_sdk_ec2::types::Tag::builder()
            .key("Name")
            .value("my-instance")
            .build();
        let sdk_instance = aws_sdk_ec2::types::Instance::builder()
            .instance_id("i-abc123")
            .tags(tag)
            .build();

        let instance: Instance = sdk_instance.into();
        assert_eq!(instance.tags.len(), 1);
        assert_eq!(instance.tags[0].key, "Name");
        assert_eq!(instance.tags[0].value, "my-instance");
    }

    #[test]
    fn test_instance_with_network_interface() {
        let ni = aws_sdk_ec2::types::InstanceNetworkInterface::builder()
            .network_interface_id("eni-abc123")
            .subnet_id("subnet-abc")
            .vpc_id("vpc-abc")
            .private_ip_address("10.0.0.5")
            .mac_address("0a:1b:2c:3d:4e:5f")
            .build();
        let sdk_instance = aws_sdk_ec2::types::Instance::builder()
            .instance_id("i-abc123")
            .network_interfaces(ni)
            .build();

        let instance: Instance = sdk_instance.into();
        assert_eq!(instance.network_interfaces.len(), 1);
        let iface = &instance.network_interfaces[0];
        assert_eq!(iface.id, "eni-abc123");
        assert_eq!(iface.subnet_id, Some("subnet-abc".to_string()));
        assert_eq!(iface.vpc_id, Some("vpc-abc".to_string()));
        assert_eq!(iface.private_ip, Some("10.0.0.5".to_string()));
        assert_eq!(
            iface.mac_address,
            Some("0a:1b:2c:3d:4e:5f".to_string())
        );
    }

    // --- Image ---

    #[test]
    fn test_image_from_sdk_populated() {
        let sdk_img = aws_sdk_ec2::types::Image::builder()
            .image_id("ami-abc123")
            .name("my-ami")
            .description("My test AMI")
            .state(aws_sdk_ec2::types::ImageState::Available)
            .architecture(aws_sdk_ec2::types::ArchitectureValues::X8664)
            .platform_details("Linux/UNIX")
            .owner_id("123456789012")
            .public(true)
            .creation_date("2024-01-01T00:00:00Z")
            .root_device_type(aws_sdk_ec2::types::DeviceType::Ebs)
            .virtualization_type(aws_sdk_ec2::types::VirtualizationType::Hvm)
            .tags(
                aws_sdk_ec2::types::Tag::builder()
                    .key("Name")
                    .value("test-ami")
                    .build(),
            )
            .build();

        let img: Image = sdk_img.into();
        assert_eq!(img.id, "ami-abc123");
        assert_eq!(img.name, Some("my-ami".to_string()));
        assert_eq!(img.description, Some("My test AMI".to_string()));
        assert_eq!(img.state, Some("available".to_string()));
        assert_eq!(img.architecture, Some("x86_64".to_string()));
        assert_eq!(img.platform, Some("Linux/UNIX".to_string()));
        assert_eq!(img.owner_id, Some("123456789012".to_string()));
        assert!(img.public);
        assert_eq!(img.creation_date, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(img.root_device_type, Some("ebs".to_string()));
        assert_eq!(img.virtualization_type, Some("hvm".to_string()));
        assert_eq!(img.tags.len(), 1);
        assert_eq!(img.tags[0].key, "Name");
        assert_eq!(img.tags[0].value, "test-ami");
    }

    #[test]
    fn test_image_from_sdk_defaults() {
        let sdk_img = aws_sdk_ec2::types::Image::builder().build();
        let img: Image = sdk_img.into();
        assert_eq!(img.id, "");
        assert_eq!(img.name, None);
        assert_eq!(img.description, None);
        assert_eq!(img.state, None);
        assert_eq!(img.architecture, None);
        assert_eq!(img.platform, None);
        assert_eq!(img.owner_id, None);
        assert!(!img.public);
        assert_eq!(img.creation_date, None);
        assert_eq!(img.root_device_type, None);
        assert_eq!(img.virtualization_type, None);
        assert!(img.tags.is_empty());
    }

    // --- ElasticIp ---

    #[test]
    fn test_elastic_ip_from_sdk_populated() {
        let sdk_addr = aws_sdk_ec2::types::Address::builder()
            .allocation_id("eipalloc-abc123")
            .public_ip("1.2.3.4")
            .instance_id("i-abc123")
            .network_interface_id("eni-abc123")
            .private_ip_address("10.0.0.5")
            .domain(aws_sdk_ec2::types::DomainType::Vpc)
            .tags(
                aws_sdk_ec2::types::Tag::builder()
                    .key("Env")
                    .value("prod")
                    .build(),
            )
            .build();

        let eip: ElasticIp = sdk_addr.into();
        assert_eq!(eip.allocation_id, Some("eipalloc-abc123".to_string()));
        assert_eq!(eip.public_ip, "1.2.3.4".to_string());
        assert_eq!(eip.instance_id, Some("i-abc123".to_string()));
        assert_eq!(eip.network_interface_id, Some("eni-abc123".to_string()));
        assert_eq!(eip.private_ip, Some("10.0.0.5".to_string()));
        assert_eq!(eip.domain, Some("vpc".to_string()));
        assert_eq!(eip.tags.len(), 1);
        assert_eq!(eip.tags[0].key, "Env");
        assert_eq!(eip.tags[0].value, "prod");
    }

    #[test]
    fn test_elastic_ip_from_sdk_empty() {
        let sdk_addr = aws_sdk_ec2::types::Address::builder().build();
        let eip: ElasticIp = sdk_addr.into();
        assert_eq!(eip.allocation_id, None);
        assert_eq!(eip.public_ip, "".to_string());
        assert_eq!(eip.instance_id, None);
        assert_eq!(eip.network_interface_id, None);
        assert_eq!(eip.private_ip, None);
        assert_eq!(eip.domain, None);
        assert!(eip.tags.is_empty());
    }

    // --- KeyPair ---

    #[test]
    fn test_key_pair_from_sdk_populated() {
        let sdk_kp = aws_sdk_ec2::types::KeyPairInfo::builder()
            .key_pair_id("key-abc123")
            .key_name("my-key")
            .key_type(aws_sdk_ec2::types::KeyType::Rsa)
            .key_fingerprint("aa:bb:cc:dd")
            .tags(
                aws_sdk_ec2::types::Tag::builder()
                    .key("Name")
                    .value("test-key")
                    .build(),
            )
            .build();

        let kp: KeyPair = sdk_kp.into();
        assert_eq!(kp.key_pair_id, "key-abc123");
        assert_eq!(kp.name, Some("my-key".to_string()));
        assert_eq!(kp.key_type, Some("rsa".to_string()));
        assert_eq!(kp.fingerprint, Some("aa:bb:cc:dd".to_string()));
        assert_eq!(kp.tags.len(), 1);
        assert_eq!(kp.tags[0].key, "Name");
        assert_eq!(kp.tags[0].value, "test-key");
    }

    #[test]
    fn test_key_pair_from_sdk_empty() {
        let sdk_kp = aws_sdk_ec2::types::KeyPairInfo::builder().build();
        let kp: KeyPair = sdk_kp.into();
        assert_eq!(kp.key_pair_id, "");
        assert_eq!(kp.name, None);
        assert_eq!(kp.key_type, None);
        assert_eq!(kp.fingerprint, None);
        assert!(kp.tags.is_empty());
    }

    #[test]
    fn test_instance_with_block_device() {
        let ebs = aws_sdk_ec2::types::EbsInstanceBlockDevice::builder()
            .volume_id("vol-abc123")
            .delete_on_termination(true)
            .build();
        let bd = aws_sdk_ec2::types::InstanceBlockDeviceMapping::builder()
            .device_name("/dev/xvda")
            .ebs(ebs)
            .build();
        let sdk_instance = aws_sdk_ec2::types::Instance::builder()
            .instance_id("i-abc123")
            .block_device_mappings(bd)
            .build();

        let instance: Instance = sdk_instance.into();
        assert_eq!(instance.block_devices.len(), 1);
        let dev = &instance.block_devices[0];
        assert_eq!(dev.device_name, "/dev/xvda");
        assert_eq!(dev.volume_id, Some("vol-abc123".to_string()));
        assert!(dev.delete_on_termination);
    }

    // --- LaunchTemplate ---

    #[test]
    fn test_launch_template_from_sdk_populated() {
        use aws_sdk_ec2::primitives::DateTime;
        let ts = DateTime::from_secs(1700000000);
        let sdk_lt = aws_sdk_ec2::types::LaunchTemplate::builder()
            .launch_template_id("lt-abc123")
            .launch_template_name("my-template")
            .default_version_number(1)
            .latest_version_number(3)
            .created_by("arn:aws:iam::123456789012:user/admin")
            .create_time(ts)
            .tags(
                aws_sdk_ec2::types::Tag::builder()
                    .key("Env")
                    .value("prod")
                    .build(),
            )
            .build();

        let lt: LaunchTemplate = sdk_lt.into();
        assert_eq!(lt.id, Some("lt-abc123".to_string()));
        assert_eq!(lt.name, Some("my-template".to_string()));
        assert_eq!(lt.default_version, Some(1));
        assert_eq!(lt.latest_version, Some(3));
        assert_eq!(
            lt.created_by,
            Some("arn:aws:iam::123456789012:user/admin".to_string())
        );
        assert!(lt.create_time.is_some());
        assert_eq!(lt.tags.len(), 1);
        assert_eq!(lt.tags[0].key, "Env");
        assert_eq!(lt.tags[0].value, "prod");
    }

    #[test]
    fn test_launch_template_from_sdk_empty() {
        let sdk_lt = aws_sdk_ec2::types::LaunchTemplate::builder().build();
        let lt: LaunchTemplate = sdk_lt.into();
        assert_eq!(lt.id, None);
        assert_eq!(lt.name, None);
        assert_eq!(lt.default_version, None);
        assert_eq!(lt.latest_version, None);
        assert_eq!(lt.created_by, None);
        assert_eq!(lt.create_time, None);
        assert!(lt.tags.is_empty());
    }

    // --- LaunchTemplateVersion ---

    #[test]
    fn test_launch_template_version_from_sdk_populated() {
        use aws_sdk_ec2::primitives::DateTime;
        let ts = DateTime::from_secs(1700000000);
        let profile = aws_sdk_ec2::types::LaunchTemplateIamInstanceProfileSpecification::builder()
            .arn("arn:aws:iam::123:instance-profile/my-profile")
            .name("my-profile")
            .build();
        let data = aws_sdk_ec2::types::ResponseLaunchTemplateData::builder()
            .image_id("ami-abc123")
            .instance_type(aws_sdk_ec2::types::InstanceType::T3Medium)
            .key_name("my-key")
            .user_data("dXNlcmRhdGE=")
            .security_group_ids("sg-abc123")
            .security_group_ids("sg-def456")
            .iam_instance_profile(profile)
            .build();
        let sdk_v = aws_sdk_ec2::types::LaunchTemplateVersion::builder()
            .launch_template_id("lt-abc123")
            .launch_template_name("my-template")
            .version_number(2)
            .version_description("v2 release")
            .default_version(false)
            .created_by("arn:aws:iam::123:user/admin")
            .create_time(ts)
            .launch_template_data(data)
            .build();

        let v: LaunchTemplateVersion = sdk_v.into();
        assert_eq!(v.launch_template_id, Some("lt-abc123".to_string()));
        assert_eq!(v.launch_template_name, Some("my-template".to_string()));
        assert_eq!(v.version_number, Some(2));
        assert_eq!(v.version_description, Some("v2 release".to_string()));
        assert_eq!(v.default_version, Some(false));
        assert_eq!(v.created_by, Some("arn:aws:iam::123:user/admin".to_string()));
        assert!(v.create_time.is_some());
        assert_eq!(v.image_id, Some("ami-abc123".to_string()));
        assert_eq!(v.instance_type, Some("t3.medium".to_string()));
        assert_eq!(v.key_name, Some("my-key".to_string()));
        assert_eq!(v.user_data, Some("dXNlcmRhdGE=".to_string()));
        assert_eq!(v.security_group_ids, vec!["sg-abc123", "sg-def456"]);
        assert_eq!(
            v.iam_instance_profile_arn,
            Some("arn:aws:iam::123:instance-profile/my-profile".to_string())
        );
        assert_eq!(
            v.iam_instance_profile_name,
            Some("my-profile".to_string())
        );
    }

    #[test]
    fn test_launch_template_version_from_sdk_no_data() {
        let sdk_v = aws_sdk_ec2::types::LaunchTemplateVersion::builder()
            .launch_template_id("lt-abc123")
            .version_number(1)
            .default_version(true)
            .build();

        let v: LaunchTemplateVersion = sdk_v.into();
        assert_eq!(v.launch_template_id, Some("lt-abc123".to_string()));
        assert_eq!(v.version_number, Some(1));
        assert_eq!(v.default_version, Some(true));
        assert_eq!(v.image_id, None);
        assert_eq!(v.instance_type, None);
        assert_eq!(v.key_name, None);
        assert_eq!(v.user_data, None);
        assert!(v.security_group_ids.is_empty());
        assert_eq!(v.iam_instance_profile_arn, None);
        assert_eq!(v.iam_instance_profile_name, None);
    }

    // --- Snapshot ---

    // --- InstanceMetadataOptions ---

    #[test]
    fn test_instance_metadata_options_imdsv2_required() {
        let metadata_opts = aws_sdk_ec2::types::InstanceMetadataOptionsResponse::builder()
            .http_tokens(aws_sdk_ec2::types::HttpTokensState::Required)
            .http_endpoint(aws_sdk_ec2::types::InstanceMetadataEndpointState::Enabled)
            .http_put_response_hop_limit(1)
            .build();
        let sdk_instance = aws_sdk_ec2::types::Instance::builder()
            .instance_id("i-imdsv2")
            .metadata_options(metadata_opts)
            .build();

        let instance: Instance = sdk_instance.into();
        let meta = instance.metadata_options.expect("metadata_options should be Some");
        assert_eq!(meta.http_tokens, Some("required".to_string()));
        assert_eq!(meta.http_endpoint, Some("enabled".to_string()));
        assert_eq!(meta.http_put_response_hop_limit, Some(1));
    }

    #[test]
    fn test_instance_metadata_options_absent() {
        let sdk_instance = aws_sdk_ec2::types::Instance::builder()
            .instance_id("i-no-meta")
            .build();

        let instance: Instance = sdk_instance.into();
        assert!(instance.metadata_options.is_none());
    }

    #[test]
    fn test_snapshot_from_sdk_populated() {
        use aws_sdk_ec2::primitives::DateTime;
        let ts = DateTime::from_secs(1700000000);
        let sdk_snap = aws_sdk_ec2::types::Snapshot::builder()
            .snapshot_id("snap-abc123")
            .volume_id("vol-abc123")
            .volume_size(100)
            .state(aws_sdk_ec2::types::SnapshotState::Completed)
            .progress("100%")
            .start_time(ts)
            .description("my snapshot")
            .owner_id("123456789012")
            .encrypted(true)
            .kms_key_id("arn:aws:kms:us-east-1:123:key/abc")
            .tags(
                aws_sdk_ec2::types::Tag::builder()
                    .key("Name")
                    .value("backup")
                    .build(),
            )
            .build();

        let snap: Snapshot = sdk_snap.into();
        assert_eq!(snap.id, "snap-abc123");
        assert_eq!(snap.volume_id, Some("vol-abc123".to_string()));
        assert_eq!(snap.volume_size, Some(100));
        assert_eq!(snap.state, Some("completed".to_string()));
        assert_eq!(snap.progress, Some("100%".to_string()));
        assert!(snap.start_time.is_some());
        assert_eq!(snap.description, Some("my snapshot".to_string()));
        assert_eq!(snap.owner_id, Some("123456789012".to_string()));
        assert!(snap.encrypted);
        assert_eq!(
            snap.kms_key_id,
            Some("arn:aws:kms:us-east-1:123:key/abc".to_string())
        );
        assert_eq!(snap.tags.len(), 1);
        assert_eq!(snap.tags[0].key, "Name");
        assert_eq!(snap.tags[0].value, "backup");
    }

    #[test]
    fn test_snapshot_from_sdk_empty() {
        let sdk_snap = aws_sdk_ec2::types::Snapshot::builder().build();
        let snap: Snapshot = sdk_snap.into();
        assert_eq!(snap.id, "");
        assert_eq!(snap.volume_id, None);
        assert_eq!(snap.volume_size, None);
        assert_eq!(snap.state, None);
        assert_eq!(snap.progress, None);
        assert_eq!(snap.start_time, None);
        assert_eq!(snap.description, None);
        assert_eq!(snap.owner_id, None);
        assert!(!snap.encrypted);
        assert_eq!(snap.kms_key_id, None);
        assert!(snap.tags.is_empty());
    }
}

impl From<aws_sdk_ec2::types::Instance> for Instance {
    fn from(i: aws_sdk_ec2::types::Instance) -> Self {
        let state = i
            .state()
            .and_then(|s| s.name())
            .map(|n| match n {
                aws_sdk_ec2::types::InstanceStateName::Pending => InstanceState::Pending,
                aws_sdk_ec2::types::InstanceStateName::Running => InstanceState::Running,
                aws_sdk_ec2::types::InstanceStateName::ShuttingDown => InstanceState::ShuttingDown,
                aws_sdk_ec2::types::InstanceStateName::Terminated => InstanceState::Terminated,
                aws_sdk_ec2::types::InstanceStateName::Stopping => InstanceState::Stopping,
                aws_sdk_ec2::types::InstanceStateName::Stopped => InstanceState::Stopped,
                _ => InstanceState::Running,
            })
            .unwrap_or(InstanceState::Running);

        let tags = i
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            })
            .collect();

        let security_groups = i
            .security_groups()
            .iter()
            .map(|sg| SecurityGroupRef {
                id: sg.group_id().unwrap_or_default().to_string(),
                name: sg.group_name().unwrap_or_default().to_string(),
            })
            .collect();

        let network_interfaces = i
            .network_interfaces()
            .iter()
            .map(|ni| NetworkInterface {
                id: ni.network_interface_id().unwrap_or_default().to_string(),
                subnet_id: ni.subnet_id().map(|s| s.to_string()),
                vpc_id: ni.vpc_id().map(|s| s.to_string()),
                private_ip: ni.private_ip_address().map(|s| s.to_string()),
                public_ip: ni
                    .association()
                    .and_then(|a| a.public_ip())
                    .map(|s| s.to_string()),
                mac_address: ni.mac_address().map(|s| s.to_string()),
            })
            .collect();

        let block_devices = i
            .block_device_mappings()
            .iter()
            .map(|bd| BlockDevice {
                device_name: bd.device_name().unwrap_or_default().to_string(),
                volume_id: bd.ebs().and_then(|e| e.volume_id()).map(|s| s.to_string()),
                status: bd
                    .ebs()
                    .and_then(|e| e.status())
                    .map(|s| s.as_str().to_string()),
                delete_on_termination: bd
                    .ebs()
                    .and_then(|e| e.delete_on_termination())
                    .unwrap_or(false),
            })
            .collect();

        let metadata_options = i.metadata_options().map(|m| InstanceMetadataOptions {
            http_tokens: m.http_tokens().map(|t| t.as_str().to_string()),
            http_endpoint: m.http_endpoint().map(|e| e.as_str().to_string()),
            http_put_response_hop_limit: m.http_put_response_hop_limit(),
            http_protocol_ipv6: m.http_protocol_ipv6().map(|p| p.as_str().to_string()),
            instance_metadata_tags: m.instance_metadata_tags().map(|t| t.as_str().to_string()),
        });

        Instance {
            id: i.instance_id().unwrap_or_default().to_string(),
            instance_type: i.instance_type().map(|t| t.as_str().to_string()),
            state,
            az: i
                .placement()
                .and_then(|p| p.availability_zone())
                .map(|s| s.to_string()),
            public_ip: i.public_ip_address().map(|s| s.to_string()),
            private_ip: i.private_ip_address().map(|s| s.to_string()),
            vpc_id: i.vpc_id().map(|s| s.to_string()),
            subnet_id: i.subnet_id().map(|s| s.to_string()),
            key_name: i.key_name().map(|s| s.to_string()),
            launch_time: i.launch_time().map(|dt| dt.to_string()),
            tags,
            security_groups,
            network_interfaces,
            block_devices,
            metadata_options,
        }
    }
}
