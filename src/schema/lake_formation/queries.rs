use async_graphql::{Context, Object, Result};

use crate::aws::lake_formation::LakeFormationClient;
use crate::schema::lake_formation::types::{
    LakeFormationPermission, LakeFormationResource, LakeFormationSettings,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct LakeFormationQuery;

#[Object]
impl LakeFormationQuery {
    /// `limit` caps the total number of results returned, default unlimited.
    async fn lake_formation_resources(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LakeFormationResource>> {
        let client = ctx.data::<LakeFormationClient>()?;
        let (resources, next_token) = client.list_resources(limit, next_token).await?;
        Ok(Page {
            items: resources.into_iter().map(LakeFormationResource::from).collect(),
            next_token,
        })
    }

    /// `limit` caps the total number of results returned, default unlimited.
    async fn lake_formation_permissions(
        &self,
        ctx: &Context<'_>,
        principal: Option<String>,
        resource_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LakeFormationPermission>> {
        let client = ctx.data::<LakeFormationClient>()?;
        let (permissions, next_token) = client
            .list_permissions(principal, resource_type, limit, next_token)
            .await?;
        Ok(Page {
            items: permissions
                .into_iter()
                .map(LakeFormationPermission::from)
                .collect(),
            next_token,
        })
    }

    async fn lake_formation_settings(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<LakeFormationSettings>> {
        let client = ctx.data::<LakeFormationClient>()?;
        let settings = client.get_data_lake_settings().await?;
        Ok(settings.map(LakeFormationSettings::from))
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::lake_formation::LakeFormationClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::LakeFormationQuery;

    const BASE: &str = "https://lakeformation.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn lake_formation_resources_maps_items_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ListResources"), r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"ResourceInfoList":[{"ResourceArn":"arn:aws:lakeformation:us-east-1:111122223333:resource/s3-bucket","RoleArn":"arn:aws:iam::111122223333:role/lf-role","LastModified":1700000000,"WithFederation":true}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LakeFormationQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ lakeFormationResources(limit: 1) { items { resourceArn roleArn lastModified withFederation } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data["lakeFormationResources"]["items"][0]["resourceArn"],
            "arn:aws:lakeformation:us-east-1:111122223333:resource/s3-bucket"
        );
        assert_eq!(
            data["lakeFormationResources"]["items"][0]["roleArn"],
            "arn:aws:iam::111122223333:role/lf-role"
        );
        assert_eq!(
            data["lakeFormationResources"]["items"][0]["lastModified"],
            "2023-11-14T22:13:20+00:00"
        );
        assert_eq!(data["lakeFormationResources"]["items"][0]["withFederation"], true);
        assert_eq!(data["lakeFormationResources"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lake_formation_permissions_forwards_principal_and_resource_type_and_maps_items() {
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
        let schema = build_query_schema(LakeFormationQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ lakeFormationPermissions(principal: "arn:aws:iam::111122223333:role/analyst", resourceType: "DATABASE") { items { principal resource { database table dataLocation catalog } permissions permissionsWithGrantOption } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let item = &data["lakeFormationPermissions"]["items"][0];
        assert_eq!(item["principal"], "arn:aws:iam::111122223333:role/analyst");
        assert_eq!(item["resource"]["database"], "sales_db");
        assert!(item["resource"]["table"].is_null());
        assert_eq!(item["permissions"], serde_json::json!(["SELECT", "ALTER"]));
        assert_eq!(item["permissionsWithGrantOption"], serde_json::json!(["SELECT"]));
        assert!(data["lakeFormationPermissions"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lake_formation_settings_maps_admins_and_default_permissions() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetDataLakeSettings"), "{}"),
            json_response(
                200,
                r#"{"DataLakeSettings":{"DataLakeAdmins":[{"DataLakePrincipalIdentifier":"arn:aws:iam::111122223333:role/admin"}],"CreateDatabaseDefaultPermissions":[{"Principal":{"DataLakePrincipalIdentifier":"IAM_ALLOWED_PRINCIPALS"},"Permissions":["ALL"]}],"CreateTableDefaultPermissions":[]}}"#,
            ),
        )]);
        let client = LakeFormationClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LakeFormationQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ lakeFormationSettings { dataLakeAdmins createDatabaseDefaultPermissions { principal permissions } createTableDefaultPermissions { principal permissions } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data["lakeFormationSettings"]["dataLakeAdmins"],
            serde_json::json!(["arn:aws:iam::111122223333:role/admin"])
        );
        assert_eq!(
            data["lakeFormationSettings"]["createDatabaseDefaultPermissions"][0]["principal"],
            "IAM_ALLOWED_PRINCIPALS"
        );
        assert_eq!(
            data["lakeFormationSettings"]["createDatabaseDefaultPermissions"][0]["permissions"],
            serde_json::json!(["ALL"])
        );
        assert!(data["lakeFormationSettings"]["createTableDefaultPermissions"]
            .as_array()
            .unwrap()
            .is_empty());
        http_client.relaxed_requests_match();
    }
}
