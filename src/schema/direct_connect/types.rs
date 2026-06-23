use async_graphql::SimpleObject;
use aws_sdk_directconnect::types::{Connection, VirtualInterface};

#[derive(SimpleObject, Clone)]
pub struct DxConnection {
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub connection_state: Option<String>,
    pub location: Option<String>,
    pub bandwidth: Option<String>,
    pub vlan: Option<i32>,
    pub partner_name: Option<String>,
    pub region: Option<String>,
    pub has_logical_redundancy: Option<String>,
}

impl From<Connection> for DxConnection {
    fn from(c: Connection) -> Self {
        Self {
            connection_id: c.connection_id().unwrap_or_default().to_string(),
            connection_name: c.connection_name().map(|s| s.to_string()),
            connection_state: c.connection_state().map(|s| s.as_str().to_string()),
            location: c.location().map(|s| s.to_string()),
            bandwidth: c.bandwidth().map(|s| s.to_string()),
            vlan: if c.vlan() == 0 { None } else { Some(c.vlan()) },
            partner_name: c.partner_name().map(|s| s.to_string()),
            region: c.region().map(|s| s.to_string()),
            has_logical_redundancy: c
                .has_logical_redundancy()
                .map(|s| s.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DxVirtualInterface {
    pub virtual_interface_id: String,
    pub virtual_interface_name: Option<String>,
    pub virtual_interface_type: Option<String>,
    pub virtual_interface_state: Option<String>,
    pub connection_id: Option<String>,
    pub vlan: Option<i32>,
    pub asn: Option<i32>,
    pub amazon_address: Option<String>,
    pub customer_address: Option<String>,
}

impl From<VirtualInterface> for DxVirtualInterface {
    fn from(v: VirtualInterface) -> Self {
        Self {
            virtual_interface_id: v.virtual_interface_id().unwrap_or_default().to_string(),
            virtual_interface_name: v.virtual_interface_name().map(|s| s.to_string()),
            virtual_interface_type: v.virtual_interface_type().map(|s| s.to_string()),
            virtual_interface_state: v
                .virtual_interface_state()
                .map(|s| s.as_str().to_string()),
            connection_id: v.connection_id().map(|s| s.to_string()),
            vlan: if v.vlan() == 0 { None } else { Some(v.vlan()) },
            asn: Some(v.asn() as i32),
            amazon_address: v.amazon_address().map(|s| s.to_string()),
            customer_address: v.customer_address().map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dx_connection_fields() {
        let conn = DxConnection {
            connection_id: "dxcon-abc123".to_string(),
            connection_name: Some("my-dx-connection".to_string()),
            connection_state: Some("available".to_string()),
            location: Some("EqSE2".to_string()),
            bandwidth: Some("1Gbps".to_string()),
            vlan: Some(100),
            partner_name: Some("Equinix".to_string()),
            region: Some("us-east-1".to_string()),
            has_logical_redundancy: Some("yes".to_string()),
        };
        assert_eq!(conn.connection_id, "dxcon-abc123");
        assert_eq!(conn.connection_name, Some("my-dx-connection".to_string()));
        assert_eq!(conn.connection_state, Some("available".to_string()));
        assert_eq!(conn.bandwidth, Some("1Gbps".to_string()));
        assert_eq!(conn.vlan, Some(100));
        assert_eq!(conn.has_logical_redundancy, Some("yes".to_string()));
    }

    #[test]
    fn test_dx_connection_minimal() {
        let conn = DxConnection {
            connection_id: "dxcon-xyz".to_string(),
            connection_name: None,
            connection_state: Some("ordering".to_string()),
            location: None,
            bandwidth: None,
            vlan: None,
            partner_name: None,
            region: None,
            has_logical_redundancy: None,
        };
        assert_eq!(conn.connection_id, "dxcon-xyz");
        assert!(conn.connection_name.is_none());
        assert!(conn.partner_name.is_none());
        assert!(conn.has_logical_redundancy.is_none());
    }

    #[test]
    fn test_dx_virtual_interface_fields() {
        let vif = DxVirtualInterface {
            virtual_interface_id: "dxvif-abc123".to_string(),
            virtual_interface_name: Some("private-vif".to_string()),
            virtual_interface_type: Some("private".to_string()),
            virtual_interface_state: Some("available".to_string()),
            connection_id: Some("dxcon-abc123".to_string()),
            vlan: Some(200),
            asn: Some(65000),
            amazon_address: Some("192.168.1.1/30".to_string()),
            customer_address: Some("192.168.1.2/30".to_string()),
        };
        assert_eq!(vif.virtual_interface_id, "dxvif-abc123");
        assert_eq!(vif.virtual_interface_type, Some("private".to_string()));
        assert_eq!(vif.virtual_interface_state, Some("available".to_string()));
        assert_eq!(vif.vlan, Some(200));
        assert_eq!(vif.asn, Some(65000));
        assert_eq!(vif.amazon_address, Some("192.168.1.1/30".to_string()));
    }

    #[test]
    fn test_dx_virtual_interface_minimal() {
        let vif = DxVirtualInterface {
            virtual_interface_id: "dxvif-xyz".to_string(),
            virtual_interface_name: None,
            virtual_interface_type: Some("transit".to_string()),
            virtual_interface_state: Some("pending".to_string()),
            connection_id: None,
            vlan: None,
            asn: None,
            amazon_address: None,
            customer_address: None,
        };
        assert!(vif.virtual_interface_name.is_none());
        assert!(vif.connection_id.is_none());
        assert!(vif.asn.is_none());
        assert!(vif.amazon_address.is_none());
    }
}
