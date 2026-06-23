use async_graphql::SimpleObject;

use crate::schema::ec2::types::Tag;

/// OpenSearch cluster hardware and availability configuration.
#[derive(SimpleObject, Clone)]
pub struct OpenSearchClusterConfig {
    pub instance_type: Option<String>,
    pub instance_count: Option<i32>,
    pub dedicated_master_enabled: Option<bool>,
    pub dedicated_master_type: Option<String>,
    pub dedicated_master_count: Option<i32>,
    pub zone_awareness_enabled: Option<bool>,
    /// From ZoneAwarenessConfig.availability_zone_count.
    pub availability_zone_count: Option<i32>,
    pub warm_enabled: Option<bool>,
    pub warm_type: Option<String>,
    pub warm_count: Option<i32>,
    /// From ColdStorageOptions.enabled.
    pub cold_storage_enabled: Option<bool>,
}

impl From<&aws_sdk_opensearch::types::ClusterConfig> for OpenSearchClusterConfig {
    fn from(cc: &aws_sdk_opensearch::types::ClusterConfig) -> Self {
        Self {
            instance_type: cc.instance_type().map(|t| t.as_str().to_string()),
            instance_count: cc.instance_count(),
            dedicated_master_enabled: cc.dedicated_master_enabled(),
            dedicated_master_type: cc.dedicated_master_type().map(|t| t.as_str().to_string()),
            dedicated_master_count: cc.dedicated_master_count(),
            zone_awareness_enabled: cc.zone_awareness_enabled(),
            availability_zone_count: cc
                .zone_awareness_config()
                .and_then(|zac| zac.availability_zone_count()),
            warm_enabled: cc.warm_enabled(),
            warm_type: cc.warm_type().map(|t| t.as_str().to_string()),
            warm_count: cc.warm_count(),
            cold_storage_enabled: cc.cold_storage_options().map(|cso| cso.enabled()),
        }
    }
}

/// EBS storage configuration for an OpenSearch domain.
#[derive(SimpleObject, Clone)]
pub struct OpenSearchEbsOptions {
    pub ebs_enabled: Option<bool>,
    /// standard | gp2 | gp3 | io1
    pub volume_type: Option<String>,
    pub volume_size: Option<i32>,
    pub iops: Option<i32>,
    pub throughput: Option<i32>,
}

impl From<&aws_sdk_opensearch::types::EbsOptions> for OpenSearchEbsOptions {
    fn from(ebs: &aws_sdk_opensearch::types::EbsOptions) -> Self {
        Self {
            ebs_enabled: ebs.ebs_enabled(),
            volume_type: ebs.volume_type().map(|t| t.as_str().to_string()),
            volume_size: ebs.volume_size(),
            iops: ebs.iops(),
            throughput: ebs.throughput(),
        }
    }
}

/// VPC network configuration derived for an OpenSearch domain.
#[derive(SimpleObject, Clone)]
pub struct OpenSearchVpcDerivedInfo {
    pub vpc_id: Option<String>,
    pub subnet_ids: Vec<String>,
    pub availability_zones: Vec<String>,
    pub security_group_ids: Vec<String>,
}

