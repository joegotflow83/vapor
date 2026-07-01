use async_graphql::SimpleObject;

use crate::schema::common::types::Tag;

// === Helper Types ===

#[derive(SimpleObject, Clone)]
pub struct DbSecurityGroupRef {
    pub name: Option<String>,
    pub status: Option<String>,
}

impl From<&aws_sdk_rds::types::DbSecurityGroupMembership> for DbSecurityGroupRef {
    fn from(sg: &aws_sdk_rds::types::DbSecurityGroupMembership) -> Self {
        Self {
            name: sg.db_security_group_name().map(|s| s.to_string()),
            status: sg.status().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DbVpcSecurityGroup {
    pub vpc_security_group_id: Option<String>,
    pub status: Option<String>,
}

impl From<&aws_sdk_rds::types::VpcSecurityGroupMembership> for DbVpcSecurityGroup {
    fn from(sg: &aws_sdk_rds::types::VpcSecurityGroupMembership) -> Self {
        Self {
            vpc_security_group_id: sg.vpc_security_group_id().map(|s| s.to_string()),
            status: sg.status().map(|s| s.to_string()),
        }
    }
}

// === Output Types ===

#[derive(SimpleObject, Clone)]
pub struct DbInstance {
    pub id: String,
    pub arn: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    pub status: Option<String>,
    pub instance_class: Option<String>,
    pub multi_az: bool,
    pub publicly_accessible: bool,
    pub storage_encrypted: bool,
    pub allocated_storage: Option<i32>,
    pub endpoint_address: Option<String>,
    pub endpoint_port: Option<i32>,
    pub vpc_id: Option<String>,
    pub az: Option<String>,
    pub db_name: Option<String>,
    pub master_username: Option<String>,
    pub backup_retention_period: Option<i32>,
    pub preferred_backup_window: Option<String>,
    pub preferred_maintenance_window: Option<String>,
    pub subnet_group: Option<String>,
    pub created_time: Option<String>,
    pub security_groups: Vec<DbVpcSecurityGroup>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_rds::types::DbInstance> for DbInstance {
    fn from(db: aws_sdk_rds::types::DbInstance) -> Self {
        Self {
            id: db.db_instance_identifier().unwrap_or_default().to_string(),
            arn: db.db_instance_arn().map(|s| s.to_string()),
            engine: db.engine().map(|s| s.to_string()),
            engine_version: db.engine_version().map(|s| s.to_string()),
            status: db.db_instance_status().map(|s| s.to_string()),
            instance_class: db.db_instance_class().map(|s| s.to_string()),
            multi_az: db.multi_az().unwrap_or(false),
            publicly_accessible: db.publicly_accessible().unwrap_or(false),
            storage_encrypted: db.storage_encrypted().unwrap_or(false),
            allocated_storage: db.allocated_storage(),
            endpoint_address: db.endpoint().and_then(|e| e.address()).map(|s| s.to_string()),
            endpoint_port: db.endpoint().and_then(|e| e.port()),
            vpc_id: db
                .db_subnet_group()
                .and_then(|s| s.vpc_id())
                .map(|s| s.to_string()),
            az: db.availability_zone().map(|s| s.to_string()),
            db_name: db.db_name().map(|s| s.to_string()),
            master_username: db.master_username().map(|s| s.to_string()),
            backup_retention_period: db.backup_retention_period(),
            preferred_backup_window: db.preferred_backup_window().map(|s| s.to_string()),
            preferred_maintenance_window: db.preferred_maintenance_window().map(|s| s.to_string()),
            subnet_group: db
                .db_subnet_group()
                .and_then(|s| s.db_subnet_group_name())
                .map(|s| s.to_string()),
            created_time: db.instance_create_time().map(|d| d.to_string()),
            security_groups: db
                .vpc_security_groups()
                .iter()
                .map(DbVpcSecurityGroup::from)
                .collect(),
            tags: db
                .tag_list()
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
pub struct DbCluster {
    pub id: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    pub multi_az: bool,
    pub endpoint: Option<String>,
    pub reader_endpoint: Option<String>,
    pub port: Option<i32>,
    pub db_name: Option<String>,
    pub master_username: Option<String>,
    pub storage_encrypted: bool,
    pub created_time: Option<String>,
    pub members: Vec<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_rds::types::DbCluster> for DbCluster {
    fn from(db: aws_sdk_rds::types::DbCluster) -> Self {
        Self {
            id: db.db_cluster_identifier().unwrap_or_default().to_string(),
            arn: db.db_cluster_arn().map(|s| s.to_string()),
            status: db.status().map(|s| s.to_string()),
            engine: db.engine().map(|s| s.to_string()),
            engine_version: db.engine_version().map(|s| s.to_string()),
            multi_az: db.multi_az().unwrap_or(false),
            endpoint: db.endpoint().map(|s| s.to_string()),
            reader_endpoint: db.reader_endpoint().map(|s| s.to_string()),
            port: db.port(),
            db_name: db.database_name().map(|s| s.to_string()),
            master_username: db.master_username().map(|s| s.to_string()),
            storage_encrypted: db.storage_encrypted().unwrap_or(false),
            created_time: db.cluster_create_time().map(|d| d.to_string()),
            members: db
                .db_cluster_members()
                .iter()
                .filter_map(|m| m.db_instance_identifier())
                .map(|s| s.to_string())
                .collect(),
            tags: db
                .tag_list()
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
pub struct DbSnapshot {
    pub id: String,
    pub arn: Option<String>,
    pub db_instance_id: Option<String>,
    pub snapshot_type: Option<String>,
    pub status: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    pub allocated_storage: Option<i32>,
    pub encrypted: bool,
    pub created_time: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_rds::types::DbSnapshot> for DbSnapshot {
    fn from(snap: aws_sdk_rds::types::DbSnapshot) -> Self {
        Self {
            id: snap.db_snapshot_identifier().unwrap_or_default().to_string(),
            arn: snap.db_snapshot_arn().map(|s| s.to_string()),
            db_instance_id: snap.db_instance_identifier().map(|s| s.to_string()),
            snapshot_type: snap.snapshot_type().map(|s| s.to_string()),
            status: snap.status().map(|s| s.to_string()),
            engine: snap.engine().map(|s| s.to_string()),
            engine_version: snap.engine_version().map(|s| s.to_string()),
            allocated_storage: snap.allocated_storage(),
            encrypted: snap.encrypted().unwrap_or(false),
            created_time: snap.snapshot_create_time().map(|d| d.to_string()),
            tags: snap
                .tag_list()
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
pub struct DbParameterGroup {
    pub name: String,
    pub arn: Option<String>,
    pub family: Option<String>,
    pub description: Option<String>,
}

impl From<aws_sdk_rds::types::DbParameterGroup> for DbParameterGroup {
    fn from(pg: aws_sdk_rds::types::DbParameterGroup) -> Self {
        Self {
            name: pg.db_parameter_group_name().unwrap_or_default().to_string(),
            arn: pg.db_parameter_group_arn().map(|s| s.to_string()),
            family: pg.db_parameter_group_family().map(|s| s.to_string()),
            description: pg.description().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RdsSubnet {
    pub subnet_id: Option<String>,
    pub az: Option<String>,
    pub status: Option<String>,
}

impl From<&aws_sdk_rds::types::Subnet> for RdsSubnet {
    fn from(s: &aws_sdk_rds::types::Subnet) -> Self {
        Self {
            subnet_id: s.subnet_identifier().map(|v| v.to_string()),
            az: s
                .subnet_availability_zone()
                .and_then(|z| z.name())
                .map(|v| v.to_string()),
            status: s.subnet_status().map(|v| v.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DbSubnetGroup {
    pub name: String,
    pub arn: Option<String>,
    pub description: Option<String>,
    pub vpc_id: Option<String>,
    pub status: Option<String>,
    pub subnets: Vec<RdsSubnet>,
}

impl From<aws_sdk_rds::types::DbSubnetGroup> for DbSubnetGroup {
    fn from(sg: aws_sdk_rds::types::DbSubnetGroup) -> Self {
        Self {
            name: sg.db_subnet_group_name().unwrap_or_default().to_string(),
            arn: sg.db_subnet_group_arn().map(|s| s.to_string()),
            description: sg.db_subnet_group_description().map(|s| s.to_string()),
            vpc_id: sg.vpc_id().map(|s| s.to_string()),
            status: sg.subnet_group_status().map(|s| s.to_string()),
            subnets: sg.subnets().iter().map(RdsSubnet::from).collect(),
        }
    }
}

// === Unit Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_vpc_security_group_from_sdk() {
        let sg = aws_sdk_rds::types::VpcSecurityGroupMembership::builder()
            .vpc_security_group_id("sg-12345")
            .status("active")
            .build();
        let result = DbVpcSecurityGroup::from(&sg);
        assert_eq!(result.vpc_security_group_id, Some("sg-12345".to_string()));
        assert_eq!(result.status, Some("active".to_string()));
    }

    #[test]
    fn test_db_instance_from_sdk() {
        let endpoint = aws_sdk_rds::types::Endpoint::builder()
            .address("mydb.abc123.us-east-1.rds.amazonaws.com")
            .port(5432)
            .build();
        let subnet_group = aws_sdk_rds::types::DbSubnetGroup::builder()
            .db_subnet_group_name("my-subnet-group")
            .vpc_id("vpc-12345")
            .build();
        let vpc_sg = aws_sdk_rds::types::VpcSecurityGroupMembership::builder()
            .vpc_security_group_id("sg-abc")
            .status("active")
            .build();
        let tag = aws_sdk_rds::types::Tag::builder()
            .key("env")
            .value("prod")
            .build();
        let db = aws_sdk_rds::types::DbInstance::builder()
            .db_instance_identifier("my-db")
            .db_instance_arn("arn:aws:rds:us-east-1:123:db:my-db")
            .engine("postgres")
            .engine_version("14.3")
            .db_instance_status("available")
            .db_instance_class("db.t3.medium")
            .multi_az(true)
            .publicly_accessible(false)
            .storage_encrypted(true)
            .allocated_storage(100)
            .endpoint(endpoint)
            .db_subnet_group(subnet_group)
            .availability_zone("us-east-1a")
            .db_name("mydb")
            .master_username("admin")
            .backup_retention_period(7)
            .preferred_backup_window("02:00-03:00")
            .preferred_maintenance_window("sun:05:00-sun:06:00")
            .vpc_security_groups(vpc_sg)
            .tag_list(tag)
            .build();
        let result = DbInstance::from(db);
        assert_eq!(result.id, "my-db");
        assert_eq!(result.arn, Some("arn:aws:rds:us-east-1:123:db:my-db".to_string()));
        assert_eq!(result.engine, Some("postgres".to_string()));
        assert_eq!(result.engine_version, Some("14.3".to_string()));
        assert_eq!(result.status, Some("available".to_string()));
        assert_eq!(result.instance_class, Some("db.t3.medium".to_string()));
        assert!(result.multi_az);
        assert!(!result.publicly_accessible);
        assert!(result.storage_encrypted);
        assert_eq!(result.allocated_storage, Some(100));
        assert_eq!(
            result.endpoint_address,
            Some("mydb.abc123.us-east-1.rds.amazonaws.com".to_string())
        );
        assert_eq!(result.endpoint_port, Some(5432));
        assert_eq!(result.vpc_id, Some("vpc-12345".to_string()));
        assert_eq!(result.az, Some("us-east-1a".to_string()));
        assert_eq!(result.db_name, Some("mydb".to_string()));
        assert_eq!(result.master_username, Some("admin".to_string()));
        assert_eq!(result.backup_retention_period, Some(7));
        assert_eq!(result.preferred_backup_window, Some("02:00-03:00".to_string()));
        assert_eq!(
            result.preferred_maintenance_window,
            Some("sun:05:00-sun:06:00".to_string())
        );
        assert_eq!(result.subnet_group, Some("my-subnet-group".to_string()));
        assert_eq!(result.security_groups.len(), 1);
        assert_eq!(
            result.security_groups[0].vpc_security_group_id,
            Some("sg-abc".to_string())
        );
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_db_instance_from_sdk_empty() {
        let db = aws_sdk_rds::types::DbInstance::builder().build();
        let result = DbInstance::from(db);
        assert_eq!(result.id, "");
        assert_eq!(result.arn, None);
        assert_eq!(result.engine, None);
        assert!(!result.multi_az);
        assert!(!result.publicly_accessible);
        assert!(!result.storage_encrypted);
        assert_eq!(result.allocated_storage, None);
        assert_eq!(result.endpoint_address, None);
        assert_eq!(result.endpoint_port, None);
        assert_eq!(result.vpc_id, None);
        assert!(result.security_groups.is_empty());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_db_instance_multi_az_defaults_false() {
        let db = aws_sdk_rds::types::DbInstance::builder().build();
        let result = DbInstance::from(db);
        assert!(!result.multi_az);
    }

    #[test]
    fn test_db_cluster_from_sdk() {
        let member = aws_sdk_rds::types::DbClusterMember::builder()
            .db_instance_identifier("my-db-1")
            .build();
        let tag = aws_sdk_rds::types::Tag::builder()
            .key("env")
            .value("prod")
            .build();
        let db = aws_sdk_rds::types::DbCluster::builder()
            .db_cluster_identifier("my-cluster")
            .db_cluster_arn("arn:aws:rds:us-east-1:123:cluster:my-cluster")
            .status("available")
            .engine("aurora-postgresql")
            .engine_version("14.3")
            .multi_az(true)
            .endpoint("my-cluster.cluster-abc.us-east-1.rds.amazonaws.com")
            .reader_endpoint("my-cluster.cluster-ro-abc.us-east-1.rds.amazonaws.com")
            .port(5432)
            .database_name("mydb")
            .master_username("admin")
            .storage_encrypted(true)
            .db_cluster_members(member)
            .tag_list(tag)
            .build();
        let result = DbCluster::from(db);
        assert_eq!(result.id, "my-cluster");
        assert_eq!(
            result.arn,
            Some("arn:aws:rds:us-east-1:123:cluster:my-cluster".to_string())
        );
        assert_eq!(result.status, Some("available".to_string()));
        assert_eq!(result.engine, Some("aurora-postgresql".to_string()));
        assert_eq!(result.engine_version, Some("14.3".to_string()));
        assert!(result.multi_az);
        assert_eq!(
            result.endpoint,
            Some("my-cluster.cluster-abc.us-east-1.rds.amazonaws.com".to_string())
        );
        assert_eq!(
            result.reader_endpoint,
            Some("my-cluster.cluster-ro-abc.us-east-1.rds.amazonaws.com".to_string())
        );
        assert_eq!(result.port, Some(5432));
        assert_eq!(result.db_name, Some("mydb".to_string()));
        assert_eq!(result.master_username, Some("admin".to_string()));
        assert!(result.storage_encrypted);
        assert_eq!(result.members, vec!["my-db-1".to_string()]);
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "env");
    }

    #[test]
    fn test_db_cluster_from_sdk_empty() {
        let db = aws_sdk_rds::types::DbCluster::builder().build();
        let result = DbCluster::from(db);
        assert_eq!(result.id, "");
        assert_eq!(result.arn, None);
        assert!(!result.multi_az);
        assert!(!result.storage_encrypted);
        assert!(result.members.is_empty());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_db_snapshot_from_sdk() {
        let tag = aws_sdk_rds::types::Tag::builder()
            .key("backup")
            .value("true")
            .build();
        let snap = aws_sdk_rds::types::DbSnapshot::builder()
            .db_snapshot_identifier("my-snapshot")
            .db_snapshot_arn("arn:aws:rds:us-east-1:123:snapshot:my-snapshot")
            .db_instance_identifier("my-db")
            .snapshot_type("manual")
            .status("available")
            .engine("postgres")
            .engine_version("14.3")
            .allocated_storage(100)
            .encrypted(true)
            .tag_list(tag)
            .build();
        let result = DbSnapshot::from(snap);
        assert_eq!(result.id, "my-snapshot");
        assert_eq!(
            result.arn,
            Some("arn:aws:rds:us-east-1:123:snapshot:my-snapshot".to_string())
        );
        assert_eq!(result.db_instance_id, Some("my-db".to_string()));
        assert_eq!(result.snapshot_type, Some("manual".to_string()));
        assert_eq!(result.status, Some("available".to_string()));
        assert_eq!(result.engine, Some("postgres".to_string()));
        assert_eq!(result.engine_version, Some("14.3".to_string()));
        assert_eq!(result.allocated_storage, Some(100));
        assert!(result.encrypted);
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "backup");
    }

    #[test]
    fn test_db_snapshot_from_sdk_empty() {
        let snap = aws_sdk_rds::types::DbSnapshot::builder().build();
        let result = DbSnapshot::from(snap);
        assert_eq!(result.id, "");
        assert_eq!(result.arn, None);
        assert_eq!(result.db_instance_id, None);
        assert!(!result.encrypted);
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_db_snapshot_encrypted_defaults_false() {
        let snap = aws_sdk_rds::types::DbSnapshot::builder().build();
        let result = DbSnapshot::from(snap);
        assert!(!result.encrypted);
    }

    #[test]
    fn test_db_parameter_group_from_sdk() {
        let pg = aws_sdk_rds::types::DbParameterGroup::builder()
            .db_parameter_group_name("my-pg")
            .db_parameter_group_arn("arn:aws:rds:us-east-1:123:pg:my-pg")
            .db_parameter_group_family("postgres14")
            .description("My parameter group")
            .build();
        let result = DbParameterGroup::from(pg);
        assert_eq!(result.name, "my-pg");
        assert_eq!(result.arn, Some("arn:aws:rds:us-east-1:123:pg:my-pg".to_string()));
        assert_eq!(result.family, Some("postgres14".to_string()));
        assert_eq!(result.description, Some("My parameter group".to_string()));
    }

    #[test]
    fn test_db_parameter_group_from_sdk_empty() {
        let pg = aws_sdk_rds::types::DbParameterGroup::builder().build();
        let result = DbParameterGroup::from(pg);
        assert_eq!(result.name, "");
        assert_eq!(result.arn, None);
        assert_eq!(result.family, None);
        assert_eq!(result.description, None);
    }

    #[test]
    fn test_rds_subnet_from_sdk() {
        let az = aws_sdk_rds::types::AvailabilityZone::builder()
            .name("us-east-1a")
            .build();
        let subnet = aws_sdk_rds::types::Subnet::builder()
            .subnet_identifier("subnet-12345")
            .subnet_availability_zone(az)
            .subnet_status("Active")
            .build();
        let result = RdsSubnet::from(&subnet);
        assert_eq!(result.subnet_id, Some("subnet-12345".to_string()));
        assert_eq!(result.az, Some("us-east-1a".to_string()));
        assert_eq!(result.status, Some("Active".to_string()));
    }

    #[test]
    fn test_rds_subnet_from_sdk_empty() {
        let subnet = aws_sdk_rds::types::Subnet::builder().build();
        let result = RdsSubnet::from(&subnet);
        assert_eq!(result.subnet_id, None);
        assert_eq!(result.az, None);
        assert_eq!(result.status, None);
    }

    #[test]
    fn test_db_subnet_group_from_sdk() {
        let az = aws_sdk_rds::types::AvailabilityZone::builder()
            .name("us-east-1a")
            .build();
        let subnet = aws_sdk_rds::types::Subnet::builder()
            .subnet_identifier("subnet-abc")
            .subnet_availability_zone(az)
            .subnet_status("Active")
            .build();
        let sg = aws_sdk_rds::types::DbSubnetGroup::builder()
            .db_subnet_group_name("my-subnet-group")
            .db_subnet_group_arn("arn:aws:rds:us-east-1:123:subgrp:my-subnet-group")
            .db_subnet_group_description("My subnet group")
            .vpc_id("vpc-12345")
            .subnet_group_status("Complete")
            .subnets(subnet)
            .build();
        let result = DbSubnetGroup::from(sg);
        assert_eq!(result.name, "my-subnet-group");
        assert_eq!(
            result.arn,
            Some("arn:aws:rds:us-east-1:123:subgrp:my-subnet-group".to_string())
        );
        assert_eq!(result.description, Some("My subnet group".to_string()));
        assert_eq!(result.vpc_id, Some("vpc-12345".to_string()));
        assert_eq!(result.status, Some("Complete".to_string()));
        assert_eq!(result.subnets.len(), 1);
        assert_eq!(result.subnets[0].subnet_id, Some("subnet-abc".to_string()));
        assert_eq!(result.subnets[0].az, Some("us-east-1a".to_string()));
    }

    #[test]
    fn test_db_subnet_group_from_sdk_empty() {
        let sg = aws_sdk_rds::types::DbSubnetGroup::builder().build();
        let result = DbSubnetGroup::from(sg);
        assert_eq!(result.name, "");
        assert_eq!(result.arn, None);
        assert_eq!(result.vpc_id, None);
        assert!(result.subnets.is_empty());
    }
}
