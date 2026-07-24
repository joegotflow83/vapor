use async_graphql::{Context, Object, Result};

use crate::aws::inspector::InspectorClient;
use crate::schema::inspector::types::{InspectorCoverage, InspectorFinding};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct InspectorQuery;

#[Object]
impl InspectorQuery {
    /// Lists Inspector findings, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn inspector_findings(
        &self,
        ctx: &Context<'_>,
        severity: Option<String>,
        resource_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<InspectorFinding>> {
        let client = ctx.data::<InspectorClient>()?;
        let (findings, next_token) = client
            .list_findings(severity, resource_type, limit, next_token)
            .await?;
        Ok(Page {
            items: findings.into_iter().map(InspectorFinding::from).collect(),
            next_token,
        })
    }

    /// Lists Inspector coverage, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn inspector_coverage(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<InspectorCoverage>> {
        let client = ctx.data::<InspectorClient>()?;
        let (resources, next_token) = client.list_coverage(limit, next_token).await?;
        Ok(Page {
            items: resources.into_iter().map(InspectorCoverage::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::inspector::InspectorClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::InspectorQuery;

    const FINDINGS_URL: &str = "https://inspector2.us-east-1.amazonaws.com/findings/list";
    const COVERAGE_URL: &str = "https://inspector2.us-east-1.amazonaws.com/coverage/list";

    #[tokio::test]
    async fn inspector_findings_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                FINDINGS_URL,
                r#"{"filterCriteria":{"severity":[{"comparison":"EQUALS","value":"HIGH"}],"resourceType":[{"comparison":"EQUALS","value":"AWS_EC2_INSTANCE"}]},"maxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"findings":[{"findingArn":"arn:aws:inspector2:us-east-1:111122223333:finding/abc123","title":"Critical vulnerability","description":"A critical vulnerability was found.","severity":"CRITICAL","status":"ACTIVE","type":"PACKAGE_VULNERABILITY","resources":[{"type":"AWS_EC2_INSTANCE","id":"i-0abc"}],"firstObservedAt":1700000000,"lastObservedAt":1700003600,"fixAvailable":"YES"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(InspectorQuery)
            .data(InspectorClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ inspectorFindings(severity: "HIGH", resourceType: "AWS_EC2_INSTANCE", limit: 1) { items { findingArn title description severity status findingType resourceType resourceId firstObservedAt lastObservedAt fixAvailable } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["inspectorFindings"]["items"];
        assert_eq!(
            items[0]["findingArn"],
            "arn:aws:inspector2:us-east-1:111122223333:finding/abc123"
        );
        assert_eq!(items[0]["title"], "Critical vulnerability");
        assert_eq!(
            items[0]["description"],
            "A critical vulnerability was found."
        );
        assert_eq!(items[0]["severity"], "CRITICAL");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["findingType"], "PACKAGE_VULNERABILITY");
        assert_eq!(items[0]["resourceType"], "AWS_EC2_INSTANCE");
        assert_eq!(items[0]["resourceId"], "i-0abc");
        assert!(items[0]["firstObservedAt"].is_string());
        assert!(items[0]["lastObservedAt"].is_string());
        assert_eq!(items[0]["fixAvailable"], "YES");
        assert_eq!(json["inspectorFindings"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn inspector_coverage_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(COVERAGE_URL, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"coveredResources":[{"resourceType":"AWS_EC2_INSTANCE","resourceId":"i-0abc","accountId":"111122223333","scanType":"NETWORK","scanStatus":{"statusCode":"INACTIVE","reason":"UNSUPPORTED_OS"}}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(InspectorQuery)
            .data(InspectorClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ inspectorCoverage(limit: 1) { items { resourceId resourceType scanStatus scanStatusReason } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["inspectorCoverage"]["items"];
        assert_eq!(items[0]["resourceId"], "i-0abc");
        assert_eq!(items[0]["resourceType"], "AWS_EC2_INSTANCE");
        assert_eq!(items[0]["scanStatus"], "INACTIVE");
        assert_eq!(items[0]["scanStatusReason"], "UNSUPPORTED_OS");
        assert_eq!(json["inspectorCoverage"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
