use aws_config::SdkConfig;
use aws_sdk_quicksight::primitives::DateTime;

use crate::error::VaporError;

#[derive(Debug)]
pub struct QuickSightUserInfo {
    pub user_name: Option<String>,
    pub arn: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub identity_type: Option<String>,
    pub active: bool,
    pub principal_id: Option<String>,
}

#[derive(Debug)]
pub struct QuickSightDashboardInfo {
    pub dashboard_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub created_time: Option<DateTime>,
    pub last_updated_time: Option<DateTime>,
    pub published_version_number: Option<i64>,
    pub last_published_time: Option<DateTime>,
}

#[derive(Debug)]
pub struct QuickSightDataSetInfo {
    pub data_set_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub created_time: Option<DateTime>,
    pub last_updated_time: Option<DateTime>,
    pub import_mode: Option<String>,
}

#[derive(Debug)]
pub struct QuickSightDataSourceInfo {
    pub data_source_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub type_: Option<String>,
    pub status: Option<String>,
    pub created_time: Option<DateTime>,
    pub last_updated_time: Option<DateTime>,
}

pub struct QuickSightClient {
    inner: aws_sdk_quicksight::Client,
}

impl QuickSightClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_quicksight::Client::new(config),
        }
    }

    /// Lists QuickSight users in a namespace, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListUsers` has both `max_results` and `next_token` (verified against
    /// pinned `aws-sdk-quicksight` 1.140.0's
    /// `operation/list_users/_list_users_input.rs`), so `limit` is capped on
    /// the request itself; `.into_paginator()` dropped since it hides the
    /// token (kinesis pattern).
    pub async fn list_users(
        &self,
        aws_account_id: String,
        namespace: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<QuickSightUserInfo>, Option<String>), VaporError> {
        let ns = namespace.unwrap_or_else(|| "default".to_string());
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_users()
                .aws_account_id(&aws_account_id)
                .namespace(&ns);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for u in output.user_list.unwrap_or_default() {
                items.push(QuickSightUserInfo {
                    user_name: u.user_name,
                    arn: u.arn,
                    email: u.email,
                    role: u.role.map(|r| r.as_str().to_string()),
                    identity_type: u.identity_type.map(|t| t.as_str().to_string()),
                    active: u.active,
                    principal_id: u.principal_id,
                });
            }

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists QuickSight dashboards, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListDashboards`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-quicksight` 1.140.0's
    /// `operation/list_dashboards/_list_dashboards_input.rs`), same pattern
    /// as `list_users` above.
    pub async fn list_dashboards(
        &self,
        aws_account_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<QuickSightDashboardInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_dashboards().aws_account_id(&aws_account_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for d in output.dashboard_summary_list.unwrap_or_default() {
                items.push(QuickSightDashboardInfo {
                    dashboard_id: d.dashboard_id,
                    arn: d.arn,
                    name: d.name,
                    created_time: d.created_time,
                    last_updated_time: d.last_updated_time,
                    published_version_number: d.published_version_number,
                    last_published_time: d.last_published_time,
                });
            }

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists QuickSight data sets, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListDataSets`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-quicksight` 1.140.0's
    /// `operation/list_data_sets/_list_data_sets_input.rs`), same pattern
    /// as `list_users` above.
    pub async fn list_data_sets(
        &self,
        aws_account_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<QuickSightDataSetInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_data_sets().aws_account_id(&aws_account_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for ds in output.data_set_summaries.unwrap_or_default() {
                items.push(QuickSightDataSetInfo {
                    data_set_id: ds.data_set_id,
                    arn: ds.arn,
                    name: ds.name,
                    created_time: ds.created_time,
                    last_updated_time: ds.last_updated_time,
                    import_mode: ds.import_mode.map(|m| m.as_str().to_string()),
                });
            }

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists QuickSight data sources, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListDataSources`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-quicksight` 1.140.0's
    /// `operation/list_data_sources/_list_data_sources_input.rs`), same
    /// pattern as `list_users` above.
    pub async fn list_data_sources(
        &self,
        aws_account_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<QuickSightDataSourceInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_data_sources().aws_account_id(&aws_account_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for src in output.data_sources.unwrap_or_default() {
                items.push(QuickSightDataSourceInfo {
                    data_source_id: src.data_source_id,
                    arn: src.arn,
                    name: src.name,
                    type_: src.r#type.map(|t| t.as_str().to_string()),
                    status: src.status.map(|s| s.as_str().to_string()),
                    created_time: src.created_time,
                    last_updated_time: src.last_updated_time,
                });
            }

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

    const BASE: &str = "https://quicksight.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_users_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/accounts/123456789012/namespaces/default/users"),
                "",
            ),
            json_response(
                200,
                r#"{"UserList":[{"UserName":"alice","Arn":"arn:aws:quicksight:us-east-1:123456789012:user/default/alice","Email":"alice@example.com","Role":"ADMIN","IdentityType":"IAM","Active":true,"PrincipalId":"p-1"},{"UserName":"bob"}]}"#,
            ),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_users("123456789012".to_string(), None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        let a1 = &items[0];
        assert_eq!(a1.user_name, Some("alice".to_string()));
        assert_eq!(
            a1.arn,
            Some("arn:aws:quicksight:us-east-1:123456789012:user/default/alice".to_string())
        );
        assert_eq!(a1.email, Some("alice@example.com".to_string()));
        assert_eq!(a1.role, Some("ADMIN".to_string()));
        assert_eq!(a1.identity_type, Some("IAM".to_string()));
        assert!(a1.active);
        assert_eq!(a1.principal_id, Some("p-1".to_string()));

        // `active` has no `*_correct_errors` default-fill (unlike gotcha
        // 14/20's shapes) — a missing `Active` key just becomes
        // `unwrap_or_default()` -> `false` at the builder level.
        let a2 = &items[1];
        assert_eq!(a2.user_name, Some("bob".to_string()));
        assert!(!a2.active);
        assert_eq!(a2.role, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/accounts/123456789012/namespaces/default/users?next-token=cursor-a"),
                "",
            ),
            json_response(200, r#"{"UserList":[{"UserName":"carol"}]}"#),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_users(
                "123456789012".to_string(),
                None,
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_caps_at_limit_across_pages() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!(
                        "{BASE}/accounts/123456789012/namespaces/custom/users?max-results=3"
                    ),
                    "",
                ),
                json_response(
                    200,
                    r#"{"UserList":[{"UserName":"u1"},{"UserName":"u2"}],"NextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!(
                        "{BASE}/accounts/123456789012/namespaces/custom/users?next-token=cursor-b&max-results=1"
                    ),
                    "",
                ),
                json_response(200, r#"{"UserList":[{"UserName":"u3"}]}"#),
            ),
        ]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_users(
                "123456789012".to_string(),
                Some("custom".to_string()),
                Some(3),
                None,
            )
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/accounts/123456789012/namespaces/default/users"),
                "",
            ),
            json_error_response("InvalidParameterValueException", "bad namespace"),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_users("123456789012".to_string(), None, None, None)
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("bad namespace"));
    }

    #[tokio::test]
    async fn list_dashboards_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/dashboards"), ""),
            json_response(
                200,
                r#"{"DashboardSummaryList":[{"DashboardId":"d-1","Arn":"arn:aws:quicksight:us-east-1:123456789012:dashboard/d-1","Name":"Sales","CreatedTime":1704067200,"LastUpdatedTime":1704153600,"PublishedVersionNumber":3,"LastPublishedTime":1704067200},{"DashboardId":"d-2"}]}"#,
            ),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_dashboards("123456789012".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        let d1 = &items[0];
        assert_eq!(d1.dashboard_id, Some("d-1".to_string()));
        assert_eq!(d1.name, Some("Sales".to_string()));
        assert_eq!(
            d1.created_time,
            Some(DateTime::from_secs(1_704_067_200))
        );
        assert_eq!(
            d1.last_updated_time,
            Some(DateTime::from_secs(1_704_153_600))
        );
        assert_eq!(d1.published_version_number, Some(3));

        let d2 = &items[1];
        assert_eq!(d2.dashboard_id, Some("d-2".to_string()));
        assert_eq!(d2.created_time, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_dashboards_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/accounts/123456789012/dashboards?next-token=cursor-a"),
                "",
            ),
            json_response(200, r#"{"DashboardSummaryList":[{"DashboardId":"d-3"}]}"#),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_dashboards(
                "123456789012".to_string(),
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_dashboards_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/dashboards"), ""),
            json_error_response("InvalidNextTokenException", "bad token"),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_dashboards("123456789012".to_string(), None, None)
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("bad token"));
    }

    #[tokio::test]
    async fn list_data_sets_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/data-sets"), ""),
            json_response(
                200,
                r#"{"DataSetSummaries":[{"DataSetId":"ds-1","Arn":"arn:aws:quicksight:us-east-1:123456789012:dataset/ds-1","Name":"Revenue","CreatedTime":1704067200,"LastUpdatedTime":1704153600,"ImportMode":"SPICE"},{"DataSetId":"ds-2"}]}"#,
            ),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_data_sets("123456789012".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        let d1 = &items[0];
        assert_eq!(d1.data_set_id, Some("ds-1".to_string()));
        assert_eq!(d1.name, Some("Revenue".to_string()));
        assert_eq!(d1.import_mode, Some("SPICE".to_string()));
        assert_eq!(d1.created_time, Some(DateTime::from_secs(1_704_067_200)));

        let d2 = &items[1];
        assert_eq!(d2.data_set_id, Some("ds-2".to_string()));
        assert_eq!(d2.import_mode, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_data_sets_caps_at_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/accounts/123456789012/data-sets?max-results=2"),
                "",
            ),
            json_response(
                200,
                r#"{"DataSetSummaries":[{"DataSetId":"ds-1"},{"DataSetId":"ds-2"}]}"#,
            ),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_data_sets("123456789012".to_string(), Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_data_sets_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/data-sets"), ""),
            json_error_response("InvalidParameterValueException", "bad account id"),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_data_sets("123456789012".to_string(), None, None)
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("bad account id"));
    }

    #[tokio::test]
    async fn list_data_sources_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/data-sources"), ""),
            json_response(
                200,
                r#"{"DataSources":[{"DataSourceId":"src-1","Arn":"arn:aws:quicksight:us-east-1:123456789012:datasource/src-1","Name":"Athena Source","Type":"ATHENA","Status":"CREATION_SUCCESSFUL","CreatedTime":1704067200,"LastUpdatedTime":1704153600},{"DataSourceId":"src-2"}]}"#,
            ),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_data_sources("123456789012".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        let s1 = &items[0];
        assert_eq!(s1.data_source_id, Some("src-1".to_string()));
        assert_eq!(s1.type_, Some("ATHENA".to_string()));
        assert_eq!(s1.status, Some("CREATION_SUCCESSFUL".to_string()));
        assert_eq!(s1.created_time, Some(DateTime::from_secs(1_704_067_200)));

        let s2 = &items[1];
        assert_eq!(s2.data_source_id, Some("src-2".to_string()));
        assert_eq!(s2.type_, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_data_sources_caps_at_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/accounts/123456789012/data-sources?max-results=1"),
                "",
            ),
            json_response(200, r#"{"DataSources":[{"DataSourceId":"src-1"}]}"#),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_data_sources("123456789012".to_string(), Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_data_sources_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/data-sources"), ""),
            json_error_response("InvalidParameterValueException", "bad data source filter"),
        )]);
        let client = QuickSightClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_data_sources("123456789012".to_string(), None, None)
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("bad data source filter"));
    }
}
