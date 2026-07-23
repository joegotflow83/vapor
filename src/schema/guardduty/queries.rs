use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use aws_sdk_guardduty::types::{Condition, FindingCriteria};

use crate::aws::guardduty::GuardDutyClient;
use crate::schema::guardduty::types::{Detector, Finding};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct GuardDutyQuery;

#[Object]
impl GuardDutyQuery {
    async fn guardduty_detectors(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Detector>> {
        let client = ctx.data::<GuardDutyClient>()?;
        let (ids, token) = client.list_detectors(limit, next_token).await?;

        let futures: Vec<_> = ids
            .iter()
            .map(|id| async move {
                let output = client.get_detector(id).await;
                (id.clone(), output)
            })
            .collect();

        let results = join_all(futures).await;
        let mut detectors = Vec::new();
        for (id, result) in results {
            match result {
                Ok(output) => detectors.push(Detector::from_output(id, output)),
                Err(e) => {
                    return Err(async_graphql::Error::new(format!(
                        "Failed to get detector {id}: {e}"
                    )));
                }
            }
        }
        Ok(Page { items: detectors, next_token: token })
    }

    async fn guardduty_findings(
        &self,
        ctx: &Context<'_>,
        detector_id: String,
        min_severity: Option<f64>,
        finding_type: Option<String>,
        archived: Option<bool>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Finding>> {
        let client = ctx.data::<GuardDutyClient>()?;

        let mut criterion = std::collections::HashMap::new();

        if let Some(sev) = min_severity {
            criterion.insert(
                "severity".to_string(),
                Condition::builder().greater_than_or_equal(sev as i64).build(),
            );
        }

        if let Some(ref ft) = finding_type {
            criterion.insert(
                "type".to_string(),
                Condition::builder().equals(ft.clone()).build(),
            );
        }

        if let Some(arch) = archived {
            criterion.insert(
                "service.archived".to_string(),
                Condition::builder()
                    .equals(if arch { "true" } else { "false" })
                    .build(),
            );
        }

        let criteria = if criterion.is_empty() {
            None
        } else {
            Some(FindingCriteria::builder().set_criterion(Some(criterion)).build())
        };

        let (finding_ids, token) = client
            .list_findings(&detector_id, criteria, limit, next_token)
            .await?;

        if finding_ids.is_empty() {
            return Ok(Page { items: Vec::new(), next_token: token });
        }

        // Batch in chunks of 50
        let mut all_findings = Vec::new();
        for chunk in finding_ids.chunks(50) {
            let findings = client
                .get_findings(&detector_id, chunk.to_vec())
                .await?;
            all_findings.extend(findings.into_iter().map(Finding::from));
        }

        Ok(Page { items: all_findings, next_token: token })
    }
}

// `guardduty_detectors` fans list_detectors out into a per-id get_detector
// call (join_all) and maps the custom error string on failure — both the
// happy path and that error mapping are real resolver logic worth testing
// beyond a bare passthrough. `guardduty_findings` builds a FindingCriteria
// map from three independent optional filters (min_severity/finding_type/
// archived) — each is tested individually, not combined, since the
// resolver's `criterion` is a `HashMap` and combining filters would make
// the mocked request body's key order non-deterministic across runs (same
// reasoning as `src/aws/guardduty.rs`'s own `list_findings_forwards_finding_criteria`
// test, which also only ever sets one key at a time).
#[cfg(test)]
mod tests {
    use crate::aws::guardduty::GuardDutyClient;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::GuardDutyQuery;

    const BASE: &str = "https://guardduty.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn guardduty_detectors_lists_and_describes_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/detector?maxResults=1"), ""),
                json_response(200, r#"{"detectorIds":["d1"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/detector/d1"), ""),
                json_response(
                    200,
                    r#"{"status":"ENABLED","createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-02-01T00:00:00Z","findingPublishingFrequency":"SIX_HOURS"}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(GuardDutyQuery)
            .data(GuardDutyClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ guarddutyDetectors(limit: 1) { items { id status createdAt updatedAt findingPublishingFrequency } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["guarddutyDetectors"]["items"];
        assert_eq!(items[0]["id"], "d1");
        assert_eq!(items[0]["status"], "ENABLED");
        assert_eq!(items[0]["createdAt"], "2024-01-01T00:00:00Z");
        assert_eq!(items[0]["findingPublishingFrequency"], "SIX_HOURS");
        assert_eq!(json["guarddutyDetectors"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn guardduty_detectors_surfaces_get_detector_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/detector"), ""),
                json_response(200, r#"{"detectorIds":["d1"]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/detector/d1"), ""),
                json_error_response("BadRequestException", "detector not found"),
            ),
        ]);
        let schema = build_query_schema(GuardDutyQuery)
            .data(GuardDutyClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ guarddutyDetectors { items { id } nextToken } }"#)
            .await;

        assert!(!res.errors.is_empty(), "expected an error, got none");
        assert!(
            res.errors[0].message.contains("Failed to get detector d1"),
            "unexpected error message: {}",
            res.errors[0].message
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn guardduty_findings_lists_and_gets_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/detector/d1/findings"), r#"{"maxResults":1}"#),
                json_response(200, r#"{"findingIds":["f1"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings/get"),
                    r#"{"findingIds":["f1"]}"#,
                ),
                json_response(
                    200,
                    r#"{"findings":[{"accountId":"111111111111","id":"f1","type":"Trojan:EC2/DNSDataExfiltration","severity":8.5,"title":"first"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(GuardDutyQuery)
            .data(GuardDutyClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ guarddutyFindings(detectorId: "d1", limit: 1) { items { id accountId severity findingType title } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["guarddutyFindings"]["items"];
        assert_eq!(items[0]["id"], "f1");
        assert_eq!(items[0]["accountId"], "111111111111");
        assert_eq!(items[0]["severity"], 8.5);
        assert_eq!(items[0]["findingType"], "Trojan:EC2/DNSDataExfiltration");
        assert_eq!(json["guarddutyFindings"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn guardduty_findings_returns_empty_page_without_calling_get_findings() {
        // Only one queued event: if the resolver called `get_findings` on an
        // empty id list, `StaticReplayClient` would fail with "no more test
        // data available".
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/detector/d1/findings"), "{}"),
            json_response(200, r#"{"findingIds":[]}"#),
        )]);
        let schema = build_query_schema(GuardDutyQuery)
            .data(GuardDutyClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ guarddutyFindings(detectorId: "d1") { items { id } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["guarddutyFindings"]["items"].as_array().unwrap().is_empty());
        assert!(json["guarddutyFindings"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn guardduty_findings_forwards_min_severity_criteria() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings"),
                    r#"{"findingCriteria":{"criterion":{"severity":{"greaterThanOrEqual":8}}}}"#,
                ),
                json_response(200, r#"{"findingIds":["f1"]}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings/get"),
                    r#"{"findingIds":["f1"]}"#,
                ),
                json_response(200, r#"{"findings":[{"id":"f1"}]}"#),
            ),
        ]);
        let schema = build_query_schema(GuardDutyQuery)
            .data(GuardDutyClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ guarddutyFindings(detectorId: "d1", minSeverity: 8) { items { id } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn guardduty_findings_forwards_finding_type_criteria() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings"),
                    r#"{"findingCriteria":{"criterion":{"type":{"equals":["Trojan"]}}}}"#,
                ),
                json_response(200, r#"{"findingIds":["f1"]}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings/get"),
                    r#"{"findingIds":["f1"]}"#,
                ),
                json_response(200, r#"{"findings":[{"id":"f1"}]}"#),
            ),
        ]);
        let schema = build_query_schema(GuardDutyQuery)
            .data(GuardDutyClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ guarddutyFindings(detectorId: "d1", findingType: "Trojan") { items { id } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn guardduty_findings_forwards_archived_criteria() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings"),
                    r#"{"findingCriteria":{"criterion":{"service.archived":{"equals":["true"]}}}}"#,
                ),
                json_response(200, r#"{"findingIds":["f1"]}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/detector/d1/findings/get"),
                    r#"{"findingIds":["f1"]}"#,
                ),
                json_response(200, r#"{"findings":[{"id":"f1"}]}"#),
            ),
        ]);
        let schema = build_query_schema(GuardDutyQuery)
            .data(GuardDutyClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ guarddutyFindings(detectorId: "d1", archived: true) { items { id } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        http_client.relaxed_requests_match();
    }
}
