use aws_config::SdkConfig;
use aws_sdk_wafv2::types::Scope;

use crate::error::VaporError;

pub struct WafV2Client {
    inner: aws_sdk_wafv2::Client,
}

impl WafV2Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_wafv2::Client::new(config),
        }
    }

    /// Lists Web ACL summaries, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListWebACLs` has
    /// both `limit` and `next_marker` (verified against pinned
    /// `aws-sdk-wafv2` 1.119.0's
    /// `operation/list_web_acls/_list_web_acls_{input,output}.rs`), with no
    /// documented minimum on `limit` (unlike the `max_records`-floor SDKs),
    /// so no clamping is needed here.
    pub async fn list_web_acls(
        &self,
        scope: Scope,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_wafv2::types::WebAclSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_web_acls().scope(scope.clone());
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                req = req.next_marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.web_acls().to_vec());

            marker = match output.next_marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            items.truncate(l.max(0) as usize);
        }

        Ok((items, marker))
    }

    pub async fn get_web_acl(
        &self,
        name: &str,
        id: &str,
        scope: Scope,
    ) -> Result<aws_sdk_wafv2::operation::get_web_acl::GetWebAclOutput, VaporError> {
        self.inner
            .get_web_acl()
            .name(name)
            .id(id)
            .scope(scope)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Lists IP set summaries, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. Same
    /// no-minimum-`limit` note as `list_web_acls` above.
    pub async fn list_ip_sets(
        &self,
        scope: Scope,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_wafv2::types::IpSetSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_ip_sets().scope(scope.clone());
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                req = req.next_marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.ip_sets().to_vec());

            marker = match output.next_marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            items.truncate(l.max(0) as usize);
        }

        Ok((items, marker))
    }

    pub async fn get_ip_set(
        &self,
        name: &str,
        id: &str,
        scope: Scope,
    ) -> Result<aws_sdk_wafv2::operation::get_ip_set::GetIpSetOutput, VaporError> {
        self.inner
            .get_ip_set()
            .name(name)
            .id(id)
            .scope(scope)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Lists rule group summaries, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. Same
    /// no-minimum-`limit` note as `list_web_acls` above.
    pub async fn list_rule_groups(
        &self,
        scope: Scope,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_wafv2::types::RuleGroupSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_rule_groups().scope(scope.clone());
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                req = req.next_marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.rule_groups().to_vec());

            marker = match output.next_marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            items.truncate(l.max(0) as usize);
        }

        Ok((items, marker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://wafv2.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_web_acls_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL"}"#),
            json_response(
                200,
                r#"{"WebACLs":[{"Name":"acl1","Id":"id1","ARN":"arn:acl1","Description":"desc1","LockToken":"lock1"},{"Name":"acl2","Id":"id2","ARN":"arn:acl2"}]}"#,
            ),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_web_acls(Scope::Regional, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name(), Some("acl1"));
        assert_eq!(items[0].id(), Some("id1"));
        assert_eq!(items[0].arn(), Some("arn:acl1"));
        assert_eq!(items[0].description(), Some("desc1"));
        assert_eq!(items[0].lock_token(), Some("lock1"));
        assert_eq!(items[1].description(), None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_web_acls_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL","NextMarker":"cursor-a"}"#),
            json_response(200, r#"{"WebACLs":[{"Name":"acl3","Id":"id3","ARN":"arn:acl3"}]}"#),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_web_acls(Scope::Regional, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_web_acls_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL","Limit":2}"#),
            json_response(
                200,
                r#"{"WebACLs":[{"Name":"acl1","Id":"id1","ARN":"arn:acl1"},{"Name":"acl2","Id":"id2","ARN":"arn:acl2"}],"NextMarker":"page2-token"}"#,
            ),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_web_acls(Scope::Regional, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_web_acls_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"REGIONAL","Limit":10}"#),
                json_response(
                    200,
                    r#"{"WebACLs":[{"Name":"acl1","Id":"id1","ARN":"arn:acl1"},{"Name":"acl2","Id":"id2","ARN":"arn:acl2"}],"NextMarker":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"REGIONAL","Limit":8,"NextMarker":"p2"}"#),
                json_response(200, r#"{"WebACLs":[{"Name":"acl3","Id":"id3","ARN":"arn:acl3"}]}"#),
            ),
        ]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_web_acls(Scope::Regional, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_web_acls_propagates_errors() {
        // `WAFInvalidParameterException`, not a throttling-classified code
        // (see memory gotcha 1: those get retried and exhaust the single
        // replay event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL"}"#),
            json_error_response("WAFInvalidParameterException", "bad parameter"),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .list_web_acls(Scope::Regional, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("WAFInvalidParameterException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_web_acl_returns_details() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Name":"acl1","Scope":"REGIONAL","Id":"id1"}"#),
            json_response(
                200,
                r#"{"WebACL":{"Name":"acl1","Id":"id1","ARN":"arn:acl1"},"LockToken":"lock1"}"#,
            ),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let output = client
            .get_web_acl("acl1", "id1", Scope::Regional)
            .await
            .unwrap();

        assert_eq!(output.web_acl().map(|w| w.name()), Some("acl1"));
        assert_eq!(output.web_acl().map(|w| w.arn()), Some("arn:acl1"));
        assert_eq!(output.lock_token(), Some("lock1"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_web_acl_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Name":"acl1","Scope":"REGIONAL","Id":"id1"}"#),
            json_error_response("WAFNonexistentItemException", "web acl not found"),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .get_web_acl("acl1", "id1", Scope::Regional)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("WAFNonexistentItemException".to_string()));
                assert_eq!(message, "web acl not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ip_sets_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL"}"#),
            json_response(
                200,
                r#"{"IPSets":[{"Name":"ipset1","Id":"id1","ARN":"arn:ipset1","Description":"desc1","LockToken":"lock1"},{"Name":"ipset2","Id":"id2","ARN":"arn:ipset2"}]}"#,
            ),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_ip_sets(Scope::Regional, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name(), Some("ipset1"));
        assert_eq!(items[0].id(), Some("id1"));
        assert_eq!(items[0].arn(), Some("arn:ipset1"));
        assert_eq!(items[0].description(), Some("desc1"));
        assert_eq!(items[0].lock_token(), Some("lock1"));
        assert_eq!(items[1].description(), None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ip_sets_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL","NextMarker":"cursor-a"}"#),
            json_response(200, r#"{"IPSets":[{"Name":"ipset3","Id":"id3","ARN":"arn:ipset3"}]}"#),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_ip_sets(Scope::Regional, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ip_sets_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL","Limit":2}"#),
            json_response(
                200,
                r#"{"IPSets":[{"Name":"ipset1","Id":"id1","ARN":"arn:ipset1"},{"Name":"ipset2","Id":"id2","ARN":"arn:ipset2"}],"NextMarker":"page2-token"}"#,
            ),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_ip_sets(Scope::Regional, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ip_sets_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"REGIONAL","Limit":10}"#),
                json_response(
                    200,
                    r#"{"IPSets":[{"Name":"ipset1","Id":"id1","ARN":"arn:ipset1"},{"Name":"ipset2","Id":"id2","ARN":"arn:ipset2"}],"NextMarker":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"REGIONAL","Limit":8,"NextMarker":"p2"}"#),
                json_response(200, r#"{"IPSets":[{"Name":"ipset3","Id":"id3","ARN":"arn:ipset3"}]}"#),
            ),
        ]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_ip_sets(Scope::Regional, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_ip_sets_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL"}"#),
            json_error_response("WAFInvalidParameterException", "bad parameter"),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .list_ip_sets(Scope::Regional, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("WAFInvalidParameterException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_ip_set_returns_details() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Name":"ipset1","Scope":"REGIONAL","Id":"id1"}"#),
            json_response(
                200,
                r#"{"IPSet":{"Name":"ipset1","Id":"id1","ARN":"arn:ipset1","IPAddressVersion":"IPV4","Addresses":["10.0.0.0/8"]},"LockToken":"lock1"}"#,
            ),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let output = client
            .get_ip_set("ipset1", "id1", Scope::Regional)
            .await
            .unwrap();

        let ip_set = output.ip_set().unwrap();
        assert_eq!(ip_set.name(), "ipset1");
        assert_eq!(ip_set.arn(), "arn:ipset1");
        assert_eq!(ip_set.addresses(), ["10.0.0.0/8"]);
        assert_eq!(output.lock_token(), Some("lock1"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_ip_set_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Name":"ipset1","Scope":"REGIONAL","Id":"id1"}"#),
            json_error_response("WAFNonexistentItemException", "ip set not found"),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .get_ip_set("ipset1", "id1", Scope::Regional)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("WAFNonexistentItemException".to_string()));
                assert_eq!(message, "ip set not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL"}"#),
            json_response(
                200,
                r#"{"RuleGroups":[{"Name":"rg1","Id":"id1","ARN":"arn:rg1","Description":"desc1","LockToken":"lock1"},{"Name":"rg2","Id":"id2","ARN":"arn:rg2"}]}"#,
            ),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_rule_groups(Scope::Regional, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name(), Some("rg1"));
        assert_eq!(items[0].id(), Some("id1"));
        assert_eq!(items[0].arn(), Some("arn:rg1"));
        assert_eq!(items[0].description(), Some("desc1"));
        assert_eq!(items[0].lock_token(), Some("lock1"));
        assert_eq!(items[1].description(), None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL","NextMarker":"cursor-a"}"#),
            json_response(200, r#"{"RuleGroups":[{"Name":"rg3","Id":"id3","ARN":"arn:rg3"}]}"#),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_rule_groups(Scope::Regional, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL","Limit":2}"#),
            json_response(
                200,
                r#"{"RuleGroups":[{"Name":"rg1","Id":"id1","ARN":"arn:rg1"},{"Name":"rg2","Id":"id2","ARN":"arn:rg2"}],"NextMarker":"page2-token"}"#,
            ),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_rule_groups(Scope::Regional, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"REGIONAL","Limit":10}"#),
                json_response(
                    200,
                    r#"{"RuleGroups":[{"Name":"rg1","Id":"id1","ARN":"arn:rg1"},{"Name":"rg2","Id":"id2","ARN":"arn:rg2"}],"NextMarker":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"REGIONAL","Limit":8,"NextMarker":"p2"}"#),
                json_response(200, r#"{"RuleGroups":[{"Name":"rg3","Id":"id3","ARN":"arn:rg3"}]}"#),
            ),
        ]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_rule_groups(Scope::Regional, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL"}"#),
            json_error_response("WAFInvalidParameterException", "bad parameter"),
        )]);
        let client = WafV2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .list_rule_groups(Scope::Regional, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("WAFInvalidParameterException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
