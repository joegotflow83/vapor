use aws_config::SdkConfig;

use crate::error::VaporError;

#[derive(Debug)]
pub struct LakeFormationResourceInfo {
    pub resource_arn: String,
    pub role_arn: Option<String>,
    pub last_modified: Option<aws_smithy_types::DateTime>,
    pub with_federation: Option<bool>,
}

#[derive(Debug)]
pub struct LfResourceIdentifierInfo {
    pub catalog: Option<bool>,
    pub database: Option<String>,
    pub table: Option<String>,
    pub data_location: Option<String>,
}

#[derive(Debug)]
pub struct LakeFormationPermissionInfo {
    pub principal: Option<String>,
    pub resource: Option<LfResourceIdentifierInfo>,
    pub permissions: Vec<String>,
    pub permissions_with_grant_option: Vec<String>,
}

#[derive(Debug)]
pub struct LfDefaultPermissionInfo {
    pub principal: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug)]
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

    /// Lists Lake Formation-registered resources, optionally capped at `limit` results (default unlimited).
    /// Resumable via `next_token`; pass the returned token back to continue.
    pub async fn list_resources(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LakeFormationResourceInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_resources();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(remaining) = limit.map(|l| l - items.len() as i32) {
                req = req.max_results(remaining);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for r in output.resource_info_list.unwrap_or_default() {
                items.push(LakeFormationResourceInfo {
                    resource_arn: r.resource_arn.unwrap_or_default(),
                    role_arn: r.role_arn,
                    last_modified: r.last_modified,
                    with_federation: r.with_federation,
                });
            }

            token = output.next_token;
            let hit_limit = limit.is_some_and(|l| items.len() as i32 >= l);
            if token.is_none() || hit_limit {
                break;
            }
        }

        Ok((items, token))
    }

    /// Lists Lake Formation permissions, optionally capped at `limit` results (default unlimited).
    /// Resumable via `next_token`; pass the returned token back to continue.
    pub async fn list_permissions(
        &self,
        principal: Option<String>,
        resource_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LakeFormationPermissionInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

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
                req = req.resource_type(aws_sdk_lakeformation::types::DataLakeResourceType::from(
                    rt.as_str(),
                ));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(remaining) = limit.map(|l| l - items.len() as i32) {
                req = req.max_results(remaining);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for p in output.principal_resource_permissions.unwrap_or_default() {
                let resource = p.resource.map(|r| LfResourceIdentifierInfo {
                    catalog: r.catalog.map(|_| true),
                    database: r.database.map(|d| d.name().to_string()),
                    table: r.table.and_then(|t| t.name().map(|s| s.to_string())),
                    data_location: r.data_location.map(|dl| dl.resource_arn().to_string()),
                });
                items.push(LakeFormationPermissionInfo {
                    principal: p
                        .principal
                        .and_then(|pr| pr.data_lake_principal_identifier)
                        .map(|s| s.to_string()),
                    resource,
                    permissions: p
                        .permissions
                        .unwrap_or_default()
                        .into_iter()
                        .map(|perm| perm.as_str().to_string())
                        .collect(),
                    permissions_with_grant_option: p
                        .permissions_with_grant_option
                        .unwrap_or_default()
                        .into_iter()
                        .map(|perm| perm.as_str().to_string())
                        .collect(),
                });
            }

            token = output.next_token;
            let hit_limit = limit.is_some_and(|l| items.len() as i32 >= l);
            if token.is_none() || hit_limit {
                break;
            }
        }

        Ok((items, token))
    }

    pub async fn get_data_lake_settings(
        &self,
    ) -> Result<Option<LakeFormationSettingsInfo>, VaporError> {
        let output = self
            .inner
            .get_data_lake_settings()
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output
            .data_lake_settings()
            .map(|s| LakeFormationSettingsInfo {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use aws_smithy_types::DateTime;

    const BASE: &str = "https://lakeformation.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_resources_lists_a_single_page() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ListResources"), "{}"),
            json_response(
                200,
                r#"{"ResourceInfoList":[{"ResourceArn":"arn:aws:lakeformation:us-east-1:111122223333:resource/s3-bucket","RoleArn":"arn:aws:iam::111122223333:role/lf-role","LastModified":1700000000,"WithFederation":true}]}"#,
            ),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_resources(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].resource_arn,
            "arn:aws:lakeformation:us-east-1:111122223333:resource/s3-bucket"
        );
        assert_eq!(
            items[0].role_arn.as_deref(),
            Some("arn:aws:iam::111122223333:role/lf-role")
        );
        assert_eq!(
            items[0].last_modified,
            Some(DateTime::from_secs(1_700_000_000))
        );
        assert_eq!(items[0].with_federation, Some(true));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resources_forwards_limit_to_aws_with_no_client_truncate() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ListResources"), r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"ResourceInfoList":[{"ResourceArn":"arn-1"},{"ResourceArn":"arn-2"}],"NextToken":"cursor-b"}"#,
            ),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_resources(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("cursor-b".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resources_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/ListResources"),
                r#"{"NextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"ResourceInfoList":[{"ResourceArn":"arn-3"}]}"#),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_resources(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resources_propagates_errors() {
        // `InvalidInputException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ListResources"), "{}"),
            json_error_response("InvalidInputException", "bad filter condition"),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let err = client.list_resources(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "bad filter condition");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permissions_lists_all_with_principal_and_resource_type_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/ListPermissions"),
                r#"{"Principal":{"DataLakePrincipalIdentifier":"arn:aws:iam::111122223333:role/analyst"},"ResourceType":"DATABASE"}"#,
            ),
            json_response(
                200,
                r#"{"PrincipalResourcePermissions":[{"Principal":{"DataLakePrincipalIdentifier":"arn:aws:iam::111122223333:role/analyst"},"Resource":{"Database":{"Name":"sales_db"}},"Permissions":["SELECT","ALTER"],"PermissionsWithGrantOption":["SELECT"]}]}"#,
            ),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_permissions(
                Some("arn:aws:iam::111122223333:role/analyst".to_string()),
                Some("DATABASE".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].principal.as_deref(),
            Some("arn:aws:iam::111122223333:role/analyst")
        );
        let resource = items[0].resource.as_ref().unwrap();
        assert_eq!(resource.database.as_deref(), Some("sales_db"));
        assert_eq!(resource.table, None);
        assert_eq!(resource.data_location, None);
        assert_eq!(resource.catalog, None);
        assert_eq!(
            items[0].permissions,
            vec!["SELECT".to_string(), "ALTER".to_string()]
        );
        assert_eq!(
            items[0].permissions_with_grant_option,
            vec!["SELECT".to_string()]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permissions_maps_table_data_location_and_catalog_resources() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ListPermissions"), "{}"),
            json_response(
                200,
                r#"{"PrincipalResourcePermissions":[
                    {"Resource":{"Table":{"DatabaseName":"db1","Name":"tbl1"}},"Permissions":[]},
                    {"Resource":{"DataLocation":{"ResourceArn":"arn:aws:s3:::bucket"}},"Permissions":[]},
                    {"Resource":{"Catalog":{}},"Permissions":[]}
                ]}"#,
            ),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .list_permissions(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(
            items[0].resource.as_ref().unwrap().table.as_deref(),
            Some("tbl1")
        );
        assert_eq!(
            items[1].resource.as_ref().unwrap().data_location.as_deref(),
            Some("arn:aws:s3:::bucket")
        );
        assert_eq!(items[2].resource.as_ref().unwrap().catalog, Some(true));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permissions_forwards_limit_to_aws_with_no_client_truncate() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ListPermissions"), r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"PrincipalResourcePermissions":[{"Permissions":[]}],"NextToken":"cursor-c"}"#,
            ),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_permissions(None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("cursor-c".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permissions_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ListPermissions"), "{}"),
            json_error_response("InvalidInputException", "bad principal"),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_permissions(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "bad principal");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_data_lake_settings_returns_settings() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetDataLakeSettings"), "{}"),
            json_response(
                200,
                r#"{"DataLakeSettings":{"DataLakeAdmins":[{"DataLakePrincipalIdentifier":"arn:aws:iam::111122223333:role/admin"}],"CreateDatabaseDefaultPermissions":[{"Principal":{"DataLakePrincipalIdentifier":"arn:aws:iam::111122223333:group/IAMAllowedPrincipals"},"Permissions":["ALL"]}],"CreateTableDefaultPermissions":[{"Principal":{"DataLakePrincipalIdentifier":"arn:aws:iam::111122223333:group/IAMAllowedPrincipals"},"Permissions":["ALL"]}]}}"#,
            ),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let settings = client.get_data_lake_settings().await.unwrap().unwrap();

        assert_eq!(
            settings.data_lake_admins,
            vec!["arn:aws:iam::111122223333:role/admin".to_string()]
        );
        assert_eq!(settings.create_database_default_permissions.len(), 1);
        assert_eq!(
            settings.create_database_default_permissions[0]
                .principal
                .as_deref(),
            Some("arn:aws:iam::111122223333:group/IAMAllowedPrincipals")
        );
        assert_eq!(
            settings.create_database_default_permissions[0].permissions,
            vec!["ALL".to_string()]
        );
        assert_eq!(settings.create_table_default_permissions.len(), 1);
        assert_eq!(
            settings.create_table_default_permissions[0].permissions,
            vec!["ALL".to_string()]
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_data_lake_settings_returns_none_when_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetDataLakeSettings"), "{}"),
            json_response(200, "{}"),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let settings = client.get_data_lake_settings().await.unwrap();

        assert!(settings.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_data_lake_settings_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetDataLakeSettings"), "{}"),
            json_error_response("EntityNotFoundException", "no settings"),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));

        let err = client.get_data_lake_settings().await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("EntityNotFoundException".to_string()));
                assert_eq!(message, "no settings");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
