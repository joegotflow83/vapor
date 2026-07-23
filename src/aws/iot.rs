use aws_config::SdkConfig;
use aws_smithy_types::DateTime;

use crate::error::VaporError;

#[derive(Debug)]
pub struct IotTagPair {
    pub key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct IotThingInfo {
    pub thing_name: Option<String>,
    pub thing_arn: Option<String>,
    pub thing_type_name: Option<String>,
    pub attributes: Vec<IotTagPair>,
    pub version: Option<i64>,
}

#[derive(Debug)]
pub struct IotThingGroupInfo {
    pub group_name: Option<String>,
    pub group_arn: Option<String>,
    pub group_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug)]
pub struct IotPolicyInfo {
    pub policy_name: Option<String>,
    pub policy_arn: Option<String>,
}

#[derive(Debug)]
pub struct IotCertificateInfo {
    pub certificate_id: Option<String>,
    pub certificate_arn: Option<String>,
    pub status: Option<String>,
    pub creation_date: Option<DateTime>,
}

#[derive(Debug)]
pub struct IotTopicRuleInfo {
    pub rule_name: Option<String>,
    pub topic_pattern: Option<String>,
    pub created_at: Option<DateTime>,
    pub rule_disabled: Option<bool>,
    pub rule_arn: Option<String>,
}

pub struct IotClient {
    inner: aws_sdk_iot::Client,
}

