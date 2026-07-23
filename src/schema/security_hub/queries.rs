use async_graphql::{Context, Object, Result};

use crate::aws::security_hub::SecurityHubClient;
use crate::schema::pagination::Page;
use crate::schema::security_hub::types::SecurityHubFinding;

#[derive(Default)]
pub struct SecurityHubQuery;

#[Object]
impl SecurityHubQuery {
    async fn security_hub_findings(
        &self,
        ctx: &Context<'_>,
        severity_label: Option<String>,
        workflow_status: Option<String>,
        record_state: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SecurityHubFinding>> {
        let client = ctx.data::<SecurityHubClient>()?;
        let (findings, next_token) = client
            .get_findings(severity_label, workflow_status, record_state, limit, next_token)
            .await?;
        Ok(Page {
            items: findings.into_iter().map(SecurityHubFinding::from).collect(),
            next_token,
        })
    }
}

// The single resolver is a bare passthrough to already-tested
// `SecurityHubClient::get_findings` (see `src/aws/security_hub.rs`'s own
// test module for the pagination/limit/error-mapping behavior). Two tests
// here: full nested field mapping (Severity/Workflow/Resources/Compliance
// unwrapping, real resolver-level logic not covered by `types.rs`'s own
// `From`-impl tests which construct `SecurityHubFinding` directly rather
// than through the wire), and the 3-filter arg passthrough. `createdAt`/
// `updatedAt` deliberately omitted from the GraphQL selection — mapped via
// `DateTime`'s `Display` impl (`Format::DateTime` render), a formatting
// detail orthogonal to this resolver's own logic.
#[cfg(test)]
mod tests {
    use crate::aws::security_hub::SecurityHubClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::SecurityHubQuery;

    const BASE: &str = "https://securityhub.us-east-1.amazonaws.com/findings";

    // `AwsSecurityFinding` is one of the largest shapes in the AWS SDK
    // (~100 optional fields); deserializing a fully-populated instance
    // through the full GraphQL resolution path overflows the test harness's
    // default thread stack in debug builds (same class of issue as
    // `src/schema/aws/test_composition.rs`'s `test_schema_composition_builds`
    // — run on a thread with a roomier stack rather than a plain
    // `#[tokio::test]`).
    #[test]
    fn security_hub_findings_maps_nested_fields_and_next_token() {
        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("build tokio runtime");
                rt.block_on(async {
                    let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
                        request(BASE, r#"{"MaxResults":1}"#),
                        json_response(
                            200,
                            r#"{"Findings":[{"Id":"finding-1","Title":"Exposed S3 bucket","Description":"An S3 bucket is publicly accessible.","SchemaVersion":"2018-10-08","ProductArn":"arn:aws:securityhub:us-east-1::product/aws/securityhub","GeneratorId":"gen-1","AwsAccountId":"111111111111","CreatedAt":"2026-01-01T00:00:00Z","UpdatedAt":"2026-01-01T00:00:00Z","Severity":{"Label":"CRITICAL"},"Workflow":{"Status":"NEW"},"RecordState":"ACTIVE","ProductName":"Security Hub","CompanyName":"AWS","Region":"us-east-1","Resources":[{"Type":"AwsS3Bucket","Id":"arn:aws:s3:::my-bucket"}],"Compliance":{"Status":"FAILED"}}],"NextToken":"page2"}"#,
                        ),
                    )]);
                    let schema = build_query_schema(SecurityHubQuery)
                        .data(SecurityHubClient::new(&sdk_config(http_client.clone())))
                        .finish();

                    let res = schema
                        .execute(
                            r#"{ securityHubFindings(limit: 1) { items { id title description severityLabel workflowStatus recordState productName companyName resourceType resourceId region complianceStatus } nextToken } }"#,
                        )
                        .await;

                    assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
                    let json = res.data.into_json().unwrap();
                    let item = &json["securityHubFindings"]["items"][0];
                    assert_eq!(item["id"], "finding-1");
                    assert_eq!(item["title"], "Exposed S3 bucket");
                    assert_eq!(item["description"], "An S3 bucket is publicly accessible.");
                    assert_eq!(item["severityLabel"], "CRITICAL");
                    assert_eq!(item["workflowStatus"], "NEW");
                    assert_eq!(item["recordState"], "ACTIVE");
                    assert_eq!(item["productName"], "Security Hub");
                    assert_eq!(item["companyName"], "AWS");
                    assert_eq!(item["resourceType"], "AwsS3Bucket");
                    assert_eq!(item["resourceId"], "arn:aws:s3:::my-bucket");
                    assert_eq!(item["region"], "us-east-1");
                    assert_eq!(item["complianceStatus"], "FAILED");
                    assert_eq!(json["securityHubFindings"]["nextToken"], "page2");
                    http_client.relaxed_requests_match();
                });
            })
            .expect("spawn security-hub-findings thread");

        handle.join().expect("security-hub-findings thread panicked");
    }

    #[tokio::test]
    async fn security_hub_findings_forwards_severity_workflow_and_record_state_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Filters":{"SeverityLabel":[{"Value":"CRITICAL","Comparison":"EQUALS"}],"WorkflowStatus":[{"Value":"NEW","Comparison":"EQUALS"}],"RecordState":[{"Value":"ACTIVE","Comparison":"EQUALS"}]}}"#,
            ),
            json_response(200, r#"{"Findings":[]}"#),
        )]);
        let schema = build_query_schema(SecurityHubQuery)
            .data(SecurityHubClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ securityHubFindings(severityLabel: "CRITICAL", workflowStatus: "NEW", recordState: "ACTIVE") { items { id } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["securityHubFindings"]["items"].as_array().unwrap().len(), 0);
        assert!(json["securityHubFindings"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
