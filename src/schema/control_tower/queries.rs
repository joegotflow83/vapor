use async_graphql::{Context, Object, Result};

use crate::aws::control_tower::ControlTowerClient;
use crate::schema::control_tower::types::{ControlTowerLandingZone, EnabledControl};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct ControlTowerQuery;

#[Object]
impl ControlTowerQuery {
    /// Lists Control Tower landing zones, optionally capped at `limit` results (default unlimited).
    async fn control_tower_landing_zones(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ControlTowerLandingZone>> {
        let client = ctx.data::<ControlTowerClient>()?;
        let (zones, next_token) = client.list_landing_zones(limit, next_token).await?;
        Ok(Page {
            items: zones.into_iter().map(ControlTowerLandingZone::from).collect(),
            next_token,
        })
    }

    /// Lists enabled controls, optionally capped at `limit` results (default unlimited).
    async fn control_tower_enabled_controls(
        &self,
        ctx: &Context<'_>,
        target_identifier: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EnabledControl>> {
        let client = ctx.data::<ControlTowerClient>()?;
        let (controls, next_token) = client
            .list_enabled_controls(target_identifier, limit, next_token)
            .await?;
        Ok(Page {
            items: controls.into_iter().map(EnabledControl::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    const BASE: &str = "https://controltower.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn control_tower_landing_zones_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/list-landingzones"), r#"{"maxResults":1}"#),
                json_response(
                    200,
                    r#"{"landingZones":[{"arn":"lz-arn-1"}],"nextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/get-landingzone"),
                    r#"{"landingZoneIdentifier":"lz-arn-1"}"#,
                ),
                json_response(
                    200,
                    r#"{"landingZone":{"arn":"lz-arn-1","version":"3.3","status":"ACTIVE","latestAvailableVersion":"3.3"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(ControlTowerQuery)
            .data(ControlTowerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ controlTowerLandingZones(limit: 1) { items { arn version latestAvailableVersion status driftStatus } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["controlTowerLandingZones"]["items"];
        assert_eq!(items[0]["arn"], "lz-arn-1");
        assert_eq!(items[0]["version"], "3.3");
        assert_eq!(items[0]["latestAvailableVersion"], "3.3");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert!(items[0]["driftStatus"].is_null());
        assert_eq!(
            json["controlTowerLandingZones"]["nextToken"],
            "page2-token"
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn control_tower_enabled_controls_passes_through_target_identifier_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/list-enabled-controls"),
                r#"{"targetIdentifier":"ou-1","maxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"enabledControls":[{"arn":"ec-arn-1","controlIdentifier":"ctrl-1","targetIdentifier":"ou-1","statusSummary":{"status":"SUCCEEDED","lastOperationIdentifier":"op-1"}}],"nextToken":"cursor-a"}"#,
            ),
        )]);
        let schema = build_query_schema(ControlTowerQuery)
            .data(ControlTowerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ controlTowerEnabledControls(targetIdentifier: "ou-1", limit: 1) { items { arn controlIdentifier targetIdentifier statusSummary { status lastOperationIdentifier } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["controlTowerEnabledControls"]["items"];
        assert_eq!(items[0]["arn"], "ec-arn-1");
        assert_eq!(items[0]["controlIdentifier"], "ctrl-1");
        assert_eq!(items[0]["targetIdentifier"], "ou-1");
        assert_eq!(items[0]["statusSummary"]["status"], "SUCCEEDED");
        assert_eq!(items[0]["statusSummary"]["lastOperationIdentifier"], "op-1");
        assert_eq!(json["controlTowerEnabledControls"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }
}
