use async_graphql::SimpleObject;

use crate::aws::keyspaces::{
    KeyspacesCapacitySpecInfo, KeyspacesEncryptionInfo, KeyspacesKeyspaceInfo, KeyspacesTableInfo,
};

#[derive(SimpleObject, Clone)]
pub struct KeyspacesKeyspace {
    pub keyspace_name: String,
    pub resource_arn: String,
    pub replication_strategy: Option<String>,
    pub replication_regions: Vec<String>,
}

impl From<KeyspacesKeyspaceInfo> for KeyspacesKeyspace {
    fn from(k: KeyspacesKeyspaceInfo) -> Self {
        Self {
            keyspace_name: k.keyspace_name,
            resource_arn: k.resource_arn,
            replication_strategy: k.replication_strategy,
            replication_regions: k.replication_regions,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct KeyspacesCapacitySpec {
    pub throughput_mode: String,
    pub read_capacity_units: Option<i64>,
    pub write_capacity_units: Option<i64>,
}

impl From<KeyspacesCapacitySpecInfo> for KeyspacesCapacitySpec {
    fn from(c: KeyspacesCapacitySpecInfo) -> Self {
        Self {
            throughput_mode: c.throughput_mode,
            read_capacity_units: c.read_capacity_units,
            write_capacity_units: c.write_capacity_units,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct KeyspacesEncryption {
    pub type_: String,
    pub kms_key_identifier: Option<String>,
}

impl From<KeyspacesEncryptionInfo> for KeyspacesEncryption {
    fn from(e: KeyspacesEncryptionInfo) -> Self {
        Self {
            type_: e.type_,
            kms_key_identifier: e.kms_key_identifier,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct KeyspacesTable {
    pub keyspace_name: String,
    pub table_name: String,
    pub resource_arn: String,
    pub status: Option<String>,
    pub creation_timestamp: Option<String>,
    pub capacity_specification: Option<KeyspacesCapacitySpec>,
    pub encryption_specification: Option<KeyspacesEncryption>,
    pub point_in_time_recovery: Option<bool>,
}

impl From<KeyspacesTableInfo> for KeyspacesTable {
    fn from(t: KeyspacesTableInfo) -> Self {
        Self {
            keyspace_name: t.keyspace_name,
            table_name: t.table_name,
            resource_arn: t.resource_arn,
            status: t.status,
            creation_timestamp: t.creation_timestamp,
            capacity_specification: t.capacity_specification.map(KeyspacesCapacitySpec::from),
            encryption_specification: t.encryption_specification.map(KeyspacesEncryption::from),
            point_in_time_recovery: t.point_in_time_recovery,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::keyspaces::{
        KeyspacesCapacitySpecInfo, KeyspacesEncryptionInfo, KeyspacesKeyspaceInfo,
        KeyspacesTableInfo,
    };

    #[test]
    fn test_keyspace_from_full() {
        let info = KeyspacesKeyspaceInfo {
            keyspace_name: "mykeyspace".to_string(),
            resource_arn: "arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace"
                .to_string(),
            replication_strategy: Some("SINGLE_REGION".to_string()),
            replication_regions: vec![],
        };
        let result = KeyspacesKeyspace::from(info);
        assert_eq!(result.keyspace_name, "mykeyspace");
        assert_eq!(
            result.resource_arn,
            "arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace"
        );
        assert_eq!(result.replication_strategy, Some("SINGLE_REGION".to_string()));
        assert!(result.replication_regions.is_empty());
    }

    #[test]
    fn test_keyspace_from_multi_region() {
        let info = KeyspacesKeyspaceInfo {
            keyspace_name: "global-ks".to_string(),
            resource_arn: "arn:aws:cassandra:us-east-1:123456789012:/keyspace/global-ks"
                .to_string(),
            replication_strategy: Some("MULTI_REGION".to_string()),
            replication_regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
        };
        let result = KeyspacesKeyspace::from(info);
        assert_eq!(result.replication_strategy, Some("MULTI_REGION".to_string()));
        assert_eq!(result.replication_regions.len(), 2);
    }

    #[test]
    fn test_capacity_spec_from() {
        let info = KeyspacesCapacitySpecInfo {
            throughput_mode: "PROVISIONED".to_string(),
            read_capacity_units: Some(100),
            write_capacity_units: Some(50),
        };
        let result = KeyspacesCapacitySpec::from(info);
        assert_eq!(result.throughput_mode, "PROVISIONED");
        assert_eq!(result.read_capacity_units, Some(100));
        assert_eq!(result.write_capacity_units, Some(50));
    }

    #[test]
    fn test_capacity_spec_pay_per_request() {
        let info = KeyspacesCapacitySpecInfo {
            throughput_mode: "PAY_PER_REQUEST".to_string(),
            read_capacity_units: None,
            write_capacity_units: None,
        };
        let result = KeyspacesCapacitySpec::from(info);
        assert_eq!(result.throughput_mode, "PAY_PER_REQUEST");
        assert!(result.read_capacity_units.is_none());
        assert!(result.write_capacity_units.is_none());
    }

    #[test]
    fn test_encryption_from() {
        let info = KeyspacesEncryptionInfo {
            type_: "CUSTOMER_MANAGED_KMS_KEY".to_string(),
            kms_key_identifier: Some(
                "arn:aws:kms:us-east-1:123456789012:key/abc123".to_string(),
            ),
        };
        let result = KeyspacesEncryption::from(info);
        assert_eq!(result.type_, "CUSTOMER_MANAGED_KMS_KEY");
        assert!(result.kms_key_identifier.is_some());
    }

    #[test]
    fn test_encryption_aws_owned() {
        let info = KeyspacesEncryptionInfo {
            type_: "AWS_OWNED_KMS_KEY".to_string(),
            kms_key_identifier: None,
        };
        let result = KeyspacesEncryption::from(info);
        assert_eq!(result.type_, "AWS_OWNED_KMS_KEY");
        assert!(result.kms_key_identifier.is_none());
    }

    #[test]
    fn test_table_from_full() {
        let info = KeyspacesTableInfo {
            keyspace_name: "mykeyspace".to_string(),
            table_name: "mytable".to_string(),
            resource_arn: "arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/mytable"
                .to_string(),
            status: Some("ACTIVE".to_string()),
            creation_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            capacity_specification: Some(KeyspacesCapacitySpecInfo {
                throughput_mode: "PAY_PER_REQUEST".to_string(),
                read_capacity_units: None,
                write_capacity_units: None,
            }),
            encryption_specification: Some(KeyspacesEncryptionInfo {
                type_: "AWS_OWNED_KMS_KEY".to_string(),
                kms_key_identifier: None,
            }),
            point_in_time_recovery: Some(true),
        };
        let result = KeyspacesTable::from(info);
        assert_eq!(result.keyspace_name, "mykeyspace");
        assert_eq!(result.table_name, "mytable");
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert!(result.capacity_specification.is_some());
        assert!(result.encryption_specification.is_some());
        assert_eq!(result.point_in_time_recovery, Some(true));
    }

    #[test]
    fn test_table_from_minimal() {
        let info = KeyspacesTableInfo {
            keyspace_name: "ks".to_string(),
            table_name: "tbl".to_string(),
            resource_arn: "arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks/table/tbl"
                .to_string(),
            status: None,
            creation_timestamp: None,
            capacity_specification: None,
            encryption_specification: None,
            point_in_time_recovery: None,
        };
        let result = KeyspacesTable::from(info);
        assert_eq!(result.keyspace_name, "ks");
        assert_eq!(result.table_name, "tbl");
        assert!(result.status.is_none());
        assert!(result.capacity_specification.is_none());
        assert!(result.encryption_specification.is_none());
        assert!(result.point_in_time_recovery.is_none());
    }
}
