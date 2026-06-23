use async_graphql::SimpleObject;
use aws_sdk_redshiftserverless::types::{Namespace, Workgroup};

#[derive(SimpleObject, Clone)]
pub struct RedshiftServerlessNamespace {
    pub namespace_name: Option<String>,
    pub namespace_arn: Option<String>,
    pub namespace_id: Option<String>,
    pub db_name: Option<String>,
    pub admin_username: Option<String>,
    pub iam_roles: Vec<String>,
    pub kms_key_id: Option<String>,
    pub log_exports: Vec<String>,
    pub status: Option<String>,
    pub creation_date: Option<String>,
}

impl From<Namespace> for RedshiftServerlessNamespace {
    fn from(n: Namespace) -> Self {
        Self {
            namespace_name: n.namespace_name().map(|v| v.to_string()),
            namespace_arn: n.namespace_arn().map(|v| v.to_string()),
            namespace_id: n.namespace_id().map(|v| v.to_string()),
            db_name: n.db_name().map(|v| v.to_string()),
            admin_username: n.admin_username().map(|v| v.to_string()),
            iam_roles: n.iam_roles().to_vec(),
            kms_key_id: n.kms_key_id().map(|v| v.to_string()),
            log_exports: n.log_exports().iter().map(|e| e.as_str().to_string()).collect(),
            status: n.status().map(|s| s.as_str().to_string()),
            creation_date: n.creation_date().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RedshiftServerlessWorkgroup {
    pub workgroup_name: Option<String>,
    pub workgroup_arn: Option<String>,
    pub workgroup_id: Option<String>,
    pub namespace_name: Option<String>,
    pub status: Option<String>,
    pub base_capacity: Option<i32>,
    pub max_capacity: Option<i32>,
    pub enhanced_vpc_routing: Option<bool>,
    pub publicly_accessible: Option<bool>,
    pub endpoint_address: Option<String>,
    pub endpoint_port: Option<i32>,
    pub security_group_ids: Vec<String>,
    pub subnet_ids: Vec<String>,
    pub creation_date: Option<String>,
}

impl From<Workgroup> for RedshiftServerlessWorkgroup {
    fn from(w: Workgroup) -> Self {
        Self {
            workgroup_name: w.workgroup_name().map(|v| v.to_string()),
            workgroup_arn: w.workgroup_arn().map(|v| v.to_string()),
            workgroup_id: w.workgroup_id().map(|v| v.to_string()),
            namespace_name: w.namespace_name().map(|v| v.to_string()),
            status: w.status().map(|s| s.as_str().to_string()),
            base_capacity: w.base_capacity(),
            max_capacity: w.max_capacity(),
            enhanced_vpc_routing: w.enhanced_vpc_routing(),
            publicly_accessible: w.publicly_accessible(),
            endpoint_address: w.endpoint().and_then(|e| e.address().map(|v| v.to_string())),
            endpoint_port: w.endpoint().and_then(|e| e.port()),
            security_group_ids: w.security_group_ids().to_vec(),
            subnet_ids: w.subnet_ids().to_vec(),
            creation_date: w.creation_date().map(|t| t.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_from_sdk_minimal() {
        let ns = Namespace::builder()
            .namespace_id("ns-123")
            .namespace_name("my-ns")
            .namespace_arn("arn:aws:redshift-serverless:us-east-1:123:namespace/ns-123")
            .status(aws_sdk_redshiftserverless::types::NamespaceStatus::Available)
            .db_name("dev")
            .admin_username("admin")
            .build();
        let result = RedshiftServerlessNamespace::from(ns);
        assert_eq!(result.namespace_name, Some("my-ns".to_string()));
        assert_eq!(result.namespace_id, Some("ns-123".to_string()));
        assert_eq!(result.namespace_arn, Some("arn:aws:redshift-serverless:us-east-1:123:namespace/ns-123".to_string()));
        assert_eq!(result.status, Some("AVAILABLE".to_string()));
        assert_eq!(result.db_name, Some("dev".to_string()));
        assert_eq!(result.admin_username, Some("admin".to_string()));
        assert!(result.iam_roles.is_empty());
        assert!(result.log_exports.is_empty());
    }

    #[test]
    fn test_namespace_defaults() {
        let ns = Namespace::builder().build();
        let result = RedshiftServerlessNamespace::from(ns);
        assert!(result.namespace_name.is_none());
        assert!(result.namespace_id.is_none());
        assert!(result.status.is_none());
        assert!(result.iam_roles.is_empty());
        assert!(result.log_exports.is_empty());
    }

    #[test]
    fn test_workgroup_from_sdk_minimal() {
        let wg = Workgroup::builder()
            .workgroup_id("wg-456")
            .workgroup_name("my-wg")
            .workgroup_arn("arn:aws:redshift-serverless:us-east-1:123:workgroup/wg-456")
            .status(aws_sdk_redshiftserverless::types::WorkgroupStatus::Available)
            .namespace_name("my-ns")
            .base_capacity(128)
            .publicly_accessible(false)
            .build();
        let result = RedshiftServerlessWorkgroup::from(wg);
        assert_eq!(result.workgroup_name, Some("my-wg".to_string()));
        assert_eq!(result.workgroup_id, Some("wg-456".to_string()));
        assert_eq!(result.workgroup_arn, Some("arn:aws:redshift-serverless:us-east-1:123:workgroup/wg-456".to_string()));
        assert_eq!(result.status, Some("AVAILABLE".to_string()));
        assert_eq!(result.namespace_name, Some("my-ns".to_string()));
        assert_eq!(result.base_capacity, Some(128));
        assert_eq!(result.publicly_accessible, Some(false));
        assert!(result.security_group_ids.is_empty());
        assert!(result.subnet_ids.is_empty());
    }

    #[test]
    fn test_workgroup_defaults() {
        let wg = Workgroup::builder().build();
        let result = RedshiftServerlessWorkgroup::from(wg);
        assert!(result.workgroup_name.is_none());
        assert!(result.workgroup_id.is_none());
        assert!(result.status.is_none());
        assert!(result.base_capacity.is_none());
        assert!(result.max_capacity.is_none());
        assert!(result.enhanced_vpc_routing.is_none());
        assert!(result.publicly_accessible.is_none());
        assert!(result.security_group_ids.is_empty());
        assert!(result.subnet_ids.is_empty());
    }
}
