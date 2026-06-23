use async_graphql::SimpleObject;

use crate::schema::ec2::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct RouteTable {
    pub id: String,
    pub vpc_id: Option<String>,
    pub routes: Vec<Route>,
    pub associations: Vec<RouteTableAssociation>,
    pub tags: Vec<Tag>,
}

#[derive(SimpleObject, Clone)]
pub struct Route {
    pub destination_cidr: Option<String>,
    pub destination_ipv6_cidr: Option<String>,
    pub gateway_id: Option<String>,
    pub instance_id: Option<String>,
    pub nat_gateway_id: Option<String>,
    pub transit_gateway_id: Option<String>,
    pub vpc_endpoint_id: Option<String>,
    pub state: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct RouteTableAssociation {
    pub id: Option<String>,
    pub subnet_id: Option<String>,
    pub main: bool,
}

#[derive(SimpleObject, Clone)]
pub struct NetworkAcl {
    pub id: String,
    pub vpc_id: Option<String>,
    pub is_default: bool,
    pub entries: Vec<NaclEntry>,
    pub associations: Vec<String>,
    pub tags: Vec<Tag>,
}

#[derive(SimpleObject, Clone)]
pub struct NaclEntry {
    pub rule_number: Option<i32>,
    pub protocol: Option<String>,
    pub rule_action: Option<String>,
    pub egress: bool,
    pub cidr_block: Option<String>,
    pub ipv6_cidr_block: Option<String>,
    pub from_port: Option<i32>,
    pub to_port: Option<i32>,
}

#[derive(SimpleObject, Clone)]
pub struct InternetGateway {
    pub id: String,
    pub attachments: Vec<IgwAttachment>,
    pub tags: Vec<Tag>,
}

#[derive(SimpleObject, Clone)]
pub struct IgwAttachment {
    pub vpc_id: Option<String>,
    pub state: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct NatGateway {
    pub id: String,
    pub vpc_id: Option<String>,
    pub subnet_id: Option<String>,
    pub state: Option<String>,
    pub connectivity_type: Option<String>,
    pub public_ip: Option<String>,
    pub tags: Vec<Tag>,
}

#[derive(SimpleObject, Clone)]
pub struct VpcEndpoint {
    pub id: String,
    pub vpc_id: Option<String>,
    pub service_name: Option<String>,
    pub endpoint_type: Option<String>,
    pub state: Option<String>,
    pub subnet_ids: Vec<String>,
    pub route_table_ids: Vec<String>,
    pub tags: Vec<Tag>,
}

#[derive(SimpleObject, Clone)]
pub struct TransitGateway {
    pub id: String,
    pub arn: Option<String>,
    pub state: Option<String>,
    pub owner_id: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<Tag>,
}


impl From<aws_sdk_ec2::types::RouteTable> for RouteTable {
    fn from(rt: aws_sdk_ec2::types::RouteTable) -> Self {
        let tags = rt.tags().iter().map(|t| Tag {
            key: t.key().unwrap_or_default().to_string(),
            value: t.value().unwrap_or_default().to_string(),
        }).collect();
        let routes = rt.routes().iter().map(|r| {
            let gateway_id = r.gateway_id().map(|s| s.to_string());
            // The SDK exposes no dedicated `vpc_endpoint_id`; a gateway VPC
            // endpoint surfaces its `vpce-…` id in the route's `gateway_id`.
            let vpc_endpoint_id = gateway_id
                .as_deref()
                .filter(|g| g.starts_with("vpce-"))
                .map(|s| s.to_string());
            Route {
                destination_cidr: r.destination_cidr_block().map(|s| s.to_string()),
                destination_ipv6_cidr: r.destination_ipv6_cidr_block().map(|s| s.to_string()),
                gateway_id,
                instance_id: r.instance_id().map(|s| s.to_string()),
                nat_gateway_id: r.nat_gateway_id().map(|s| s.to_string()),
                transit_gateway_id: r.transit_gateway_id().map(|s| s.to_string()),
                vpc_endpoint_id,
                state: r.state().map(|s| s.as_str().to_string()),
            }
        }).collect();
        let associations = rt.associations().iter().map(|a| RouteTableAssociation {
            id: a.route_table_association_id().map(|s| s.to_string()),
            subnet_id: a.subnet_id().map(|s| s.to_string()),
            main: a.main().unwrap_or(false),
        }).collect();
        RouteTable { id: rt.route_table_id().unwrap_or_default().to_string(), vpc_id: rt.vpc_id().map(|s| s.to_string()), routes, associations, tags }
    }
}

impl From<aws_sdk_ec2::types::NetworkAcl> for NetworkAcl {
    fn from(nacl: aws_sdk_ec2::types::NetworkAcl) -> Self {
        let tags = nacl.tags().iter().map(|t| Tag {
            key: t.key().unwrap_or_default().to_string(),
            value: t.value().unwrap_or_default().to_string(),
        }).collect();
        let entries = nacl.entries().iter().map(|e| NaclEntry {
            rule_number: e.rule_number(),
            protocol: e.protocol().map(|s| s.to_string()),
            rule_action: e.rule_action().map(|s| s.as_str().to_string()),
            egress: e.egress().unwrap_or(false),
            cidr_block: e.cidr_block().map(|s| s.to_string()),
            ipv6_cidr_block: e.ipv6_cidr_block().map(|s| s.to_string()),
            from_port: e.port_range().and_then(|p| p.from()),
            to_port: e.port_range().and_then(|p| p.to()),
        }).collect();
        let associations = nacl.associations().iter().filter_map(|a| a.subnet_id().map(|s| s.to_string())).collect();
        NetworkAcl {
            id: nacl.network_acl_id().unwrap_or_default().to_string(),
            vpc_id: nacl.vpc_id().map(|s| s.to_string()),
            is_default: nacl.is_default().unwrap_or(false),
            entries, associations, tags,
        }
    }
}

impl From<aws_sdk_ec2::types::InternetGateway> for InternetGateway {
    fn from(igw: aws_sdk_ec2::types::InternetGateway) -> Self {
        let tags = igw.tags().iter().map(|t| Tag {
            key: t.key().unwrap_or_default().to_string(),
            value: t.value().unwrap_or_default().to_string(),
        }).collect();
        let attachments = igw.attachments().iter().map(|a| IgwAttachment {
            vpc_id: a.vpc_id().map(|s| s.to_string()),
            state: a.state().map(|s| s.as_str().to_string()),
        }).collect();
        InternetGateway { id: igw.internet_gateway_id().unwrap_or_default().to_string(), attachments, tags }
    }
}

impl From<aws_sdk_ec2::types::NatGateway> for NatGateway {
    fn from(ng: aws_sdk_ec2::types::NatGateway) -> Self {
        let tags = ng.tags().iter().map(|t| Tag {
            key: t.key().unwrap_or_default().to_string(),
            value: t.value().unwrap_or_default().to_string(),
        }).collect();
        let public_ip = ng.nat_gateway_addresses().iter().find_map(|a| a.public_ip().map(|s| s.to_string()));
        NatGateway {
            id: ng.nat_gateway_id().unwrap_or_default().to_string(),
            vpc_id: ng.vpc_id().map(|s| s.to_string()),
            subnet_id: ng.subnet_id().map(|s| s.to_string()),
            state: ng.state().map(|s| s.as_str().to_string()),
            connectivity_type: ng.connectivity_type().map(|s| s.as_str().to_string()),
            public_ip,
            tags,
        }
    }
}

impl From<aws_sdk_ec2::types::VpcEndpoint> for VpcEndpoint {
    fn from(ep: aws_sdk_ec2::types::VpcEndpoint) -> Self {
        let tags = ep.tags().iter().map(|t| Tag {
            key: t.key().unwrap_or_default().to_string(),
            value: t.value().unwrap_or_default().to_string(),
        }).collect();
        VpcEndpoint {
            id: ep.vpc_endpoint_id().unwrap_or_default().to_string(),
            vpc_id: ep.vpc_id().map(|s| s.to_string()),
            service_name: ep.service_name().map(|s| s.to_string()),
            endpoint_type: ep.vpc_endpoint_type().map(|s| s.as_str().to_string()),
            state: ep.state().map(|s| s.as_str().to_string()),
            subnet_ids: ep.subnet_ids().iter().map(|s| s.to_string()).collect(),
            route_table_ids: ep.route_table_ids().iter().map(|s| s.to_string()).collect(),
            tags,
        }
    }
}

impl From<aws_sdk_ec2::types::TransitGateway> for TransitGateway {
    fn from(tg: aws_sdk_ec2::types::TransitGateway) -> Self {
        let tags = tg.tags().iter().map(|t| Tag {
            key: t.key().unwrap_or_default().to_string(),
            value: t.value().unwrap_or_default().to_string(),
        }).collect();
        TransitGateway {
            id: tg.transit_gateway_id().unwrap_or_default().to_string(),
            arn: tg.transit_gateway_arn().map(|s| s.to_string()),
            state: tg.state().map(|s| s.as_str().to_string()),
            owner_id: tg.owner_id().map(|s| s.to_string()),
            description: tg.description().map(|s| s.to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct VpcFlowLog {
    pub flow_log_id: String,
    pub flow_log_status: Option<String>,
    pub resource_id: Option<String>,
    pub traffic_type: Option<String>,
    pub log_destination_type: Option<String>,
    pub log_destination: Option<String>,
    pub log_group_name: Option<String>,
    pub log_format: Option<String>,
    pub deliver_logs_status: Option<String>,
    pub deliver_logs_error_message: Option<String>,
    pub creation_time: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ec2::types::FlowLog> for VpcFlowLog {
    fn from(fl: aws_sdk_ec2::types::FlowLog) -> Self {
        let tags = fl.tags().iter().map(|t| Tag {
            key: t.key().unwrap_or_default().to_string(),
            value: t.value().unwrap_or_default().to_string(),
        }).collect();
        VpcFlowLog {
            flow_log_id: fl.flow_log_id().unwrap_or_default().to_string(),
            flow_log_status: fl.flow_log_status().map(|s| s.to_string()),
            resource_id: fl.resource_id().map(|s| s.to_string()),
            traffic_type: fl.traffic_type().map(|s| s.as_str().to_string()),
            log_destination_type: fl.log_destination_type().map(|s| s.as_str().to_string()),
            log_destination: fl.log_destination().map(|s| s.to_string()),
            log_group_name: fl.log_group_name().map(|s| s.to_string()),
            log_format: fl.log_format().map(|s| s.to_string()),
            deliver_logs_status: fl.deliver_logs_status().map(|s| s.to_string()),
            deliver_logs_error_message: fl.deliver_logs_error_message().map(|s| s.to_string()),
            creation_time: fl.creation_time().map(|t| t.to_string()),
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_table_from_sdk_populated() {
        let sdk = aws_sdk_ec2::types::RouteTable::builder()
            .route_table_id("rtb-123")
            .vpc_id("vpc-abc")
            .routes(aws_sdk_ec2::types::Route::builder().destination_cidr_block("0.0.0.0/0").gateway_id("igw-1").build())
            .associations(aws_sdk_ec2::types::RouteTableAssociation::builder().route_table_association_id("rtbassoc-1").main(true).build())
            .tags(aws_sdk_ec2::types::Tag::builder().key("Name").value("main").build())
            .build();
        let rt: RouteTable = sdk.into();
        assert_eq!(rt.id, "rtb-123");
        assert_eq!(rt.vpc_id, Some("vpc-abc".to_string()));
        assert_eq!(rt.routes.len(), 1);
        assert_eq!(rt.routes[0].destination_cidr, Some("0.0.0.0/0".to_string()));
        assert_eq!(rt.routes[0].gateway_id, Some("igw-1".to_string()));
        assert_eq!(rt.associations.len(), 1);
        assert!(rt.associations[0].main);
        assert_eq!(rt.tags.len(), 1);
    }

    #[test]
    fn test_route_table_from_sdk_empty() {
        let sdk = aws_sdk_ec2::types::RouteTable::builder().build();
        let rt: RouteTable = sdk.into();
        assert_eq!(rt.id, "");
        assert_eq!(rt.vpc_id, None);
        assert!(rt.routes.is_empty());
        assert!(rt.associations.is_empty());
        assert!(rt.tags.is_empty());
    }

    #[test]
    fn test_network_acl_from_sdk_populated() {
        let sdk = aws_sdk_ec2::types::NetworkAcl::builder()
            .network_acl_id("acl-123")
            .vpc_id("vpc-abc")
            .is_default(true)
            .entries(aws_sdk_ec2::types::NetworkAclEntry::builder().rule_number(100).protocol("-1").egress(false).cidr_block("0.0.0.0/0").rule_action(aws_sdk_ec2::types::RuleAction::Allow).build())
            .associations(aws_sdk_ec2::types::NetworkAclAssociation::builder().subnet_id("subnet-1").build())
            .tags(aws_sdk_ec2::types::Tag::builder().key("Env").value("prod").build())
            .build();
        let nacl: NetworkAcl = sdk.into();
        assert_eq!(nacl.id, "acl-123");
        assert!(nacl.is_default);
        assert_eq!(nacl.entries.len(), 1);
        assert_eq!(nacl.entries[0].rule_number, Some(100));
        assert!(!nacl.entries[0].egress);
        assert_eq!(nacl.associations, vec!["subnet-1".to_string()]);
    }

    #[test]
    fn test_network_acl_from_sdk_empty() {
        let sdk = aws_sdk_ec2::types::NetworkAcl::builder().build();
        let nacl: NetworkAcl = sdk.into();
        assert_eq!(nacl.id, "");
        assert!(!nacl.is_default);
        assert!(nacl.entries.is_empty());
        assert!(nacl.associations.is_empty());
    }

    #[test]
    fn test_internet_gateway_from_sdk_populated() {
        let sdk = aws_sdk_ec2::types::InternetGateway::builder()
            .internet_gateway_id("igw-123")
            .attachments(aws_sdk_ec2::types::InternetGatewayAttachment::builder().vpc_id("vpc-abc").state(aws_sdk_ec2::types::AttachmentStatus::Attached).build())
            .tags(aws_sdk_ec2::types::Tag::builder().key("Name").value("main-igw").build())
            .build();
        let igw: InternetGateway = sdk.into();
        assert_eq!(igw.id, "igw-123");
        assert_eq!(igw.attachments.len(), 1);
        assert_eq!(igw.attachments[0].vpc_id, Some("vpc-abc".to_string()));
        assert_eq!(igw.attachments[0].state, Some("attached".to_string()));
    }

    #[test]
    fn test_internet_gateway_from_sdk_empty() {
        let sdk = aws_sdk_ec2::types::InternetGateway::builder().build();
        let igw: InternetGateway = sdk.into();
        assert_eq!(igw.id, "");
        assert!(igw.attachments.is_empty());
        assert!(igw.tags.is_empty());
    }

    #[test]
    fn test_nat_gateway_from_sdk_populated() {
        let sdk = aws_sdk_ec2::types::NatGateway::builder()
            .nat_gateway_id("nat-123")
            .vpc_id("vpc-abc")
            .subnet_id("subnet-1")
            .state(aws_sdk_ec2::types::NatGatewayState::Available)
            .connectivity_type(aws_sdk_ec2::types::ConnectivityType::Public)
            .nat_gateway_addresses(aws_sdk_ec2::types::NatGatewayAddress::builder().public_ip("1.2.3.4").build())
            .tags(aws_sdk_ec2::types::Tag::builder().key("Name").value("nat").build())
            .build();
        let ng: NatGateway = sdk.into();
        assert_eq!(ng.id, "nat-123");
        assert_eq!(ng.vpc_id, Some("vpc-abc".to_string()));
        assert_eq!(ng.state, Some("available".to_string()));
        assert_eq!(ng.public_ip, Some("1.2.3.4".to_string()));
    }

    #[test]
    fn test_nat_gateway_from_sdk_empty() {
        let sdk = aws_sdk_ec2::types::NatGateway::builder().build();
        let ng: NatGateway = sdk.into();
        assert_eq!(ng.id, "");
        assert_eq!(ng.public_ip, None);
    }

    #[test]
    fn test_vpc_endpoint_from_sdk_populated() {
        let sdk = aws_sdk_ec2::types::VpcEndpoint::builder()
            .vpc_endpoint_id("vpce-123")
            .vpc_id("vpc-abc")
            .service_name("com.amazonaws.us-east-1.s3")
            .vpc_endpoint_type(aws_sdk_ec2::types::VpcEndpointType::Gateway)
            .state(aws_sdk_ec2::types::State::Available)
            .subnet_ids("subnet-1")
            .route_table_ids("rtb-1")
            .tags(aws_sdk_ec2::types::Tag::builder().key("Name").value("s3-ep").build())
            .build();
        let ep: VpcEndpoint = sdk.into();
        assert_eq!(ep.id, "vpce-123");
        assert_eq!(ep.service_name, Some("com.amazonaws.us-east-1.s3".to_string()));
        assert_eq!(ep.endpoint_type, Some("Gateway".to_string()));
        assert_eq!(ep.subnet_ids, vec!["subnet-1".to_string()]);
        assert_eq!(ep.route_table_ids, vec!["rtb-1".to_string()]);
    }

    #[test]
    fn test_vpc_endpoint_from_sdk_empty() {
        let sdk = aws_sdk_ec2::types::VpcEndpoint::builder().build();
        let ep: VpcEndpoint = sdk.into();
        assert_eq!(ep.id, "");
        assert!(ep.subnet_ids.is_empty());
        assert!(ep.route_table_ids.is_empty());
    }

    #[test]
    fn test_transit_gateway_from_sdk_populated() {
        let sdk = aws_sdk_ec2::types::TransitGateway::builder()
            .transit_gateway_id("tgw-123")
            .transit_gateway_arn("arn:aws:ec2:us-east-1:123456789012:transit-gateway/tgw-123")
            .state(aws_sdk_ec2::types::TransitGatewayState::Available)
            .owner_id("123456789012")
            .description("Main TGW")
            .tags(aws_sdk_ec2::types::Tag::builder().key("Name").value("main-tgw").build())
            .build();
        let tg: TransitGateway = sdk.into();
        assert_eq!(tg.id, "tgw-123");
        assert_eq!(tg.state, Some("available".to_string()));
        assert_eq!(tg.owner_id, Some("123456789012".to_string()));
        assert_eq!(tg.description, Some("Main TGW".to_string()));
    }

    #[test]
    fn test_transit_gateway_from_sdk_empty() {
        let sdk = aws_sdk_ec2::types::TransitGateway::builder().build();
        let tg: TransitGateway = sdk.into();
        assert_eq!(tg.id, "");
        assert_eq!(tg.arn, None);
        assert_eq!(tg.state, None);
    }

    #[test]
    fn test_vpc_flow_log_from_sdk_populated() {
        let sdk = aws_sdk_ec2::types::FlowLog::builder()
            .flow_log_id("fl-12345")
            .flow_log_status("ACTIVE")
            .resource_id("vpc-abc")
            .traffic_type(aws_sdk_ec2::types::TrafficType::All)
            .log_destination_type(aws_sdk_ec2::types::LogDestinationType::S3)
            .log_destination("arn:aws:s3:::my-bucket/flow-logs/")
            .log_group_name("my-log-group")
            .log_format("${version} ${account-id} ${interface-id}")
            .deliver_logs_status("SUCCESS")
            .tags(aws_sdk_ec2::types::Tag::builder().key("Name").value("my-flow-log").build())
            .build();
        let fl: VpcFlowLog = sdk.into();
        assert_eq!(fl.flow_log_id, "fl-12345");
        assert_eq!(fl.flow_log_status, Some("ACTIVE".to_string()));
        assert_eq!(fl.resource_id, Some("vpc-abc".to_string()));
        assert!(fl.traffic_type.is_some());
        assert!(fl.log_destination_type.is_some());
        assert_eq!(fl.log_destination, Some("arn:aws:s3:::my-bucket/flow-logs/".to_string()));
        assert_eq!(fl.log_group_name, Some("my-log-group".to_string()));
        assert_eq!(fl.deliver_logs_status, Some("SUCCESS".to_string()));
        assert_eq!(fl.tags.len(), 1);
        assert_eq!(fl.tags[0].key, "Name");
        assert_eq!(fl.tags[0].value, "my-flow-log");
    }

    #[test]
    fn test_vpc_flow_log_from_sdk_empty() {
        let sdk = aws_sdk_ec2::types::FlowLog::builder().build();
        let fl: VpcFlowLog = sdk.into();
        assert_eq!(fl.flow_log_id, "");
        assert_eq!(fl.flow_log_status, None);
        assert_eq!(fl.resource_id, None);
        assert_eq!(fl.traffic_type, None);
        assert_eq!(fl.log_destination, None);
        assert_eq!(fl.deliver_logs_error_message, None);
        assert!(fl.tags.is_empty());
    }
}
