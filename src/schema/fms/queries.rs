use async_graphql::{Context, Object, Result};

use crate::aws::fms::FmsClient;
use crate::schema::fms::types::{FmsPolicy, FmsPolicyComplianceStatus};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct FmsQuery;

#[Object]
impl FmsQuery {
    /// Lists FMS policies. `limit` optionally caps the number of results (default
    /// unlimited); `nextToken` resumes from a prior page.
    async fn fms_policies(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<FmsPolicy>> {
        let client = ctx.data::<FmsClient>()?;
        let (policies, next_token) = client.list_policies(limit, next_token).await?;
        Ok(Page {
            items: policies.into_iter().map(FmsPolicy::from).collect(),
            next_token,
        })
    }

    /// Lists policy compliance statuses. `limit` optionally caps the number of
    /// results (default unlimited); `nextToken` resumes from a prior page.
    async fn fms_policy_compliance_statuses(
        &self,
        ctx: &Context<'_>,
        policy_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<FmsPolicyComplianceStatus>> {
        let client = ctx.data::<FmsClient>()?;
        let (statuses, next_token) = client
            .list_compliance_status(&policy_id, limit, next_token)
            .await?;
        Ok(Page {
            items: statuses
                .into_iter()
                .map(FmsPolicyComplianceStatus::from)
                .collect(),
            next_token,
        })
    }

    /// Lists FMS member accounts. `limit` optionally caps the number of results
    /// (default unlimited); `nextToken` resumes from a prior page.
    async fn fms_member_accounts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<String>> {
        let client = ctx.data::<FmsClient>()?;
        let (accounts, next_token) = client.list_member_accounts(limit, next_token).await?;
        Ok(Page {
            items: accounts,
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::fms::FmsClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::FmsQuery;

    const ENDPOINT: &str = "https://fms.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn fms_policies_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"PolicyList":[{"PolicyId":"policy-a","PolicyArn":"arn:aws:fms:us-east-1:123456789012:policy/policy-a","PolicyName":"policy-a-name","SecurityServiceType":"WAFV2","RemediationEnabled":true,"ResourceType":"AWS::EC2::Instance","DeleteUnusedFMManagedResources":true}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let schema = build_query_schema(FmsQuery)
            .data(FmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ fmsPolicies(limit: 1) { items { policyId policyArn policyName securityServiceType remediationEnabled resourceType deleteUnusedFmManagedResources } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["fmsPolicies"]["items"];
        assert_eq!(items[0]["policyId"], "policy-a");
        assert_eq!(
            items[0]["policyArn"],
            "arn:aws:fms:us-east-1:123456789012:policy/policy-a"
        );
        assert_eq!(items[0]["policyName"], "policy-a-name");
        assert_eq!(items[0]["securityServiceType"], "WAFV2");
        assert_eq!(items[0]["remediationEnabled"], true);
        assert_eq!(items[0]["resourceType"], "AWS::EC2::Instance");
        assert_eq!(items[0]["deleteUnusedFmManagedResources"], true);
        assert_eq!(json["fmsPolicies"]["nextToken"], "tok-2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn fms_policy_compliance_statuses_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"PolicyId":"policy-1","MaxResults":1}"#),
            json_response(
                200,
                r#"{"PolicyComplianceStatusList":[{"PolicyOwner":"123456789012","PolicyId":"policy-1","PolicyName":"TestPolicy","MemberAccount":"987654321098","EvaluationResults":[{"ComplianceStatus":"NON_COMPLIANT","ViolatorCount":2,"EvaluationLimitExceeded":false}]}],"NextToken":"tok-3"}"#,
            ),
        )]);
        let schema = build_query_schema(FmsQuery)
            .data(FmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ fmsPolicyComplianceStatuses(policyId: "policy-1", limit: 1) { items { policyOwner policyId policyName memberAccount evaluationResults { complianceStatus violatorCount evaluationLimitExceeded } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["fmsPolicyComplianceStatuses"]["items"];
        assert_eq!(items[0]["policyOwner"], "123456789012");
        assert_eq!(items[0]["policyId"], "policy-1");
        assert_eq!(items[0]["policyName"], "TestPolicy");
        assert_eq!(items[0]["memberAccount"], "987654321098");
        assert_eq!(
            items[0]["evaluationResults"][0]["complianceStatus"],
            "NON_COMPLIANT"
        );
        assert_eq!(items[0]["evaluationResults"][0]["violatorCount"], 2);
        assert_eq!(
            items[0]["evaluationResults"][0]["evaluationLimitExceeded"],
            false
        );
        assert_eq!(json["fmsPolicyComplianceStatuses"]["nextToken"], "tok-3");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn fms_member_accounts_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(200, r#"{"MemberAccounts":["111111111111"],"NextToken":"tok-4"}"#),
        )]);
        let schema = build_query_schema(FmsQuery)
            .data(FmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ fmsMemberAccounts(limit: 1) { items nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["fmsMemberAccounts"]["items"][0], "111111111111");
        assert_eq!(json["fmsMemberAccounts"]["nextToken"], "tok-4");
        http_client.relaxed_requests_match();
    }
}
