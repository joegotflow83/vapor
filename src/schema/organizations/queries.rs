use async_graphql::{Context, Object, Result};
use aws_sdk_organizations::types::PolicyType;

use crate::aws::organizations::OrganizationsClient;
use crate::schema::organizations::types::{OrgAccount, OrgPolicy, OrganizationalUnit};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct OrganizationsQuery;

#[Object]
impl OrganizationsQuery {
    /// Lists accounts in the organization, optionally capped at `limit`
    /// results (default unlimited) and resumed from `nextToken`.
    async fn org_accounts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<OrgAccount>> {
        let client = ctx.data::<OrganizationsClient>()?;
        let (accounts, next_token) = client.list_accounts(limit, next_token).await?;
        Ok(Page {
            items: accounts.iter().map(OrgAccount::from).collect(),
            next_token,
        })
    }

    /// Lists organizational units under a parent, optionally capped at
    /// `limit` results (default unlimited) and resumed from `nextToken`.
    async fn org_organizational_units(
        &self,
        ctx: &Context<'_>,
        parent_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<OrganizationalUnit>> {
        let client = ctx.data::<OrganizationsClient>()?;
        let (ous, next_token) = client
            .list_organizational_units_for_parent(&parent_id, limit, next_token)
            .await?;
        Ok(Page {
            items: ous.iter().map(OrganizationalUnit::from).collect(),
            next_token,
        })
    }

    /// Lists policies of the given type, optionally capped at `limit`
    /// results (default unlimited) and resumed from `nextToken`.
    async fn org_policies(
        &self,
        ctx: &Context<'_>,
        policy_type: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<OrgPolicy>> {
        let client = ctx.data::<OrganizationsClient>()?;
        let pt = PolicyType::from(policy_type.as_str());
        let (policies, next_token) = client.list_policies(pt, limit, next_token).await?;
        Ok(Page {
            items: policies.iter().map(OrgPolicy::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::organizations::OrganizationsClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::OrganizationsQuery;

    const ENDPOINT: &str = "https://organizations.us-east-1.amazonaws.com/";

    // `org_accounts` is a bare passthrough to the already-tested
    // `OrganizationsClient::list_accounts` (pagination/limit/error-mapping
    // covered in `src/aws/organizations.rs`) — one light smoke test. Also
    // proves `joinedTimestamp` (an EpochSeconds-format SDK timestamp)
    // converts to RFC3339 via `to_utc`.
    #[tokio::test]
    async fn org_accounts_lists_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Accounts":[{"Id":"123456789012","Arn":"arn:aws:organizations::123456789012:account/o-abc/123456789012","Name":"Production","Email":"prod@example.com","Status":"ACTIVE","JoinedMethod":"CREATED","JoinedTimestamp":1000000}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(OrganizationsQuery)
            .data(OrganizationsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ orgAccounts(limit: 1) { items { id arn name email status joinedMethod joinedTimestamp } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["orgAccounts"]["items"];
        assert_eq!(items[0]["id"], "123456789012");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:organizations::123456789012:account/o-abc/123456789012"
        );
        assert_eq!(items[0]["name"], "Production");
        assert_eq!(items[0]["email"], "prod@example.com");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["joinedMethod"], "CREATED");
        assert_eq!(items[0]["joinedTimestamp"], "1970-01-12T13:46:40+00:00");
        assert_eq!(json["orgAccounts"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    // `org_organizational_units` is a bare passthrough to the already-tested
    // `OrganizationsClient::list_organizational_units_for_parent`
    // (pagination/limit/error-mapping covered in `src/aws/organizations.rs`)
    // — one light smoke test that also proves the `parentId` GraphQL arg
    // forwards into the request's `ParentId` field.
    #[tokio::test]
    async fn org_organizational_units_lists_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ParentId":"r-root1","MaxResults":1}"#),
            json_response(
                200,
                r#"{"OrganizationalUnits":[{"Id":"ou-abc-12345","Arn":"arn:aws:organizations::123456789012:ou/o-abc/ou-abc-12345","Name":"Engineering"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(OrganizationsQuery)
            .data(OrganizationsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ orgOrganizationalUnits(parentId: "r-root1", limit: 1) { items { id arn name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["orgOrganizationalUnits"]["items"];
        assert_eq!(items[0]["id"], "ou-abc-12345");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:organizations::123456789012:ou/o-abc/ou-abc-12345"
        );
        assert_eq!(items[0]["name"], "Engineering");
        assert_eq!(json["orgOrganizationalUnits"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    // `org_policies` is a bare passthrough to the already-tested
    // `OrganizationsClient::list_policies` (pagination/limit/error-mapping
    // covered in `src/aws/organizations.rs`) — one light smoke test that
    // also proves the `policyType` GraphQL arg forwards into the request's
    // `Filter` field.
    #[tokio::test]
    async fn org_policies_lists_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"Filter":"SERVICE_CONTROL_POLICY","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Policies":[{"Id":"p-12345","Arn":"arn:aws:organizations::123456789012:policy/o-abc/service_control_policy/p-12345","Name":"DenyAll","Description":"Deny all actions","Type":"SERVICE_CONTROL_POLICY","AwsManaged":true}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(OrganizationsQuery)
            .data(OrganizationsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ orgPolicies(policyType: "SERVICE_CONTROL_POLICY", limit: 1) { items { id arn name description policyType awsManaged } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["orgPolicies"]["items"];
        assert_eq!(items[0]["id"], "p-12345");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:organizations::123456789012:policy/o-abc/service_control_policy/p-12345"
        );
        assert_eq!(items[0]["name"], "DenyAll");
        assert_eq!(items[0]["description"], "Deny all actions");
        assert_eq!(items[0]["policyType"], "SERVICE_CONTROL_POLICY");
        assert_eq!(items[0]["awsManaged"], true);
        assert_eq!(json["orgPolicies"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