impl From<&aws_sdk_opensearch::types::VpcDerivedInfo> for OpenSearchVpcDerivedInfo {
    fn from(vpc: &aws_sdk_opensearch::types::VpcDerivedInfo) -> Self {
        Self {
            vpc_id: vpc.vpc_id().map(|s| s.to_string()),
            subnet_ids: vpc.subnet_ids().iter().map(|s| s.to_string()).collect(),
            availability_zones: vpc.availability_zones().iter().map(|s| s.to_string()).collect(),
            security_group_ids: vpc.security_group_ids().iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Encryption-at-rest configuration for an OpenSearch domain.
#[derive(SimpleObject, Clone)]
pub struct OpenSearchEncryptionAtRest {
    pub enabled: Option<bool>,
    pub kms_key_id: Option<String>,
}

impl From<&aws_sdk_opensearch::types::EncryptionAtRestOptions> for OpenSearchEncryptionAtRest {
    fn from(enc: &aws_sdk_opensearch::types::EncryptionAtRestOptions) -> Self {
        Self {
            enabled: enc.enabled(),
            kms_key_id: enc.kms_key_id().map(|s| s.to_string()),
        }
    }
}

/// Node-to-node encryption configuration for an OpenSearch domain.
#[derive(SimpleObject, Clone)]
pub struct OpenSearchNodeToNodeEncryption {
    pub enabled: Option<bool>,
}

impl From<&aws_sdk_opensearch::types::NodeToNodeEncryptionOptions>
    for OpenSearchNodeToNodeEncryption
{
    fn from(n2n: &aws_sdk_opensearch::types::NodeToNodeEncryptionOptions) -> Self {
        Self {
            enabled: n2n.enabled(),
        }
    }
}

/// Service software update status for an OpenSearch domain.
#[derive(SimpleObject, Clone)]
pub struct OpenSearchServiceSoftwareOptions {
    pub current_version: Option<String>,
    pub new_version: Option<String>,
    pub update_available: Option<bool>,
    pub cancellable: Option<bool>,
    pub update_status: Option<String>,
    pub description: Option<String>,
    /// ISO 8601 timestamp of the next scheduled automated update.
    pub automated_update_date: Option<String>,
    pub optional_deployment: Option<bool>,
}

impl From<&aws_sdk_opensearch::types::ServiceSoftwareOptions>
    for OpenSearchServiceSoftwareOptions
{
    fn from(sso: &aws_sdk_opensearch::types::ServiceSoftwareOptions) -> Self {
        Self {
            current_version: sso.current_version().map(|s| s.to_string()),
            new_version: sso.new_version().map(|s| s.to_string()),
            update_available: sso.update_available(),
            cancellable: sso.cancellable(),
            update_status: sso.update_status().map(|s| s.as_str().to_string()),
            description: sso.description().map(|s| s.to_string()),
            automated_update_date: sso.automated_update_date().map(|d| d.to_string()),
            optional_deployment: sso.optional_deployment(),
        }
    }
}

/// An Amazon OpenSearch Service domain with its full configuration.
///
/// `endpoints` surfaces the VPC endpoint HashMap as key/value pairs (typically
/// `{"vpc": "<endpoint>"}` for VPC-based domains).
///
/// `tags` is always empty — use the `opensearchDomainTags(arn)` query to fetch
/// tags separately, avoiding N+1 API calls in list queries.
#[derive(SimpleObject, Clone)]
pub struct OpenSearchDomain {
    pub domain_id: String,
    pub domain_name: String,
    pub arn: Option<String>,
    pub created: Option<bool>,
    pub deleted: Option<bool>,
    pub endpoint: Option<String>,
    /// VPC endpoint map surfaced as key/value Tag pairs.
    pub endpoints: Vec<Tag>,
    pub engine_version: Option<String>,
    pub processing: Option<bool>,
    pub upgrade_processing: Option<bool>,
    pub cluster_config: Option<OpenSearchClusterConfig>,
    pub ebs_options: Option<OpenSearchEbsOptions>,
    /// Raw IAM resource policy JSON string.
    pub access_policies: Option<String>,
    pub vpc_options: Option<OpenSearchVpcDerivedInfo>,
    pub encryption_at_rest: Option<OpenSearchEncryptionAtRest>,
    pub node_to_node_encryption: Option<OpenSearchNodeToNodeEncryption>,
    pub service_software_options: Option<OpenSearchServiceSoftwareOptions>,
    /// Always empty in domain list/describe queries. Use `opensearchDomainTags(arn)`.
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_opensearch::types::DomainStatus> for OpenSearchDomain {
    fn from(ds: aws_sdk_opensearch::types::DomainStatus) -> Self {
        let endpoints: Vec<Tag> = ds
            .endpoints()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| Tag { key: k.clone(), value: v.clone() })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            domain_id: ds.domain_id().to_string(),
            domain_name: ds.domain_name().to_string(),
            arn: Some(ds.arn().to_string()),
            created: ds.created(),
            deleted: ds.deleted(),
            endpoint: ds.endpoint().map(|s| s.to_string()),
            endpoints,
            engine_version: ds.engine_version().map(|s| s.to_string()),
            processing: ds.processing(),
            upgrade_processing: ds.upgrade_processing(),
            cluster_config: ds.cluster_config().map(OpenSearchClusterConfig::from),
            ebs_options: ds.ebs_options().map(OpenSearchEbsOptions::from),
            access_policies: ds.access_policies().map(|s| s.to_string()),
            vpc_options: ds.vpc_options().map(OpenSearchVpcDerivedInfo::from),
            encryption_at_rest: ds
                .encryption_at_rest_options()
                .map(OpenSearchEncryptionAtRest::from),
            node_to_node_encryption: ds
                .node_to_node_encryption_options()
                .map(OpenSearchNodeToNodeEncryption::from),
            service_software_options: ds
                .service_software_options()
                .map(OpenSearchServiceSoftwareOptions::from),
            tags: vec![],
        }
    }
}

/// Convert an OpenSearch SDK Tag to the shared ec2 Tag type.
pub fn convert_opensearch_tag(t: &aws_sdk_opensearch::types::Tag) -> Tag {
    Tag {
        key: t.key().to_string(),
        value: t.value().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opensearch_ebs_options_from() {
        let ebs = aws_sdk_opensearch::types::EbsOptions::builder()
            .ebs_enabled(true)
            .volume_size(100)
            .iops(3000)
            .throughput(125)
            .build();
        let result = OpenSearchEbsOptions::from(&ebs);
        assert_eq!(result.ebs_enabled, Some(true));
        assert_eq!(result.volume_size, Some(100));
        assert_eq!(result.iops, Some(3000));
        assert_eq!(result.throughput, Some(125));
        assert_eq!(result.volume_type, None);
    }

    #[test]
    fn test_opensearch_ebs_options_empty() {
        let ebs = aws_sdk_opensearch::types::EbsOptions::builder().build();
        let result = OpenSearchEbsOptions::from(&ebs);
        assert_eq!(result.ebs_enabled, None);
        assert_eq!(result.volume_type, None);
        assert_eq!(result.volume_size, None);
        assert_eq!(result.iops, None);
        assert_eq!(result.throughput, None);
    }

    #[test]
    fn test_opensearch_vpc_derived_info_from() {
        let vpc = aws_sdk_opensearch::types::VpcDerivedInfo::builder()
            .vpc_id("vpc-12345")
            .subnet_ids("subnet-abc")
            .subnet_ids("subnet-def")
            .availability_zones("us-east-1a")
            .security_group_ids("sg-001")
            .build();
        let result = OpenSearchVpcDerivedInfo::from(&vpc);
        assert_eq!(result.vpc_id, Some("vpc-12345".to_string()));
        assert_eq!(result.subnet_ids.len(), 2);
        assert!(result.subnet_ids.contains(&"subnet-abc".to_string()));
        assert!(result.subnet_ids.contains(&"subnet-def".to_string()));
        assert_eq!(result.availability_zones, vec!["us-east-1a".to_string()]);
        assert_eq!(result.security_group_ids, vec!["sg-001".to_string()]);
    }

    #[test]
    fn test_opensearch_vpc_derived_info_empty() {
        let vpc = aws_sdk_opensearch::types::VpcDerivedInfo::builder().build();
        let result = OpenSearchVpcDerivedInfo::from(&vpc);
        assert_eq!(result.vpc_id, None);
        assert!(result.subnet_ids.is_empty());
        assert!(result.availability_zones.is_empty());
        assert!(result.security_group_ids.is_empty());
    }

    #[test]
    fn test_opensearch_encryption_at_rest_from() {
        let enc = aws_sdk_opensearch::types::EncryptionAtRestOptions::builder()
            .enabled(true)
            .kms_key_id("arn:aws:kms:us-east-1:123:key/abc")
            .build();
        let result = OpenSearchEncryptionAtRest::from(&enc);
        assert_eq!(result.enabled, Some(true));
        assert_eq!(
            result.kms_key_id,
            Some("arn:aws:kms:us-east-1:123:key/abc".to_string())
        );
    }

    #[test]
    fn test_opensearch_encryption_at_rest_empty() {
        let enc = aws_sdk_opensearch::types::EncryptionAtRestOptions::builder().build();
        let result = OpenSearchEncryptionAtRest::from(&enc);
        assert_eq!(result.enabled, None);
        assert_eq!(result.kms_key_id, None);
    }

    #[test]
    fn test_opensearch_node_to_node_encryption_from() {
        let n2n = aws_sdk_opensearch::types::NodeToNodeEncryptionOptions::builder()
            .enabled(true)
            .build();
        let result = OpenSearchNodeToNodeEncryption::from(&n2n);
        assert_eq!(result.enabled, Some(true));
    }

    #[test]
    fn test_opensearch_service_software_options_from() {
        let sso = aws_sdk_opensearch::types::ServiceSoftwareOptions::builder()
            .current_version("OpenSearch_1.0")
            .new_version("OpenSearch_1.1")
            .update_available(true)
            .cancellable(false)
            .description("Software update available")
            .optional_deployment(false)
            .build();
        let result = OpenSearchServiceSoftwareOptions::from(&sso);
        assert_eq!(result.current_version, Some("OpenSearch_1.0".to_string()));
        assert_eq!(result.new_version, Some("OpenSearch_1.1".to_string()));
        assert_eq!(result.update_available, Some(true));
        assert_eq!(result.cancellable, Some(false));
        assert_eq!(
            result.description,
            Some("Software update available".to_string())
        );
        assert_eq!(result.optional_deployment, Some(false));
    }

    #[test]
    fn test_opensearch_cluster_config_from() {
        let cc = aws_sdk_opensearch::types::ClusterConfig::builder()
            .instance_count(3)
            .dedicated_master_enabled(true)
            .dedicated_master_count(3)
            .zone_awareness_enabled(true)
            .warm_enabled(false)
            .build();
        let result = OpenSearchClusterConfig::from(&cc);
        assert_eq!(result.instance_count, Some(3));
        assert_eq!(result.dedicated_master_enabled, Some(true));
        assert_eq!(result.dedicated_master_count, Some(3));
        assert_eq!(result.zone_awareness_enabled, Some(true));
        assert_eq!(result.availability_zone_count, None);
        assert_eq!(result.warm_enabled, Some(false));
        assert_eq!(result.cold_storage_enabled, None);
    }

    #[test]
    fn test_convert_opensearch_tag() {
        let sdk_tag = aws_sdk_opensearch::types::Tag::builder()
            .key("env")
            .value("prod")
            .build()
            .unwrap();
        let result = convert_opensearch_tag(&sdk_tag);
        assert_eq!(result.key, "env");
        assert_eq!(result.value, "prod");
    }

    #[test]
    fn test_opensearch_domain_from() {
        let ds = aws_sdk_opensearch::types::DomainStatus::builder()
            .domain_id("123456789/my-domain")
            .domain_name("my-domain")
            .arn("arn:aws:es:us-east-1:123456789:domain/my-domain")
            .created(true)
            .deleted(false)
            .cluster_config(aws_sdk_opensearch::types::ClusterConfig::builder().build())
            .build()
            .unwrap();
        let domain = OpenSearchDomain::from(ds);
        assert_eq!(domain.domain_id, "123456789/my-domain");
        assert_eq!(domain.domain_name, "my-domain");
        assert_eq!(
            domain.arn,
            Some("arn:aws:es:us-east-1:123456789:domain/my-domain".to_string())
        );
        assert_eq!(domain.created, Some(true));
        assert_eq!(domain.deleted, Some(false));
        assert!(domain.endpoints.is_empty());
        assert!(domain.tags.is_empty());
    }

    #[test]
    fn test_opensearch_domain_endpoints_to_tags() {
        let mut endpoints_map = std::collections::HashMap::new();
        endpoints_map
            .insert("vpc".to_string(), "vpc-endpoint.us-east-1.es.amazonaws.com".to_string());
        let ds = aws_sdk_opensearch::types::DomainStatus::builder()
            .domain_id("123456789/vpc-domain")
            .domain_name("vpc-domain")
            .arn("arn:aws:es:us-east-1:123456789012:domain/vpc-domain")
            .set_endpoints(Some(endpoints_map))
            .cluster_config(aws_sdk_opensearch::types::ClusterConfig::builder().build())
            .build()
            .unwrap();
        let domain = OpenSearchDomain::from(ds);
        assert_eq!(domain.endpoints.len(), 1);
        assert_eq!(domain.endpoints[0].key, "vpc");
        assert_eq!(
            domain.endpoints[0].value,
            "vpc-endpoint.us-east-1.es.amazonaws.com"
        );
    }
}