impl IotClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_iot::Client::new(config),
        }
    }

    /// Lists things, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListThings` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-iot` 1.118.0's
    /// `operation/list_things/_list_things_input.rs`), so `limit` is capped
    /// on the request itself; `.into_paginator()` dropped since it hides
    /// the token (kinesis/translate pattern).
    pub async fn list_things(
        &self,
        thing_type_name: Option<String>,
        attribute_name: Option<String>,
        attribute_value: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<IotThingInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_things();
            if let Some(ref t) = thing_type_name {
                req = req.thing_type_name(t);
            }
            if let Some(ref n) = attribute_name {
                req = req.attribute_name(n);
            }
            if let Some(ref v) = attribute_value {
                req = req.attribute_value(v);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for thing in output.things.unwrap_or_default() {
                let attributes: Vec<IotTagPair> = thing
                    .attributes
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(key, value)| IotTagPair { key, value })
                    .collect();
                items.push(IotThingInfo {
                    thing_name: thing.thing_name,
                    thing_arn: thing.thing_arn,
                    thing_type_name: thing.thing_type_name,
                    attributes,
                    version: Some(thing.version),
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

    /// Lists thing groups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListThingGroups` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-iot` 1.118.0's
    /// `operation/list_thing_groups/_list_thing_groups_input.rs`). N+1
    /// `describe_thing_group` fan-out covers only the group names collected
    /// for this page (codecommit::list_branches precedent).
    pub async fn list_thing_groups(
        &self,
        parent_group: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<IotThingGroupInfo>, Option<String>), VaporError> {
        let mut groups: Vec<(Option<String>, Option<String>)> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_thing_groups();
            if let Some(ref pg) = parent_group {
                req = req.parent_group(pg);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - groups.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for g in output.thing_groups.unwrap_or_default() {
                groups.push((g.group_name, g.group_arn));
            }

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if groups.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut items = Vec::new();
        for (group_name, group_arn) in groups {
            let (group_id, status) = if let Some(ref name) = group_name {
                let result = self
                    .inner
                    .describe_thing_group()
                    .thing_group_name(name)
                    .send()
                    .await;
                match result {
                    Ok(output) => (
                        output.thing_group_id().map(|s| s.to_string()),
                        output.status().map(|s| s.as_str().to_string()),
                    ),
                    Err(_) => (None, None),
                }
            } else {
                (None, None)
            };
            items.push(IotThingGroupInfo {
                group_name,
                group_arn,
                group_id,
                status,
            });
        }

        Ok((items, token))
    }

    /// Lists policies, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListPolicies` has
    /// `page_size` (input, not `max_results`) and `marker`/`next_marker`
    /// (input/output, not `next_token`) — verified against pinned
    /// `aws-sdk-iot` 1.118.0's
    /// `operation/list_policies/_list_policies_{input,output}.rs`
    /// (mq/msk-class naming mismatch).
    pub async fn list_policies(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<IotPolicyInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_policies();
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.page_size(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_marker;
            for policy in output.policies.unwrap_or_default() {
                items.push(IotPolicyInfo {
                    policy_name: policy.policy_name,
                    policy_arn: policy.policy_arn,
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

    /// Lists certificates, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListCertificates` has
    /// `page_size` (input, not `max_results`) and `marker`/`next_marker`
    /// (input/output, not `next_token`) — same naming-mismatch class as
    /// `list_policies` above (verified against pinned `aws-sdk-iot`
    /// 1.118.0's
    /// `operation/list_certificates/_list_certificates_{input,output}.rs`).
    pub async fn list_certificates(
        &self,
        ascending_order: Option<bool>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<IotCertificateInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_certificates();
            if let Some(asc) = ascending_order {
                req = req.ascending_order(asc);
            }
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.page_size(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_marker;
            for cert in output.certificates.unwrap_or_default() {
                items.push(IotCertificateInfo {
                    certificate_id: cert.certificate_id,
                    certificate_arn: cert.certificate_arn,
                    status: cert.status.map(|s| s.as_str().to_string()),
                    creation_date: cert.creation_date,
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

    /// Lists topic rules, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListTopicRules` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-iot` 1.118.0's
    /// `operation/list_topic_rules/_list_topic_rules_input.rs`).
    pub async fn list_topic_rules(
        &self,
        topic_rule_disabled: Option<bool>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<IotTopicRuleInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_topic_rules();
            if let Some(disabled) = topic_rule_disabled {
                req = req.rule_disabled(disabled);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for rule in output.rules.unwrap_or_default() {
                items.push(IotTopicRuleInfo {
                    rule_name: rule.rule_name,
                    topic_pattern: rule.topic_pattern,
                    created_at: rule.created_at,
                    rule_disabled: rule.rule_disabled,
                    rule_arn: rule.rule_arn,
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
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::error::VaporError;

    const BASE: &str = "https://iot.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_things_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/things"), ""),
            json_response(
                200,
                r#"{"things":[{"thingName":"t1","thingArn":"arn1","thingTypeName":"sensor","attributes":{"loc":"nyc"},"version":1},{"thingName":"t2","version":2}]}"#,
            ),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_things(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].thing_name, Some("t1".to_string()));
        assert_eq!(items[0].thing_arn, Some("arn1".to_string()));
        assert_eq!(items[0].thing_type_name, Some("sensor".to_string()));
        assert_eq!(items[0].attributes.len(), 1);
        assert_eq!(items[0].attributes[0].key, "loc");
        assert_eq!(items[0].attributes[0].value, "nyc");
        assert_eq!(items[0].version, Some(1));
        assert_eq!(items[1].thing_name, Some("t2".to_string()));
        assert_eq!(items[1].version, Some(2));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_things_passes_through_attribute_and_type_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/things?attributeName=loc&attributeValue=nyc&thingTypeName=sensor"),
                "",
            ),
            json_response(200, r#"{"things":[]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_things(
                Some("sensor".to_string()),
                Some("loc".to_string()),
                Some("nyc".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_things_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/things?nextToken=cursor-a"), ""),
            json_response(200, r#"{"things":[{"thingName":"t2","version":2}]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_things(None, None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_things_stops_at_limit_and_returns_resume_token() {
        // `ListThings` forwards `maxResults` straight to AWS with no
        // client-side truncate, so the canned response must return exactly
        // `limit` items (gotcha 13's AWS-side category).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/things?maxResults=1"), ""),
            json_response(
                200,
                r#"{"things":[{"thingName":"t1","version":1}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_things(None, None, None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_things_propagates_errors() {
        // `InvalidRequestException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/things"), ""),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let err = client.list_things(None, None, None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_thing_groups_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/thing-groups"), ""),
                json_response(200, r#"{"thingGroups":[{"groupName":"g1","groupArn":"garn1"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/thing-groups/g1"), ""),
                json_response(200, r#"{"thingGroupId":"gid1","status":"ACTIVE"}"#),
            ),
        ]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_thing_groups(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].group_name, Some("g1".to_string()));
        assert_eq!(items[0].group_arn, Some("garn1".to_string()));
        assert_eq!(items[0].group_id, Some("gid1".to_string()));
        assert_eq!(items[0].status, Some("ACTIVE".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_thing_groups_passes_through_parent_group_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/thing-groups?parentGroup=factory"), ""),
            json_response(200, r#"{"thingGroups":[]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_thing_groups(Some("factory".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_thing_groups_skips_describe_fan_out_when_name_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/thing-groups"), ""),
            json_response(200, r#"{"thingGroups":[{"groupArn":"garn-no-name"}]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_thing_groups(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].group_name, None);
        assert_eq!(items[0].group_arn, Some("garn-no-name".to_string()));
        assert_eq!(items[0].group_id, None);
        assert_eq!(items[0].status, None);
        assert_eq!(token, None);
        // Only 1 replay event registered above: if the fan-out fired despite
        // the missing name, this would panic on an unconsumed/missing event.
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_thing_groups_swallows_describe_errors_as_none_fields() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/thing-groups"), ""),
                json_response(200, r#"{"thingGroups":[{"groupName":"g1"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/thing-groups/g1"), ""),
                json_error_response("InvalidRequestException", "not found"),
            ),
        ]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        // `list_thing_groups` folds the per-group `describe_thing_group`
        // fan-out through a plain `match ... Err(_) => (None, None)`, so a
        // describe-level error is swallowed rather than propagating as a
        // `VaporError` (unlike the top-level `list_thing_groups` call
        // itself, which does propagate via `sdk_err`).
        let (items, token) = client.list_thing_groups(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].group_name, Some("g1".to_string()));
        assert_eq!(items[0].group_id, None);
        assert_eq!(items[0].status, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_thing_groups_stops_at_limit_and_returns_resume_token() {
        // `ListThingGroups` forwards `maxResults` straight to AWS with no
        // client-side truncate; the fan-out then runs once per group
        // collected on that already-AWS-capped page (elasticache.rs-style
        // shape, not the ecs.rs "always fires regardless of page 1 limit"
        // shape) — 1 discovery event + 1 fan-out event for a limit of 1.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/thing-groups?maxResults=1"), ""),
                json_response(
                    200,
                    r#"{"thingGroups":[{"groupName":"g1","groupArn":"garn1"}],"nextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/thing-groups/g1"), ""),
                json_response(200, r#"{"thingGroupId":"gid1"}"#),
            ),
        ]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_thing_groups(None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_thing_groups_propagates_discovery_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/thing-groups"), ""),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let err = client.list_thing_groups(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/policies"), ""),
            json_response(200, r#"{"policies":[{"policyName":"p1","policyArn":"parn1"}]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_policies(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].policy_name, Some("p1".to_string()));
        assert_eq!(items[0].policy_arn, Some("parn1".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/policies?marker=cursor-a"), ""),
            json_response(200, r#"{"policies":[{"policyName":"p2"}]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_policies(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_stops_at_limit_and_returns_resume_token() {
        // `ListPolicies` has `pageSize` (input, not `maxResults`) forwarded
        // straight to AWS with no client-side truncate (gotcha 13's
        // AWS-side category, mq/msk-class naming mismatch).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/policies?pageSize=1"), ""),
            json_response(
                200,
                r#"{"policies":[{"policyName":"p1"}],"nextMarker":"m2"}"#,
            ),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_policies(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("m2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/policies"), ""),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let err = client.list_policies(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificates_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/certificates"), ""),
            json_response(
                200,
                r#"{"certificates":[{"certificateId":"c1","certificateArn":"carn1","status":"ACTIVE","creationDate":1700000000}]}"#,
            ),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_certificates(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].certificate_id, Some("c1".to_string()));
        assert_eq!(items[0].certificate_arn, Some("carn1".to_string()));
        assert_eq!(items[0].status, Some("ACTIVE".to_string()));
        assert_eq!(items[0].creation_date, Some(DateTime::from_secs(1_700_000_000)));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificates_passes_through_ascending_order_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/certificates?isAscendingOrder=true"), ""),
            json_response(200, r#"{"certificates":[]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_certificates(Some(true), None, None).await.unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificates_stops_at_limit_and_returns_resume_token() {
        // `ListCertificates` has `pageSize` (not `maxResults`) forwarded
        // straight to AWS with no client-side truncate — same naming
        // mismatch class as `list_policies` above.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/certificates?pageSize=1"), ""),
            json_response(
                200,
                r#"{"certificates":[{"certificateId":"c1"}],"nextMarker":"m2"}"#,
            ),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_certificates(None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("m2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificates_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/certificates"), ""),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let err = client.list_certificates(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topic_rules_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/rules"), ""),
            json_response(
                200,
                r#"{"rules":[{"ruleName":"r1","topicPattern":"sensors/+/temp","createdAt":1700000000,"ruleDisabled":false,"ruleArn":"rarn1"}]}"#,
            ),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_topic_rules(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].rule_name, Some("r1".to_string()));
        assert_eq!(items[0].topic_pattern, Some("sensors/+/temp".to_string()));
        assert_eq!(items[0].created_at, Some(DateTime::from_secs(1_700_000_000)));
        assert_eq!(items[0].rule_disabled, Some(false));
        assert_eq!(items[0].rule_arn, Some("rarn1".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topic_rules_passes_through_disabled_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/rules?ruleDisabled=true"), ""),
            json_response(200, r#"{"rules":[]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_topic_rules(Some(true), None, None).await.unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topic_rules_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/rules?nextToken=cursor-a"), ""),
            json_response(200, r#"{"rules":[{"ruleName":"r2"}]}"#),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_topic_rules(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topic_rules_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/rules?maxResults=1"), ""),
            json_response(
                200,
                r#"{"rules":[{"ruleName":"r1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_topic_rules(None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topic_rules_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/rules"), ""),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = IotClient::new(&sdk_config(http_client.clone()));

        let err = client.list_topic_rules(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
