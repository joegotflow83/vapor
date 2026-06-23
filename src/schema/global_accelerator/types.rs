use async_graphql::SimpleObject;
use aws_sdk_globalaccelerator::types::{
    Accelerator as SdkAccelerator, EndpointGroup, Listener,
};

#[derive(SimpleObject, Clone)]
pub struct Accelerator {
    pub arn: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub enabled: bool,
    pub ip_address_type: Option<String>,
    pub ip_addresses: Vec<String>,
    pub dns_name: Option<String>,
    pub created_time: Option<String>,
}

impl From<SdkAccelerator> for Accelerator {
    fn from(a: SdkAccelerator) -> Self {
        let ip_addresses = a
            .ip_sets()
            .iter()
            .flat_map(|s| s.ip_addresses().iter().map(|ip| ip.to_string()))
            .collect();
        Self {
            arn: a.accelerator_arn().unwrap_or_default().to_string(),
            name: a.name().map(|s| s.to_string()),
            status: a.status().map(|s| s.as_str().to_string()),
            enabled: a.enabled().unwrap_or(false),
            ip_address_type: a.ip_address_type().map(|t| t.as_str().to_string()),
            ip_addresses,
            dns_name: a.dns_name().map(|s| s.to_string()),
            created_time: a.created_time().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GaListener {
    pub listener_arn: String,
    pub protocol: Option<String>,
    pub from_port: Option<i32>,
    pub to_port: Option<i32>,
}

impl From<Listener> for GaListener {
    fn from(l: Listener) -> Self {
        let first_range = l.port_ranges().first();
        Self {
            listener_arn: l.listener_arn().unwrap_or_default().to_string(),
            protocol: l.protocol().map(|p| p.as_str().to_string()),
            from_port: first_range.and_then(|r| r.from_port()),
            to_port: first_range.and_then(|r| r.to_port()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GaEndpointGroup {
    pub endpoint_group_arn: String,
    pub endpoint_group_region: Option<String>,
    pub health_check_protocol: Option<String>,
    pub traffic_dial_percentage: Option<f32>,
}

impl From<EndpointGroup> for GaEndpointGroup {
    fn from(g: EndpointGroup) -> Self {
        Self {
            endpoint_group_arn: g.endpoint_group_arn().unwrap_or_default().to_string(),
            endpoint_group_region: g.endpoint_group_region().map(|s| s.to_string()),
            health_check_protocol: g.health_check_protocol().map(|p| p.as_str().to_string()),
            traffic_dial_percentage: g.traffic_dial_percentage(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ga_accelerator_fields() {
        let acc = Accelerator {
            arn: "arn:aws:globalaccelerator::123456789012:accelerator/test".to_string(),
            name: Some("test-accelerator".to_string()),
            status: Some("DEPLOYED".to_string()),
            enabled: true,
            ip_address_type: Some("IPV4".to_string()),
            ip_addresses: vec!["1.2.3.4".to_string(), "5.6.7.8".to_string()],
            dns_name: Some("abc123.awsglobalaccelerator.com".to_string()),
            created_time: Some("2024-01-01T00:00:00Z".to_string()),
        };
        assert_eq!(acc.arn, "arn:aws:globalaccelerator::123456789012:accelerator/test");
        assert_eq!(acc.name, Some("test-accelerator".to_string()));
        assert_eq!(acc.status, Some("DEPLOYED".to_string()));
        assert!(acc.enabled);
        assert_eq!(acc.ip_addresses.len(), 2);
        assert_eq!(acc.ip_addresses[0], "1.2.3.4");
    }

    #[test]
    fn test_ga_accelerator_disabled() {
        let acc = Accelerator {
            arn: "arn:aws:globalaccelerator::123456789012:accelerator/disabled".to_string(),
            name: None,
            status: Some("IN_PROGRESS".to_string()),
            enabled: false,
            ip_address_type: None,
            ip_addresses: vec![],
            dns_name: None,
            created_time: None,
        };
        assert!(!acc.enabled);
        assert!(acc.ip_addresses.is_empty());
        assert!(acc.name.is_none());
    }

    #[test]
    fn test_ga_listener_fields() {
        let listener = GaListener {
            listener_arn: "arn:aws:globalaccelerator::123456789012:accelerator/test/listener/abc".to_string(),
            protocol: Some("TCP".to_string()),
            from_port: Some(80),
            to_port: Some(80),
        };
        assert_eq!(listener.protocol, Some("TCP".to_string()));
        assert_eq!(listener.from_port, Some(80));
        assert_eq!(listener.to_port, Some(80));
    }

    #[test]
    fn test_ga_listener_no_ports() {
        let listener = GaListener {
            listener_arn: "arn:aws:globalaccelerator::123456789012:accelerator/test/listener/xyz".to_string(),
            protocol: Some("UDP".to_string()),
            from_port: None,
            to_port: None,
        };
        assert!(listener.from_port.is_none());
        assert!(listener.to_port.is_none());
    }

    #[test]
    fn test_ga_endpoint_group_fields() {
        let group = GaEndpointGroup {
            endpoint_group_arn: "arn:aws:globalaccelerator::123456789012:accelerator/test/listener/abc/endpoint-group/us-east-1".to_string(),
            endpoint_group_region: Some("us-east-1".to_string()),
            health_check_protocol: Some("HTTP".to_string()),
            traffic_dial_percentage: Some(100.0),
        };
        assert_eq!(group.endpoint_group_region, Some("us-east-1".to_string()));
        assert_eq!(group.health_check_protocol, Some("HTTP".to_string()));
        assert_eq!(group.traffic_dial_percentage, Some(100.0));
    }

    #[test]
    fn test_ga_endpoint_group_minimal() {
        let group = GaEndpointGroup {
            endpoint_group_arn: "arn:aws:globalaccelerator::123456789012:accelerator/test/listener/abc/endpoint-group/us-west-2".to_string(),
            endpoint_group_region: None,
            health_check_protocol: None,
            traffic_dial_percentage: None,
        };
        assert!(group.endpoint_group_region.is_none());
        assert!(group.traffic_dial_percentage.is_none());
    }
}
