use async_graphql::SimpleObject;

use crate::aws::lightsail::{
    LightsailDatabaseInfo, LightsailEndpointInfo, LightsailInstanceHealthInfo,
    LightsailInstanceInfo, LightsailLoadBalancerInfo, LightsailStaticIpInfo,
};
use crate::schema::common::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct LightsailInstance {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub blueprint_id: Option<String>,
    pub bundle_id: Option<String>,
    pub state: Option<String>,
    pub public_ip_address: Option<String>,
    pub private_ip_address: Option<String>,
    pub location: Option<String>,
    pub created_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<LightsailInstanceInfo> for LightsailInstance {
    fn from(i: LightsailInstanceInfo) -> Self {
        Self {
            name: i.name,
            arn: i.arn,
            blueprint_id: i.blueprint_id,
            bundle_id: i.bundle_id,
            state: i.state,
            public_ip_address: i.public_ip_address,
            private_ip_address: i.private_ip_address,
            location: i.location,
            created_at: i.created_at,
            tags: i.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LightsailEndpoint {
    pub port: i32,
    pub address: String,
}

impl From<LightsailEndpointInfo> for LightsailEndpoint {
    fn from(e: LightsailEndpointInfo) -> Self {
        Self {
            port: e.port,
            address: e.address,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LightsailDatabase {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    pub state: Option<String>,
    pub master_username: Option<String>,
    pub master_endpoint: Option<LightsailEndpoint>,
    pub created_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<LightsailDatabaseInfo> for LightsailDatabase {
    fn from(d: LightsailDatabaseInfo) -> Self {
        Self {
            name: d.name,
            arn: d.arn,
            engine: d.engine,
            engine_version: d.engine_version,
            state: d.state,
            master_username: d.master_username,
            master_endpoint: d.master_endpoint.map(LightsailEndpoint::from),
            created_at: d.created_at,
            tags: d.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LightsailInstanceHealth {
    pub instance_name: Option<String>,
    pub instance_health: Option<String>,
}

impl From<LightsailInstanceHealthInfo> for LightsailInstanceHealth {
    fn from(h: LightsailInstanceHealthInfo) -> Self {
        Self {
            instance_name: h.instance_name,
            instance_health: h.instance_health,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LightsailLoadBalancer {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub dns_name: Option<String>,
    pub state: Option<String>,
    pub protocol: Option<String>,
    pub instance_port: Option<i32>,
    pub instance_health_summary: Vec<LightsailInstanceHealth>,
    pub created_at: Option<String>,
}

impl From<LightsailLoadBalancerInfo> for LightsailLoadBalancer {
    fn from(lb: LightsailLoadBalancerInfo) -> Self {
        Self {
            name: lb.name,
            arn: lb.arn,
            dns_name: lb.dns_name,
            state: lb.state,
            protocol: lb.protocol,
            instance_port: lb.instance_port,
            instance_health_summary: lb
                .instance_health_summary
                .into_iter()
                .map(LightsailInstanceHealth::from)
                .collect(),
            created_at: lb.created_at,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LightsailStaticIp {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub ip_address: Option<String>,
    pub attached_to: Option<String>,
    pub is_attached: bool,
}

impl From<LightsailStaticIpInfo> for LightsailStaticIp {
    fn from(ip: LightsailStaticIpInfo) -> Self {
        Self {
            name: ip.name,
            arn: ip.arn,
            ip_address: ip.ip_address,
            attached_to: ip.attached_to,
            is_attached: ip.is_attached,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::lightsail::{
        LightsailDatabaseInfo, LightsailEndpointInfo, LightsailInstanceHealthInfo,
        LightsailInstanceInfo, LightsailLoadBalancerInfo, LightsailStaticIpInfo,
    };

    #[test]
    fn test_instance_from() {
        let info = LightsailInstanceInfo {
            name: Some("my-instance".to_string()),
            arn: Some("arn:aws:lightsail:us-east-1:123:Instance/my-instance".to_string()),
            blueprint_id: Some("amazon_linux_2".to_string()),
            bundle_id: Some("nano_2_0".to_string()),
            state: Some("running".to_string()),
            public_ip_address: Some("1.2.3.4".to_string()),
            private_ip_address: Some("10.0.0.5".to_string()),
            location: Some("us-east-1a".to_string()),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            tags: vec![("Env".to_string(), "prod".to_string())],
        };
        let result = LightsailInstance::from(info);
        assert_eq!(result.name, Some("my-instance".to_string()));
        assert_eq!(result.state, Some("running".to_string()));
        assert_eq!(result.public_ip_address, Some("1.2.3.4".to_string()));
        assert_eq!(result.location, Some("us-east-1a".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_instance_minimal() {
        let info = LightsailInstanceInfo {
            name: None,
            arn: None,
            blueprint_id: None,
            bundle_id: None,
            state: None,
            public_ip_address: None,
            private_ip_address: None,
            location: None,
            created_at: None,
            tags: vec![],
        };
        let result = LightsailInstance::from(info);
        assert!(result.name.is_none());
        assert!(result.state.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_database_from() {
        let info = LightsailDatabaseInfo {
            name: Some("my-db".to_string()),
            arn: Some("arn:aws:lightsail:us-east-1:123:RelationalDatabase/my-db".to_string()),
            engine: Some("mysql".to_string()),
            engine_version: Some("8.0.28".to_string()),
            state: Some("available".to_string()),
            master_username: Some("admin".to_string()),
            master_endpoint: Some(LightsailEndpointInfo {
                port: 3306,
                address: "my-db.abc123.us-east-1.rds.amazonaws.com".to_string(),
            }),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            tags: vec![],
        };
        let result = LightsailDatabase::from(info);
        assert_eq!(result.name, Some("my-db".to_string()));
        assert_eq!(result.engine, Some("mysql".to_string()));
        assert_eq!(result.state, Some("available".to_string()));
        let ep = result.master_endpoint.unwrap();
        assert_eq!(ep.port, 3306);
        assert_eq!(ep.address, "my-db.abc123.us-east-1.rds.amazonaws.com");
    }

    #[test]
    fn test_database_no_endpoint() {
        let info = LightsailDatabaseInfo {
            name: Some("my-db-2".to_string()),
            arn: None,
            engine: None,
            engine_version: None,
            state: None,
            master_username: None,
            master_endpoint: None,
            created_at: None,
            tags: vec![],
        };
        let result = LightsailDatabase::from(info);
        assert!(result.master_endpoint.is_none());
    }

    #[test]
    fn test_load_balancer_from() {
        let info = LightsailLoadBalancerInfo {
            name: Some("my-lb".to_string()),
            arn: Some("arn:aws:lightsail:us-east-1:123:LoadBalancer/my-lb".to_string()),
            dns_name: Some("my-lb.us-east-1.elb.amazonaws.com".to_string()),
            state: Some("active".to_string()),
            protocol: Some("HTTP_HTTPS".to_string()),
            instance_port: Some(80),
            instance_health_summary: vec![LightsailInstanceHealthInfo {
                instance_name: Some("my-instance".to_string()),
                instance_health: Some("healthy".to_string()),
            }],
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
        };
        let result = LightsailLoadBalancer::from(info);
        assert_eq!(result.name, Some("my-lb".to_string()));
        assert_eq!(result.instance_port, Some(80));
        assert_eq!(result.instance_health_summary.len(), 1);
        assert_eq!(
            result.instance_health_summary[0].instance_name,
            Some("my-instance".to_string())
        );
        assert_eq!(
            result.instance_health_summary[0].instance_health,
            Some("healthy".to_string())
        );
    }

    #[test]
    fn test_static_ip_from() {
        let info = LightsailStaticIpInfo {
            name: Some("my-static-ip".to_string()),
            arn: Some("arn:aws:lightsail:us-east-1:123:StaticIp/my-static-ip".to_string()),
            ip_address: Some("1.2.3.4".to_string()),
            attached_to: Some("my-instance".to_string()),
            is_attached: true,
        };
        let result = LightsailStaticIp::from(info);
        assert_eq!(result.name, Some("my-static-ip".to_string()));
        assert_eq!(result.ip_address, Some("1.2.3.4".to_string()));
        assert_eq!(result.attached_to, Some("my-instance".to_string()));
        assert!(result.is_attached);
    }

    #[test]
    fn test_static_ip_unattached() {
        let info = LightsailStaticIpInfo {
            name: Some("free-ip".to_string()),
            arn: None,
            ip_address: Some("5.6.7.8".to_string()),
            attached_to: None,
            is_attached: false,
        };
        let result = LightsailStaticIp::from(info);
        assert!(result.attached_to.is_none());
        assert!(!result.is_attached);
    }
}
