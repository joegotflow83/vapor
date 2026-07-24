use async_graphql::{Context, Object, Result};

use crate::aws::pinpoint::PinpointClient;
use crate::schema::pagination::Page;
use crate::schema::pinpoint::types::{PinpointApp, PinpointCampaign, PinpointSegment};

#[derive(Default)]
pub struct PinpointQuery;

#[Object]
impl PinpointQuery {
    /// Lists Pinpoint apps, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn pinpoint_apps(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<PinpointApp>> {
        let client = ctx.data::<PinpointClient>()?;
        let (items, next_token) = client.get_apps(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(PinpointApp::from).collect(),
            next_token,
        })
    }

    /// Lists campaigns for `application_id`, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn pinpoint_campaigns(
        &self,
        ctx: &Context<'_>,
        application_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<PinpointCampaign>> {
        let client = ctx.data::<PinpointClient>()?;
        let (items, next_token) = client
            .get_campaigns(&application_id, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(PinpointCampaign::from).collect(),
            next_token,
        })
    }

    /// Lists segments for `application_id`, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn pinpoint_segments(
        &self,
        ctx: &Context<'_>,
        application_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<PinpointSegment>> {
        let client = ctx.data::<PinpointClient>()?;
        let (items, next_token) = client
            .get_segments(&application_id, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(PinpointSegment::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::pinpoint::PinpointClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::PinpointQuery;

    const BASE: &str = "https://pinpoint.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn pinpoint_apps_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps?page-size=1"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"app-1","Name":"App One","Arn":"arn:aws:mobiletargeting:us-east-1:123456789012:apps/app-1","CreationDate":"2024-01-01T00:00:00Z","tags":{"env":"prod"}}],"NextToken":"cursor-a"}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(PinpointQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ pinpointApps(limit: 1) { items { id name arn creationDate tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["pinpointApps"]["items"][0]["id"], "app-1");
        assert_eq!(data["pinpointApps"]["items"][0]["name"], "App One");
        assert_eq!(
            data["pinpointApps"]["items"][0]["arn"],
            "arn:aws:mobiletargeting:us-east-1:123456789012:apps/app-1"
        );
        assert_eq!(
            data["pinpointApps"]["items"][0]["creationDate"],
            "2024-01-01T00:00:00Z"
        );
        assert_eq!(data["pinpointApps"]["items"][0]["tags"][0]["key"], "env");
        assert_eq!(data["pinpointApps"]["items"][0]["tags"][0]["value"], "prod");
        assert_eq!(data["pinpointApps"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pinpoint_campaigns_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps/app-1/campaigns?page-size=1"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"camp-1","ApplicationId":"app-1","Name":"Campaign One","State":{"CampaignStatus":"EXECUTING"},"CreationDate":"2024-01-01T00:00:00Z","LastModifiedDate":"2024-01-02T00:00:00Z"}],"NextToken":"cursor-b"}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(PinpointQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ pinpointCampaigns(applicationId: "app-1", limit: 1) { items { id applicationId name status creationDate lastModifiedDate } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["pinpointCampaigns"]["items"][0]["id"], "camp-1");
        assert_eq!(
            data["pinpointCampaigns"]["items"][0]["applicationId"],
            "app-1"
        );
        assert_eq!(
            data["pinpointCampaigns"]["items"][0]["name"],
            "Campaign One"
        );
        assert_eq!(data["pinpointCampaigns"]["items"][0]["status"], "EXECUTING");
        assert_eq!(
            data["pinpointCampaigns"]["items"][0]["creationDate"],
            "2024-01-01T00:00:00Z"
        );
        assert_eq!(
            data["pinpointCampaigns"]["items"][0]["lastModifiedDate"],
            "2024-01-02T00:00:00Z"
        );
        assert_eq!(data["pinpointCampaigns"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn pinpoint_segments_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps/app-1/segments?page-size=1"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"seg-1","ApplicationId":"app-1","Name":"Segment One","SegmentType":"DIMENSIONAL","CreationDate":"2024-01-01T00:00:00Z","LastModifiedDate":"2024-01-02T00:00:00Z"}],"NextToken":"cursor-c"}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(PinpointQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ pinpointSegments(applicationId: "app-1", limit: 1) { items { id applicationId name segmentType creationDate lastModifiedDate } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["pinpointSegments"]["items"][0]["id"], "seg-1");
        assert_eq!(
            data["pinpointSegments"]["items"][0]["applicationId"],
            "app-1"
        );
        assert_eq!(data["pinpointSegments"]["items"][0]["name"], "Segment One");
        assert_eq!(
            data["pinpointSegments"]["items"][0]["segmentType"],
            "DIMENSIONAL"
        );
        assert_eq!(
            data["pinpointSegments"]["items"][0]["creationDate"],
            "2024-01-01T00:00:00Z"
        );
        assert_eq!(
            data["pinpointSegments"]["items"][0]["lastModifiedDate"],
            "2024-01-02T00:00:00Z"
        );
        assert_eq!(data["pinpointSegments"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }
}
