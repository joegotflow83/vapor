use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct LakeFormationResourceInfo {
    pub resource_arn: String,
    pub role_arn: Option<String>,
    pub last_modified: Option<String>,
    pub with_federation: Option<bool>,
}

pub struct LfResourceIdentifierInfo {
    pub catalog: Option<bool>,
    pub database: Option<String>,
    pub table: Option<String>,
    pub data_location: Option<String>,
}

pub struct LakeFormationPermissionInfo {
    pub principal: Option<String>,
    pub resource: Option<LfResourceIdentifierInfo>,
    pub permissions: Vec<String>,
    pub permissions_with_grant_option: Vec<String>,
}

pub struct LfDefaultPermissionInfo {
    pub principal: Option<String>,
    pub permissions: Vec<String>,
}

pub struct LakeFormationSettingsInfo {
    pub data_lake_admins: Vec<String>,
    pub create_database_default_permissions: Vec<LfDefaultPermissionInfo>,
    pub create_table_default_permissions: Vec<LfDefaultPermissionInfo>,
}

pub struct LakeFormationClient {
    inner: aws_sdk_lakeformation::Client,
}

impl LakeFormationClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_lakeformation::Client::new(config),
        }
    }

    pub async fn list_resources(&self) -> Result<Vec<LakeFormationResourceInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_resources();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for r in output.resource_info_list() {
                items.push(LakeFormationResourceInfo {
                    resource_arn: r.resource_arn().unwrap_or_default().to_string(),
                    role_arn: r.role_arn().map(|s| s.to_string()),
                    last_modified: r.last_modified().map(|t| t.to_string()),
                    with_federation: r.with_federation(),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_permissions(
        &self,
        principal: Option<String>,
        resource_type: Option<String>,
    ) -> Result<Vec<LakeFormationPermissionInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_permissions();
            if let Some(ref p) = principal {
                req = req.principal(
                    aws_sdk_lakeformation::types::DataLakePrincipal::builder()
                        .data_lake_principal_identifier(p)
                        .build(),
                );
            }
            if let Some(ref rt) = resource_type {
                req = req.resource_type(
                    aws_sdk_lakeformation::types::DataLakeResourceType::from(rt.as_str()),
                );
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for p in output.principal_resource_permissions() {
                let resource = p.resource().map(|r| LfResourceIdentifierInfo {
                    catalog: r.catalog().map(|_| true),
                    database: r.database().map(|d| d.name().to_string()),
                    table: r.table().and_then(|t| t.name()).map(|s| s.to_string()),
                    data_location: r
                        .data_location()
                        .map(|dl| dl.resource_arn().to_string()),
                });
                items.push(LakeFormationPermissionInfo {
                    principal: p
                        .principal()
                        .and_then(|pr| pr.data_lake_principal_identifier())
                        .map(|s| s.to_string()),
                    resource,
                    permissions: p
                        .permissions()
                        .iter()
                        .map(|perm| perm.as_str().to_string())
                        .collect(),
                    permissions_with_grant_option: p
                        .permissions_with_grant_option()
                        .iter()
                        .map(|perm| perm.as_str().to_string())
                        .collect(),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_data_lake_settings(
        &self,
    ) -> Result<Option<LakeFormationSettingsInfo>, VaporError> {
        let output = self
            .inner
            .get_data_lake_settings()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.data_lake_settings().map(|s| LakeFormationSettingsInfo {
            data_lake_admins: s
                .data_lake_admins()
                .iter()
                .filter_map(|a| a.data_lake_principal_identifier())
                .map(|s| s.to_string())
                .collect(),
            create_database_default_permissions: s
                .create_database_default_permissions()
                .iter()
                .map(|pp| LfDefaultPermissionInfo {
                    principal: pp
                        .principal()
                        .and_then(|p| p.data_lake_principal_identifier())
                        .map(|s| s.to_string()),
                    permissions: pp
                        .permissions()
                        .iter()
                        .map(|perm| perm.as_str().to_string())
                        .collect(),
                })
                .collect(),
            create_table_default_permissions: s
                .create_table_default_permissions()
                .iter()
                .map(|pp| LfDefaultPermissionInfo {
                    principal: pp
                        .principal()
                        .and_then(|p| p.data_lake_principal_identifier())
                        .map(|s| s.to_string()),
                    permissions: pp
                        .permissions()
                        .iter()
                        .map(|perm| perm.as_str().to_string())
                        .collect(),
                })
                .collect(),
        }))
    }
}
