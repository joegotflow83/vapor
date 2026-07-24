use aws_config::SdkConfig;

use crate::error::VaporError;

#[derive(Debug)]
pub struct PinpointAppInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub arn: Option<String>,
    pub creation_date: Option<String>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct PinpointCampaignInfo {
    pub id: Option<String>,
    pub application_id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub creation_date: Option<String>,
    pub last_modified_date: Option<String>,
}

#[derive(Debug)]
pub struct PinpointSegmentInfo {
    pub id: Option<String>,
    pub application_id: Option<String>,
    pub name: Option<String>,
    pub segment_type: Option<String>,
    pub creation_date: Option<String>,
    pub last_modified_date: Option<String>,
}

pub struct PinpointClient {
    inner: aws_sdk_pinpoint::Client,
}

impl PinpointClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_pinpoint::Client::new(config),
        }
    }

    /// Lists apps, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `GetApps` has no SDK paginator
    /// (confirmed: no `paginator.rs` under `aws-sdk-pinpoint`'s generated
    /// `operation/get_apps/`) and `page_size` is a string-typed field (not
    /// `max_results`/`limit` like kinesis/mq), so the remaining budget is
    /// capped and stringified on each request, matching `kinesis.rs`'s
    /// `list_streams` loop shape otherwise.
    pub async fn get_apps(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PinpointAppInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_apps();
            if let Some(ref t) = token {
                req = req.token(t);
            }
            let page_size = match limit {
                Some(l) => std::cmp::min(100, l - items.len() as i32),
                None => 100,
            };
            req = req.page_size(page_size.to_string());

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            let apps_response = match output.applications_response() {
                Some(r) => r,
                None => break,
            };

            for app in apps_response.item() {
                items.push(PinpointAppInfo {
                    id: app.id().map(|s| s.to_string()),
                    name: app.name().map(|s| s.to_string()),
                    arn: app.arn().map(|s| s.to_string()),
                    creation_date: app.creation_date().map(|s| s.to_string()),
                    tags: app
                        .tags()
                        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default(),
                });
            }

            token = match apps_response.next_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists campaigns for `application_id`, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `GetCampaigns` has no SDK paginator (confirmed against the generated
    /// SDK source), same as `get_apps`.
    pub async fn get_campaigns(
        &self,
        application_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PinpointCampaignInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_campaigns().application_id(application_id);
            if let Some(ref t) = token {
                req = req.token(t);
            }
            let page_size = match limit {
                Some(l) => std::cmp::min(100, l - items.len() as i32),
                None => 100,
            };
            req = req.page_size(page_size.to_string());

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            let campaigns_response = match output.campaigns_response() {
                Some(r) => r,
                None => break,
            };

            for campaign in campaigns_response.item() {
                items.push(PinpointCampaignInfo {
                    id: campaign.id().map(|s| s.to_string()),
                    application_id: campaign.application_id().map(|s| s.to_string()),
                    name: campaign.name().map(|s| s.to_string()),
                    status: campaign
                        .state()
                        .and_then(|s| s.campaign_status())
                        .map(|s| s.as_str().to_string()),
                    creation_date: campaign.creation_date().map(|s| s.to_string()),
                    last_modified_date: campaign.last_modified_date().map(|s| s.to_string()),
                });
            }

            token = match campaigns_response.next_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists segments for `application_id`, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `GetSegments` has no SDK paginator (confirmed against the generated
    /// SDK source), same as `get_apps`.
    pub async fn get_segments(
        &self,
        application_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PinpointSegmentInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_segments().application_id(application_id);
            if let Some(ref t) = token {
                req = req.token(t);
            }
            let page_size = match limit {
                Some(l) => std::cmp::min(100, l - items.len() as i32),
                None => 100,
            };
            req = req.page_size(page_size.to_string());

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            let segments_response = match output.segments_response() {
                Some(r) => r,
                None => break,
            };

            for segment in segments_response.item() {
                items.push(PinpointSegmentInfo {
                    id: segment.id().map(|s| s.to_string()),
                    application_id: segment.application_id().map(|s| s.to_string()),
                    name: segment.name().map(|s| s.to_string()),
                    segment_type: segment.segment_type().map(|t| t.as_str().to_string()),
                    creation_date: segment.creation_date().map(|s| s.to_string()),
                    last_modified_date: segment.last_modified_date().map(|s| s.to_string()),
                });
            }

            token = match segments_response.next_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://pinpoint.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn get_apps_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps?page-size=100"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"app-1","Name":"App One","Arn":"arn:aws:mobiletargeting:us-east-1:123456789012:apps/app-1","CreationDate":"2024-01-01T00:00:00Z","tags":{"env":"prod"}},{"Id":"app-2"}]}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_apps(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let a1 = &items[0];
        assert_eq!(a1.id, Some("app-1".to_string()));
        assert_eq!(a1.name, Some("App One".to_string()));
        assert_eq!(
            a1.arn,
            Some("arn:aws:mobiletargeting:us-east-1:123456789012:apps/app-1".to_string())
        );
        assert_eq!(a1.creation_date, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(a1.tags, vec![("env".to_string(), "prod".to_string())]);

        // `application_response_correct_errors` in the pinned SDK
        // default-fills `arn`/`id`/`name` to `Some(String::new())` rather
        // than leaving them `None` when absent from the response (gotcha
        // 14's mq.rs sub-case, confirmed here too).
        let a2 = &items[1];
        assert_eq!(a2.id, Some("app-2".to_string()));
        assert_eq!(a2.name, Some(String::new()));
        assert_eq!(a2.tags, Vec::<(String, String)>::new());

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_apps_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps?page-size=100&token=cursor-a"), ""),
            json_response(200, r#"{"Item":[{"Id":"app-3"}]}"#),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_apps(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_apps_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps?page-size=2"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"app-1"},{"Id":"app-2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_apps(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_apps_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/apps?page-size=10"), ""),
                json_response(
                    200,
                    r#"{"Item":[{"Id":"app-1"},{"Id":"app-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/apps?page-size=8&token=p2"), ""),
                json_response(200, r#"{"Item":[{"Id":"app-3"}]}"#),
            ),
        ]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_apps(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_apps_propagates_errors() {
        // `BadRequestException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps?page-size=100"), ""),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let err = client.get_apps(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_campaigns_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps/app-1/campaigns?page-size=100"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"camp-1","ApplicationId":"app-1","Name":"Campaign One","State":{"CampaignStatus":"EXECUTING"},"CreationDate":"2024-01-01T00:00:00Z","LastModifiedDate":"2024-01-02T00:00:00Z"},{"Id":"camp-2"}]}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_campaigns("app-1", None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let c1 = &items[0];
        assert_eq!(c1.id, Some("camp-1".to_string()));
        assert_eq!(c1.application_id, Some("app-1".to_string()));
        assert_eq!(c1.name, Some("Campaign One".to_string()));
        assert_eq!(c1.status, Some("EXECUTING".to_string()));
        assert_eq!(c1.creation_date, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(
            c1.last_modified_date,
            Some("2024-01-02T00:00:00Z".to_string())
        );

        let c2 = &items[1];
        assert_eq!(c2.id, Some("camp-2".to_string()));
        assert_eq!(c2.status, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_campaigns_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v1/apps/app-1/campaigns?page-size=100&token=cursor-a"),
                "",
            ),
            json_response(200, r#"{"Item":[{"Id":"camp-3"}]}"#),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_campaigns("app-1", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_campaigns_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps/app-1/campaigns?page-size=2"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"camp-1"},{"Id":"camp-2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_campaigns("app-1", Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_campaigns_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/apps/app-1/campaigns?page-size=10"), ""),
                json_response(
                    200,
                    r#"{"Item":[{"Id":"camp-1"},{"Id":"camp-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v1/apps/app-1/campaigns?page-size=8&token=p2"),
                    "",
                ),
                json_response(200, r#"{"Item":[{"Id":"camp-3"}]}"#),
            ),
        ]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_campaigns("app-1", Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_campaigns_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps/app-1/campaigns?page-size=100"), ""),
            json_error_response("NotFoundException", "app not found"),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let err = client.get_campaigns("app-1", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFoundException".to_string()));
                assert_eq!(message, "app not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_segments_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps/app-1/segments?page-size=100"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"seg-1","ApplicationId":"app-1","Name":"Segment One","SegmentType":"DIMENSIONAL","CreationDate":"2024-01-01T00:00:00Z","LastModifiedDate":"2024-01-02T00:00:00Z"},{"Id":"seg-2"}]}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_segments("app-1", None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let s1 = &items[0];
        assert_eq!(s1.id, Some("seg-1".to_string()));
        assert_eq!(s1.application_id, Some("app-1".to_string()));
        assert_eq!(s1.name, Some("Segment One".to_string()));
        assert_eq!(s1.segment_type, Some("DIMENSIONAL".to_string()));
        assert_eq!(s1.creation_date, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(
            s1.last_modified_date,
            Some("2024-01-02T00:00:00Z".to_string())
        );

        // `segment_response_correct_errors` default-fills a missing
        // `SegmentType` by parsing the literal string "no value was set"
        // into `SegmentType::Unknown(...)`, whose `as_str()` echoes that
        // string back rather than leaving the field `None`.
        let s2 = &items[1];
        assert_eq!(s2.id, Some("seg-2".to_string()));
        assert_eq!(s2.segment_type, Some("no value was set".to_string()));

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_segments_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v1/apps/app-1/segments?page-size=100&token=cursor-a"),
                "",
            ),
            json_response(200, r#"{"Item":[{"Id":"seg-3"}]}"#),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_segments("app-1", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_segments_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps/app-1/segments?page-size=2"), ""),
            json_response(
                200,
                r#"{"Item":[{"Id":"seg-1"},{"Id":"seg-2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_segments("app-1", Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_segments_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/apps/app-1/segments?page-size=10"), ""),
                json_response(
                    200,
                    r#"{"Item":[{"Id":"seg-1"},{"Id":"seg-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v1/apps/app-1/segments?page-size=8&token=p2"),
                    "",
                ),
                json_response(200, r#"{"Item":[{"Id":"seg-3"}]}"#),
            ),
        ]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_segments("app-1", Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_segments_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/apps/app-1/segments?page-size=100"), ""),
            json_error_response("NotFoundException", "app not found"),
        )]);
        let client = PinpointClient::new(&sdk_config(http_client.clone()));

        let err = client.get_segments("app-1", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFoundException".to_string()));
                assert_eq!(message, "app not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
