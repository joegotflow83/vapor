use async_graphql::SimpleObject;

use crate::aws::lake_formation::{
    LakeFormationPermissionInfo, LakeFormationResourceInfo, LakeFormationSettingsInfo,
    LfDefaultPermissionInfo, LfResourceIdentifierInfo,
};

#[derive(SimpleObject, Clone)]
pub struct LakeFormationResource {
    pub resource_arn: String,
    pub role_arn: Option<String>,
    pub last_modified: Option<String>,
    pub with_federation: Option<bool>,
}

impl From<LakeFormationResourceInfo> for LakeFormationResource {
    fn from(r: LakeFormationResourceInfo) -> Self {
        Self {
            resource_arn: r.resource_arn,
            role_arn: r.role_arn,
            last_modified: r.last_modified,
            with_federation: r.with_federation,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LfResourceIdentifier {
    pub catalog: Option<bool>,
    pub database: Option<String>,
    pub table: Option<String>,
    pub data_location: Option<String>,
}

impl From<LfResourceIdentifierInfo> for LfResourceIdentifier {
    fn from(r: LfResourceIdentifierInfo) -> Self {
        Self {
            catalog: r.catalog,
            database: r.database,
            table: r.table,
            data_location: r.data_location,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LakeFormationPermission {
    pub principal: Option<String>,
    pub resource: Option<LfResourceIdentifier>,
    pub permissions: Vec<String>,
    pub permissions_with_grant_option: Vec<String>,
}

impl From<LakeFormationPermissionInfo> for LakeFormationPermission {
    fn from(p: LakeFormationPermissionInfo) -> Self {
        Self {
            principal: p.principal,
            resource: p.resource.map(LfResourceIdentifier::from),
            permissions: p.permissions,
            permissions_with_grant_option: p.permissions_with_grant_option,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LfDefaultPermission {
    pub principal: Option<String>,
    pub permissions: Vec<String>,
}

impl From<LfDefaultPermissionInfo> for LfDefaultPermission {
    fn from(p: LfDefaultPermissionInfo) -> Self {
        Self {
            principal: p.principal,
            permissions: p.permissions,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LakeFormationSettings {
    pub data_lake_admins: Vec<String>,
    pub create_database_default_permissions: Vec<LfDefaultPermission>,
    pub create_table_default_permissions: Vec<LfDefaultPermission>,
}

impl From<LakeFormationSettingsInfo> for LakeFormationSettings {
    fn from(s: LakeFormationSettingsInfo) -> Self {
        Self {
            data_lake_admins: s.data_lake_admins,
            create_database_default_permissions: s
                .create_database_default_permissions
                .into_iter()
                .map(LfDefaultPermission::from)
                .collect(),
            create_table_default_permissions: s
                .create_table_default_permissions
                .into_iter()
                .map(LfDefaultPermission::from)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::lake_formation::{
        LakeFormationPermissionInfo, LakeFormationResourceInfo, LakeFormationSettingsInfo,
        LfDefaultPermissionInfo, LfResourceIdentifierInfo,
    };

    #[test]
    fn test_resource_from_full() {
        let info = LakeFormationResourceInfo {
            resource_arn: "arn:aws:s3:::my-data-lake".to_string(),
            role_arn: Some("arn:aws:iam::123456789012:role/LakeFormationRole".to_string()),
            last_modified: Some("2024-01-15T10:30:00Z".to_string()),
            with_federation: Some(false),
        };
        let result = LakeFormationResource::from(info);
        assert_eq!(result.resource_arn, "arn:aws:s3:::my-data-lake");
        assert!(result.role_arn.is_some());
        assert!(result.last_modified.is_some());
        assert_eq!(result.with_federation, Some(false));
    }

    #[test]
    fn test_resource_from_minimal() {
        let info = LakeFormationResourceInfo {
            resource_arn: "arn:aws:s3:::bucket".to_string(),
            role_arn: None,
            last_modified: None,
            with_federation: None,
        };
        let result = LakeFormationResource::from(info);
        assert_eq!(result.resource_arn, "arn:aws:s3:::bucket");
        assert!(result.role_arn.is_none());
        assert!(result.last_modified.is_none());
        assert!(result.with_federation.is_none());
    }

    #[test]
    fn test_lf_resource_identifier_catalog() {
        let info = LfResourceIdentifierInfo {
            catalog: Some(true),
            database: None,
            table: None,
            data_location: None,
        };
        let result = LfResourceIdentifier::from(info);
        assert_eq!(result.catalog, Some(true));
        assert!(result.database.is_none());
    }

    #[test]
    fn test_lf_resource_identifier_database() {
        let info = LfResourceIdentifierInfo {
            catalog: None,
            database: Some("my_database".to_string()),
            table: None,
            data_location: None,
        };
        let result = LfResourceIdentifier::from(info);
        assert!(result.catalog.is_none());
        assert_eq!(result.database, Some("my_database".to_string()));
        assert!(result.table.is_none());
    }

    #[test]
    fn test_lf_resource_identifier_table() {
        let info = LfResourceIdentifierInfo {
            catalog: None,
            database: None,
            table: Some("my_table".to_string()),
            data_location: None,
        };
        let result = LfResourceIdentifier::from(info);
        assert_eq!(result.table, Some("my_table".to_string()));
        assert!(result.data_location.is_none());
    }

    #[test]
    fn test_lf_resource_identifier_data_location() {
        let info = LfResourceIdentifierInfo {
            catalog: None,
            database: None,
            table: None,
            data_location: Some("arn:aws:s3:::my-bucket/path".to_string()),
        };
        let result = LfResourceIdentifier::from(info);
        assert_eq!(
            result.data_location,
            Some("arn:aws:s3:::my-bucket/path".to_string())
        );
    }

    #[test]
    fn test_permission_from_full() {
        let info = LakeFormationPermissionInfo {
            principal: Some("arn:aws:iam::123456789012:role/DataEngineer".to_string()),
            resource: Some(LfResourceIdentifierInfo {
                catalog: None,
                database: Some("analytics".to_string()),
                table: Some("events".to_string()),
                data_location: None,
            }),
            permissions: vec!["SELECT".to_string(), "DESCRIBE".to_string()],
            permissions_with_grant_option: vec![],
        };
        let result = LakeFormationPermission::from(info);
        assert!(result.principal.is_some());
        assert!(result.resource.is_some());
        assert_eq!(result.permissions.len(), 2);
        assert!(result.permissions_with_grant_option.is_empty());
    }

    #[test]
    fn test_permission_from_minimal() {
        let info = LakeFormationPermissionInfo {
            principal: None,
            resource: None,
            permissions: vec![],
            permissions_with_grant_option: vec![],
        };
        let result = LakeFormationPermission::from(info);
        assert!(result.principal.is_none());
        assert!(result.resource.is_none());
        assert!(result.permissions.is_empty());
    }

    #[test]
    fn test_lf_default_permission_from() {
        let info = LfDefaultPermissionInfo {
            principal: Some("IAM_ALLOWED_PRINCIPALS".to_string()),
            permissions: vec!["ALL".to_string()],
        };
        let result = LfDefaultPermission::from(info);
        assert_eq!(result.principal, Some("IAM_ALLOWED_PRINCIPALS".to_string()));
        assert_eq!(result.permissions, vec!["ALL"]);
    }

    #[test]
    fn test_lf_default_permission_no_principal() {
        let info = LfDefaultPermissionInfo {
            principal: None,
            permissions: vec!["SELECT".to_string()],
        };
        let result = LfDefaultPermission::from(info);
        assert!(result.principal.is_none());
        assert_eq!(result.permissions.len(), 1);
    }

    #[test]
    fn test_settings_from_full() {
        let info = LakeFormationSettingsInfo {
            data_lake_admins: vec![
                "arn:aws:iam::123456789012:user/admin".to_string(),
            ],
            create_database_default_permissions: vec![LfDefaultPermissionInfo {
                principal: Some("IAM_ALLOWED_PRINCIPALS".to_string()),
                permissions: vec!["ALL".to_string()],
            }],
            create_table_default_permissions: vec![LfDefaultPermissionInfo {
                principal: Some("IAM_ALLOWED_PRINCIPALS".to_string()),
                permissions: vec!["ALL".to_string()],
            }],
        };
        let result = LakeFormationSettings::from(info);
        assert_eq!(result.data_lake_admins.len(), 1);
        assert_eq!(result.create_database_default_permissions.len(), 1);
        assert_eq!(result.create_table_default_permissions.len(), 1);
    }

    #[test]
    fn test_settings_from_empty() {
        let info = LakeFormationSettingsInfo {
            data_lake_admins: vec![],
            create_database_default_permissions: vec![],
            create_table_default_permissions: vec![],
        };
        let result = LakeFormationSettings::from(info);
        assert!(result.data_lake_admins.is_empty());
        assert!(result.create_database_default_permissions.is_empty());
        assert!(result.create_table_default_permissions.is_empty());
    }
}
