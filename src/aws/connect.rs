use aws_config::SdkConfig;
use aws_sdk_connect::types::{ContactFlowType, QueueType};
use chrono::{DateTime, Utc};

use crate::error::VaporError;
use crate::schema::time::to_utc;

#[derive(Debug)]
pub struct ConnectInstanceInfo {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub identity_management_type: Option<String>,
    pub instance_alias: Option<String>,
    pub created_time: Option<DateTime<Utc>>,
    pub service_role: Option<String>,
    pub instance_status: Option<String>,
    pub inbound_calls_enabled: Option<bool>,
    pub outbound_calls_enabled: Option<bool>,
}

pub struct ConnectQueueInfo {
    pub queue_id: Option<String>,
    pub queue_arn: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub queue_type: Option<String>,
    pub status: Option<String>,
}

pub struct ConnectContactFlowInfo {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub contact_flow_type: Option<String>,
    pub description: Option<String>,
}

pub struct ConnectUserInfo {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub username: Option<String>,
    pub routing_profile_id: Option<String>,
    pub hierarchy_group_id: Option<String>,
    pub security_profile_ids: Vec<String>,
}

pub struct ConnectClient {
    inner: aws_sdk_connect::Client,
}

impl ConnectClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_connect::Client::new(config),
        }
    }

    pub async fn list_instances(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ConnectInstanceInfo>, Option<String>), VaporError> {
        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_instances();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            summaries.extend(output.instance_summary_list.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let items = summaries
            .into_iter()
            .map(|inst| ConnectInstanceInfo {
                id: inst.id().map(|s| s.to_string()),
                arn: inst.arn().map(|s| s.to_string()),
                identity_management_type: inst
                    .identity_management_type()
                    .map(|t| t.as_str().to_string()),
                instance_alias: inst.instance_alias().map(|s| s.to_string()),
                created_time: to_utc(inst.created_time()),
                service_role: inst.service_role().map(|s| s.to_string()),
                instance_status: inst.instance_status().map(|s| s.as_str().to_string()),
                inbound_calls_enabled: inst.inbound_calls_enabled(),
                outbound_calls_enabled: inst.outbound_calls_enabled(),
            })
            .collect();

        Ok((items, token))
    }

    pub async fn list_queues(
        &self,
        instance_id: &str,
        queue_types: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ConnectQueueInfo>, Option<String>), VaporError> {
        let sdk_types: Option<Vec<QueueType>> =
            queue_types.map(|types| types.iter().map(|s| QueueType::from(s.as_str())).collect());

        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_queues().instance_id(instance_id);
            if let Some(ref types) = sdk_types {
                req = req.set_queue_types(Some(types.clone()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            summaries.extend(output.queue_summary_list.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut items = Vec::with_capacity(summaries.len());
        for queue in summaries {
            let queue_id = queue.id().map(|s| s.to_string());

            let (description, status) = if let Some(ref qid) = queue_id {
                match self
                    .inner
                    .describe_queue()
                    .instance_id(instance_id)
                    .queue_id(qid)
                    .send()
                    .await
                    .ok()
                    .and_then(|o| o.queue().cloned())
                {
                    Some(q) => (
                        q.description().map(|s| s.to_string()),
                        q.status().map(|s| s.as_str().to_string()),
                    ),
                    None => (None, None),
                }
            } else {
                (None, None)
            };

            items.push(ConnectQueueInfo {
                queue_id,
                queue_arn: queue.arn().map(|s| s.to_string()),
                name: queue.name().map(|s| s.to_string()),
                description,
                queue_type: queue.queue_type().map(|t| t.as_str().to_string()),
                status,
            });
        }

        Ok((items, token))
    }

    pub async fn list_contact_flows(
        &self,
        instance_id: &str,
        contact_flow_types: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ConnectContactFlowInfo>, Option<String>), VaporError> {
        let sdk_types: Option<Vec<ContactFlowType>> = contact_flow_types.map(|types| {
            types
                .iter()
                .map(|s| ContactFlowType::from(s.as_str()))
                .collect()
        });

        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_contact_flows().instance_id(instance_id);
            if let Some(ref types) = sdk_types {
                req = req.set_contact_flow_types(Some(types.clone()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            summaries.extend(output.contact_flow_summary_list.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut items = Vec::with_capacity(summaries.len());
        for flow in summaries {
            let flow_id = flow.id().map(|s| s.to_string());

            let description = if let Some(ref fid) = flow_id {
                self.inner
                    .describe_contact_flow()
                    .instance_id(instance_id)
                    .contact_flow_id(fid)
                    .send()
                    .await
                    .ok()
                    .and_then(|o| o.contact_flow().cloned())
                    .and_then(|cf| cf.description().map(|s| s.to_string()))
            } else {
                None
            };

            items.push(ConnectContactFlowInfo {
                id: flow_id,
                arn: flow.arn().map(|s| s.to_string()),
                name: flow.name().map(|s| s.to_string()),
                contact_flow_type: flow.contact_flow_type().map(|t| t.as_str().to_string()),
                description,
            });
        }

        Ok((items, token))
    }

    pub async fn list_users(
        &self,
        instance_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ConnectUserInfo>, Option<String>), VaporError> {
        let mut raw_summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_users().instance_id(instance_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - raw_summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            raw_summaries.extend(output.user_summary_list.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if raw_summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let summaries: Vec<_> = raw_summaries
            .into_iter()
            .map(|user| {
                (
                    user.id().map(|s| s.to_string()),
                    user.arn().map(|s| s.to_string()),
                    user.username().map(|s| s.to_string()),
                )
            })
            .collect();

        let mut items = Vec::new();
        for (id, arn, username) in summaries {
            let (routing_profile_id, hierarchy_group_id, security_profile_ids) =
                if let Some(ref uid) = id {
                    match self
                        .inner
                        .describe_user()
                        .user_id(uid)
                        .instance_id(instance_id)
                        .send()
                        .await
                        .ok()
                        .and_then(|o| o.user().cloned())
                    {
                        Some(user) => (
                            user.routing_profile_id().map(|s| s.to_string()),
                            user.hierarchy_group_id().map(|s| s.to_string()),
                            user.security_profile_ids().to_vec(),
                        ),
                        None => (None, None, Vec::new()),
                    }
                } else {
                    (None, None, Vec::new())
                };

            items.push(ConnectUserInfo {
                id,
                arn,
                username,
                routing_profile_id,
                hierarchy_group_id,
                security_profile_ids,
            });
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

    const BASE: &str = "https://connect.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_instances_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/instance"), ""),
            json_response(
                200,
                r#"{"InstanceSummaryList":[{"Id":"i1","Arn":"arn1","CreatedTime":0,"InstanceStatus":"ACTIVE","InboundCallsEnabled":true,"OutboundCallsEnabled":false},{"Id":"i2","Arn":"arn2"}]}"#,
            ),
        )]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_instances(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, Some("i1".to_string()));
        assert_eq!(items[0].created_time, Some(DateTime::<Utc>::UNIX_EPOCH));
        assert_eq!(items[0].instance_status, Some("ACTIVE".to_string()));
        assert_eq!(items[0].inbound_calls_enabled, Some(true));
        assert_eq!(items[0].outbound_calls_enabled, Some(false));
        assert_eq!(items[1].id, Some("i2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_instances_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/instance?nextToken=cursor-a"), ""),
            json_response(200, r#"{"InstanceSummaryList":[{"Id":"i3"}]}"#),
        )]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_instances(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_instances_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/instance?maxResults=2"), ""),
            json_response(
                200,
                r#"{"InstanceSummaryList":[{"Id":"i1"},{"Id":"i2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_instances(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_instances_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/instance?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"InstanceSummaryList":[{"Id":"i1"},{"Id":"i2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/instance?nextToken=p2&maxResults=8"), ""),
                json_response(200, r#"{"InstanceSummaryList":[{"Id":"i3"}]}"#),
            ),
        ]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_instances(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_instances_propagates_errors() {
        // `InvalidRequestException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/instance"), ""),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let err = client.list_instances(None, None).await.unwrap_err();

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
    async fn list_queues_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/queues-summary/inst1"), ""),
                json_response(
                    200,
                    r#"{"QueueSummaryList":[{"Id":"q1","Arn":"qarn1","Name":"queue-one","QueueType":"STANDARD"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/queues/inst1/q1"), ""),
                json_response(
                    200,
                    r#"{"Queue":{"QueueId":"q1","Description":"first queue","Status":"ENABLED"}}"#,
                ),
            ),
        ]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_queues("inst1", None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].queue_id, Some("q1".to_string()));
        assert_eq!(items[0].queue_arn, Some("qarn1".to_string()));
        assert_eq!(items[0].name, Some("queue-one".to_string()));
        assert_eq!(items[0].queue_type, Some("STANDARD".to_string()));
        assert_eq!(items[0].description, Some("first queue".to_string()));
        assert_eq!(items[0].status, Some("ENABLED".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queues_passes_through_queue_types_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/queues-summary/inst1?queueTypes=STANDARD"),
                "",
            ),
            json_response(200, r#"{"QueueSummaryList":[]}"#),
        )]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_queues("inst1", Some(vec!["STANDARD".to_string()]), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queues_skips_describe_fan_out_when_id_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/queues-summary/inst1"), ""),
            json_response(200, r#"{"QueueSummaryList":[{"Arn":"qarn-no-id"}]}"#),
        )]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_queues("inst1", None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].queue_id, None);
        assert_eq!(items[0].description, None);
        assert_eq!(items[0].status, None);
        assert_eq!(token, None);
        // Only 1 replay event registered above: if the fan-out fired despite
        // the missing id, this would panic on an unconsumed/missing event.
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queues_swallows_describe_errors_as_none_fields() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/queues-summary/inst1"), ""),
                json_response(200, r#"{"QueueSummaryList":[{"Id":"q1"}]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/queues/inst1/q1"), ""),
                json_error_response("AccessDeniedException", "not authorized"),
            ),
        ]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        // `list_queues` folds the per-queue `describe_queue` fan-out through
        // `.ok()`, so a describe-level error is swallowed into None fields
        // rather than propagating as a `VaporError` (unlike the top-level
        // `list_queues` call itself, which does propagate via `sdk_err`).
        let (items, token) = client.list_queues("inst1", None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].queue_id, Some("q1".to_string()));
        assert_eq!(items[0].description, None);
        assert_eq!(items[0].status, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_contact_flows_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/contact-flows-summary/inst1"), ""),
                json_response(
                    200,
                    r#"{"ContactFlowSummaryList":[{"Id":"f1","Arn":"farn1","Name":"flow-one","ContactFlowType":"CONTACT_FLOW"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/contact-flows/inst1/f1"), ""),
                json_response(
                    200,
                    r#"{"ContactFlow":{"Id":"f1","Description":"first flow"}}"#,
                ),
            ),
        ]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_contact_flows("inst1", None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, Some("f1".to_string()));
        assert_eq!(items[0].contact_flow_type, Some("CONTACT_FLOW".to_string()));
        assert_eq!(items[0].description, Some("first flow".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_contact_flows_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/contact-flows-summary/inst1?maxResults=1"),
                    "",
                ),
                json_response(
                    200,
                    r#"{"ContactFlowSummaryList":[{"Id":"f1"}],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/contact-flows/inst1/f1"), ""),
                json_response(200, r#"{"ContactFlow":{"Id":"f1"}}"#),
            ),
        ]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_contact_flows("inst1", None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/users-summary/inst1"), ""),
                json_response(
                    200,
                    r#"{"UserSummaryList":[{"Id":"u1","Arn":"uarn1","Username":"alice"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/users/inst1/u1"), ""),
                json_response(
                    200,
                    r#"{"User":{"Id":"u1","RoutingProfileId":"rp1","HierarchyGroupId":"hg1","SecurityProfileIds":["sp1","sp2"]}}"#,
                ),
            ),
        ]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_users("inst1", None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, Some("u1".to_string()));
        assert_eq!(items[0].username, Some("alice".to_string()));
        assert_eq!(items[0].routing_profile_id, Some("rp1".to_string()));
        assert_eq!(items[0].hierarchy_group_id, Some("hg1".to_string()));
        assert_eq!(
            items[0].security_profile_ids,
            vec!["sp1".to_string(), "sp2".to_string()]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/users-summary/inst1?maxResults=1"), ""),
                json_response(
                    200,
                    r#"{"UserSummaryList":[{"Id":"u1"}],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/users/inst1/u1"), ""),
                json_response(200, r#"{"User":{"Id":"u1"}}"#),
            ),
        ]);
        let client = ConnectClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_users("inst1", Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }
}
