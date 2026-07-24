use async_graphql::{Context, Object, Result};

use crate::aws::sso_admin::SsoAdminClient;
use crate::schema::pagination::Page;
use crate::schema::sso_admin::types::{SsoAccountAssignment, SsoInstance, SsoPermissionSet};

#[derive(Default)]
pub struct SsoAdminQuery;

#[Object]
impl SsoAdminQuery {
    /// Lists IAM Identity Center instances, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn sso_instances(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SsoInstance>> {
        let client = ctx.data::<SsoAdminClient>()?;
        let (instances, next_token) = client.list_instances(limit, next_token).await?;
        Ok(Page {
            items: instances.into_iter().map(SsoInstance::from).collect(),
            next_token,
        })
    }

    /// Lists permission sets for an instance, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn sso_permission_sets(
        &self,
        ctx: &Context<'_>,
        instance_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SsoPermissionSet>> {
        let client = ctx.data::<SsoAdminClient>()?;
        let (permission_sets, next_token) = client
            .list_permission_sets(&instance_arn, limit, next_token)
            .await?;
        Ok(Page {
            items: permission_sets
                .into_iter()
                .map(SsoPermissionSet::from)
                .collect(),
            next_token,
        })
    }

    /// Lists account assignments, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn sso_account_assignments(
        &self,
        ctx: &Context<'_>,
        instance_arn: String,
        account_id: String,
        permission_set_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SsoAccountAssignment>> {
        let client = ctx.data::<SsoAdminClient>()?;
        let (assignments, next_token) = client
            .list_account_assignments(
                &instance_arn,
                &account_id,
                &permission_set_arn,
                limit,
                next_token,
            )
            .await?;
        Ok(Page {
            items: assignments
                .into_iter()
                .map(SsoAccountAssignment::from)
                .collect(),
            next_token,
        })
    }
}

// `sso_instances`/`sso_account_assignments` are 1:1 passthroughs to a single
// already-tested `SsoAdminClient` method each; `sso_permission_sets`' own
// list+describe fan-out also lives entirely inside `src/aws/sso_admin.rs`
// (mq/ram precedent) so it's exercised end-to-end here rather than given
// bespoke branch coverage. See that file's test module for the underlying
// pagination/limit/error-mapping behavior.
#[cfg(test)]
mod tests {
    use crate::aws::sso_admin::SsoAdminClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::SsoAdminQuery;

    const BASE: &str = "https://sso.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn sso_instances_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Instances":[{"InstanceArn":"arn:aws:sso:::instance/ssoins-1","IdentityStoreId":"d-1","OwnerAccountId":"111111111111","Name":"main","Status":"ACTIVE"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(SsoAdminQuery)
            .data(SsoAdminClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ssoInstances(limit: 1) { items { instanceArn identityStoreId ownerAccountId name status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ssoInstances"]["items"];
        assert_eq!(items[0]["instanceArn"], "arn:aws:sso:::instance/ssoins-1");
        assert_eq!(items[0]["identityStoreId"], "d-1");
        assert_eq!(items[0]["ownerAccountId"], "111111111111");
        assert_eq!(items[0]["name"], "main");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(json["ssoInstances"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn sso_permission_sets_lists_and_describes_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"InstanceArn":"arn:instance","MaxResults":1}"#),
                json_response(200, r#"{"PermissionSets":["ps-1"],"NextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-1"}"#,
                ),
                json_response(
                    200,
                    r#"{"PermissionSet":{"Name":"ps-one","PermissionSetArn":"ps-1","Description":"first","SessionDuration":"PT1H"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(SsoAdminQuery)
            .data(SsoAdminClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ssoPermissionSets(instanceArn: "arn:instance", limit: 1) { items { permissionSetArn name description sessionDuration } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ssoPermissionSets"]["items"];
        assert_eq!(items[0]["permissionSetArn"], "ps-1");
        assert_eq!(items[0]["name"], "ps-one");
        assert_eq!(items[0]["description"], "first");
        assert_eq!(items[0]["sessionDuration"], "PT1H");
        assert_eq!(json["ssoPermissionSets"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn sso_account_assignments_maps_items_and_forwards_args() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"InstanceArn":"arn:instance","AccountId":"111111111111","PermissionSetArn":"ps-1","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"AccountAssignments":[{"AccountId":"111111111111","PermissionSetArn":"ps-1","PrincipalType":"USER","PrincipalId":"principal-1"}]}"#,
            ),
        )]);
        let schema = build_query_schema(SsoAdminQuery)
            .data(SsoAdminClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ssoAccountAssignments(instanceArn: "arn:instance", accountId: "111111111111", permissionSetArn: "ps-1", limit: 1) { items { accountId permissionSetArn principalType principalId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ssoAccountAssignments"]["items"];
        assert_eq!(items[0]["accountId"], "111111111111");
        assert_eq!(items[0]["permissionSetArn"], "ps-1");
        assert_eq!(items[0]["principalType"], "USER");
        assert_eq!(items[0]["principalId"], "principal-1");
        assert!(json["ssoAccountAssignments"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
