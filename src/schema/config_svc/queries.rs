use async_graphql::{Context, Object, Result};

use crate::aws::config_svc::AwsConfigClient;
use crate::schema::config_svc::types::{ComplianceByResource, ComplianceSummary, ConfigRule};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AwsConfigQuery;

#[Object]
impl AwsConfigQuery {
    async fn config_rules(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ConfigRule>> {
        let client = ctx.data::<AwsConfigClient>()?;
        let (rules, next_token) = client
            .describe_config_rules(names, limit, next_token)
            .await?;
        Ok(Page {
            items: rules.into_iter().map(ConfigRule::from).collect(),
            next_token,
        })
    }

    async fn compliance_by_rule(
        &self,
        ctx: &Context<'_>,
        rule_names: Option<Vec<String>>,
        compliance_types: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ComplianceSummary>> {
        let client = ctx.data::<AwsConfigClient>()?;
        let (results, next_token) = client
            .describe_compliance_by_config_rule(rule_names, compliance_types, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(ComplianceSummary::from).collect(),
            next_token,
        })
    }

    async fn compliance_by_resource(
        &self,
        ctx: &Context<'_>,
        resource_type: Option<String>,
        compliance_types: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ComplianceByResource>> {
        let client = ctx.data::<AwsConfigClient>()?;
        let (results, next_token) = client
            .describe_compliance_by_resource(resource_type, compliance_types, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(ComplianceByResource::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `AwsConfigClient` method each (see `src/aws/config_svc.rs`'s own test
// module for the pagination/error-mapping behavior) — only light smoke
// tests are needed here per the resolver-layer sweep's stated scope
// (codeartifact/comprehend precedent: one test per resolver).
#[cfg(test)]
mod tests {
    use crate::aws::config_svc::AwsConfigClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::AwsConfigQuery;

    const ENDPOINT: &str = "https://config.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn config_rules_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ConfigRuleNames":["ruleA"]}"#),
            json_response(
                200,
                r#"{"ConfigRules":[{"ConfigRuleName":"ruleA","ConfigRuleArn":"arn:aws:config:us-east-1:123456789012:config-rule/config-rule-abc","ConfigRuleId":"config-rule-abc","Description":"Checks S3 bucket versioning","ConfigRuleState":"ACTIVE","Source":{"Owner":"AWS","SourceIdentifier":"S3_BUCKET_VERSIONING_ENABLED"}}],"NextToken":"cursor-a"}"#,
            ),
        )]);
        let schema = build_query_schema(AwsConfigQuery)
            .data(AwsConfigClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ configRules(names: ["ruleA"], limit: 1) { items { name arn ruleId description state sourceIdentifier sourceOwner } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["configRules"]["items"];
        assert_eq!(items[0]["name"], "ruleA");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:config:us-east-1:123456789012:config-rule/config-rule-abc"
        );
        assert_eq!(items[0]["ruleId"], "config-rule-abc");
        assert_eq!(items[0]["description"], "Checks S3 bucket versioning");
        assert_eq!(items[0]["state"], "ACTIVE");
        assert_eq!(items[0]["sourceIdentifier"], "S3_BUCKET_VERSIONING_ENABLED");
        assert_eq!(items[0]["sourceOwner"], "AWS");
        assert_eq!(json["configRules"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn compliance_by_rule_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"ConfigRuleNames":["ruleA"],"ComplianceTypes":["NON_COMPLIANT"]}"#,
            ),
            json_response(
                200,
                r#"{"ComplianceByConfigRules":[{"ConfigRuleName":"ruleA","Compliance":{"ComplianceType":"NON_COMPLIANT","ComplianceContributorCount":{"CappedCount":3}}}],"NextToken":"cursor-b"}"#,
            ),
        )]);
        let schema = build_query_schema(AwsConfigQuery)
            .data(AwsConfigClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ complianceByRule(ruleNames: ["ruleA"], complianceTypes: ["NON_COMPLIANT"], limit: 1) { items { ruleName complianceType compliantCount nonCompliantCount } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["complianceByRule"]["items"];
        assert_eq!(items[0]["ruleName"], "ruleA");
        assert_eq!(items[0]["complianceType"], "NON_COMPLIANT");
        assert!(items[0]["compliantCount"].is_null());
        assert_eq!(items[0]["nonCompliantCount"], 3);
        assert_eq!(json["complianceByRule"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn compliance_by_resource_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"ResourceType":"AWS::EC2::Instance","ComplianceTypes":["COMPLIANT"],"Limit":1}"#,
            ),
            json_response(
                200,
                r#"{"ComplianceByResources":[{"ResourceType":"AWS::EC2::Instance","ResourceId":"i-1","Compliance":{"ComplianceType":"COMPLIANT"}}],"NextToken":"cursor-c"}"#,
            ),
        )]);
        let schema = build_query_schema(AwsConfigQuery)
            .data(AwsConfigClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ complianceByResource(resourceType: "AWS::EC2::Instance", complianceTypes: ["COMPLIANT"], limit: 1) { items { resourceType resourceId complianceType } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["complianceByResource"]["items"];
        assert_eq!(items[0]["resourceType"], "AWS::EC2::Instance");
        assert_eq!(items[0]["resourceId"], "i-1");
        assert_eq!(items[0]["complianceType"], "COMPLIANT");
        assert_eq!(json["complianceByResource"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }
}
