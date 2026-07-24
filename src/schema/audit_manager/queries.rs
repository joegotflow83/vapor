use async_graphql::{Context, Object, Result};

use crate::aws::audit_manager::AuditManagerClient;
use crate::schema::audit_manager::types::{
    AuditManagerAssessment, AuditManagerControl, AuditManagerFramework,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AuditManagerQuery;

#[Object]
impl AuditManagerQuery {
    /// Lists assessments, optionally capped at `limit` results and resumed via `next_token`.
    async fn audit_manager_assessments(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AuditManagerAssessment>> {
        let client = ctx.data::<AuditManagerClient>()?;
        let (assessments, next_token) = client.list_assessments(limit, next_token).await?;
        Ok(Page {
            items: assessments
                .into_iter()
                .map(AuditManagerAssessment::from)
                .collect(),
            next_token,
        })
    }

    /// Lists frameworks, optionally capped at `limit` results and resumed via `next_token`.
    async fn audit_manager_frameworks(
        &self,
        ctx: &Context<'_>,
        framework_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AuditManagerFramework>> {
        let client = ctx.data::<AuditManagerClient>()?;
        let (frameworks, next_token) = client
            .list_frameworks(framework_type, limit, next_token)
            .await?;
        Ok(Page {
            items: frameworks
                .into_iter()
                .map(AuditManagerFramework::from)
                .collect(),
            next_token,
        })
    }

    /// Lists controls, optionally capped at `limit` results and resumed via `next_token`.
    async fn audit_manager_controls(
        &self,
        ctx: &Context<'_>,
        control_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AuditManagerControl>> {
        let client = ctx.data::<AuditManagerClient>()?;
        let (controls, next_token) = client
            .list_controls(control_type, limit, next_token)
            .await?;
        Ok(Page {
            items: controls
                .into_iter()
                .map(AuditManagerControl::from)
                .collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `AuditManagerClient` method each (see `src/aws/audit_manager.rs`'s own
// test module for the pagination/limit/merge/error-mapping behavior) —
// only light smoke tests are needed here per the resolver-layer sweep's
// stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::audit_manager::AuditManagerClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::AuditManagerQuery;

    const ASSESSMENTS: &str = "https://auditmanager.us-east-1.amazonaws.com/assessments";
    const FRAMEWORKS: &str = "https://auditmanager.us-east-1.amazonaws.com/assessmentFrameworks";
    const CONTROLS: &str = "https://auditmanager.us-east-1.amazonaws.com/controls";

    #[tokio::test]
    async fn audit_manager_assessments_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{ASSESSMENTS}?maxResults=1"), ""),
            json_response(
                200,
                r#"{"assessmentMetadata":[{"id":"a1","name":"Assessment One","complianceType":"PCI DSS"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(AuditManagerQuery)
            .data(AuditManagerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ auditManagerAssessments(limit: 1) { items { id name complianceType } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["auditManagerAssessments"]["items"];
        assert_eq!(items[0]["id"], "a1");
        assert_eq!(items[0]["name"], "Assessment One");
        assert_eq!(items[0]["complianceType"], "PCI DSS");
        assert_eq!(json["auditManagerAssessments"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn audit_manager_frameworks_maps_items_for_given_type() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FRAMEWORKS}?frameworkType=Standard"), ""),
            json_response(
                200,
                r#"{"frameworkMetadataList":[{"id":"f1","arn":"arn:aws:auditmanager:us-east-1:123456789012:assessmentFramework/f1","name":"PCI DSS","complianceType":"PCI DSS","controlsCount":133}]}"#,
            ),
        )]);
        let schema = build_query_schema(AuditManagerQuery)
            .data(AuditManagerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ auditManagerFrameworks(frameworkType: "Standard") { items { id arn name complianceType controlsCount } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["auditManagerFrameworks"]["items"];
        assert_eq!(items[0]["id"], "f1");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:auditmanager:us-east-1:123456789012:assessmentFramework/f1"
        );
        assert_eq!(items[0]["name"], "PCI DSS");
        assert_eq!(items[0]["complianceType"], "PCI DSS");
        assert_eq!(items[0]["controlsCount"], 133);
        assert!(json["auditManagerFrameworks"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn audit_manager_controls_maps_items_for_given_type() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{CONTROLS}?controlType=Custom"), ""),
            json_response(
                200,
                r#"{"controlMetadataList":[{"id":"c1","arn":"arn:aws:auditmanager:us-east-1:123456789012:control/c1","name":"Firewall Protection"}]}"#,
            ),
        )]);
        let schema = build_query_schema(AuditManagerQuery)
            .data(AuditManagerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ auditManagerControls(controlType: "Custom") { items { id arn name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["auditManagerControls"]["items"];
        assert_eq!(items[0]["id"], "c1");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:auditmanager:us-east-1:123456789012:control/c1"
        );
        assert_eq!(items[0]["name"], "Firewall Protection");
        assert!(json["auditManagerControls"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
