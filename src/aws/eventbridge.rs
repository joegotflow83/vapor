use aws_config::SdkConfig;
use aws_sdk_eventbridge::types::{EventBus, Rule, Target};

use crate::error::VaporError;

pub struct EventBridgeClient {
    inner: aws_sdk_eventbridge::Client,
}

impl EventBridgeClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_eventbridge::Client::new(config),
        }
    }

    /// Lists event buses, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListEventBuses` has no SDK
    /// paginator (confirmed: no `paginator.rs` under `aws-sdk-eventbridge`'s
    /// generated `operation/list_event_buses/`), so the loop is hand-rolled,
    /// but `limit` is handed to AWS via `ListEventBusesInput::limit` so a
    /// capped page boundary lands exactly on the returned token (kinesis.rs
    /// pattern).
    pub async fn list_event_buses(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<EventBus>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_event_buses();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.event_buses().to_vec());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists rules, optionally scoped to `event_bus_name` and capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `ListRules` has no SDK paginator (confirmed against the generated SDK
    /// source), same as `list_event_buses`.
    pub async fn list_rules(
        &self,
        event_bus_name: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Rule>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_rules();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(bus) = event_bus_name {
                req = req.event_bus_name(bus);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.rules().to_vec());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists targets for `rule_name`, optionally scoped to `event_bus_name`
    /// and capped at `limit` results (default unlimited) and resumed from
    /// `next_token`. `ListTargetsByRule` has no SDK paginator (confirmed
    /// against the generated SDK source), same as `list_event_buses`.
    pub async fn list_targets_by_rule(
        &self,
        rule_name: &str,
        event_bus_name: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Target>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_targets_by_rule().rule(rule_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(bus) = event_bus_name {
                req = req.event_bus_name(bus);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.targets().to_vec());
            token = output.next_token().map(|t| t.to_string());

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
    use crate::error::VaporError;
    use aws_sdk_eventbridge::types::RuleState;

    const ENDPOINT: &str = "https://events.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_event_buses_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"EventBuses":[{"Name":"default","Arn":"arn:aws:events:us-east-1:1:event-bus/default"},{"Name":"custom","Arn":"arn:aws:events:us-east-1:1:event-bus/custom"}]}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (buses, token) = client.list_event_buses(None, None).await.unwrap();

        assert_eq!(buses.len(), 2);
        assert_eq!(buses[0].name(), Some("default"));
        assert_eq!(buses[1].name(), Some("custom"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_event_buses_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"EventBuses":[{"Name":"bus3"}]}"#),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (buses, token) = client
            .list_event_buses(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(buses.len(), 1);
        assert_eq!(buses[0].name(), Some("bus3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_event_buses_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":2}"#),
            json_response(
                200,
                r#"{"EventBuses":[{"Name":"a"},{"Name":"b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (buses, token) = client.list_event_buses(Some(2), None).await.unwrap();

        assert_eq!(buses.len(), 2);
        assert_eq!(buses[0].name(), Some("a"));
        assert_eq!(buses[1].name(), Some("b"));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_event_buses_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":10}"#),
                json_response(
                    200,
                    r#"{"EventBuses":[{"Name":"a"},{"Name":"b"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","Limit":8}"#),
                json_response(200, r#"{"EventBuses":[{"Name":"c"}]}"#),
            ),
        ]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (buses, token) = client.list_event_buses(Some(10), None).await.unwrap();

        assert_eq!(buses.len(), 3);
        assert_eq!(buses[2].name(), Some("c"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_event_buses_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InternalException", "internal failure"),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let err = client.list_event_buses(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InternalException"));
                assert_eq!(message, "internal failure");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rules_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Rules":[{"Name":"rule-1","Arn":"arn:aws:events:us-east-1:1:rule/rule-1","State":"ENABLED"},{"Name":"rule-2","State":"DISABLED"}]}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (rules, token) = client.list_rules(None, None, None).await.unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name(), Some("rule-1"));
        assert_eq!(rules[0].state(), Some(&RuleState::Enabled));
        assert_eq!(rules[1].state(), Some(&RuleState::Disabled));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rules_passes_event_bus_name_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"EventBusName":"custom-bus"}"#),
            json_response(200, r#"{"Rules":[{"Name":"rule-3"}]}"#),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (rules, _token) = client
            .list_rules(Some("custom-bus"), None, None)
            .await
            .unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name(), Some("rule-3"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rules_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Rules":[{"Name":"rule-4"}]}"#),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (rules, token) = client
            .list_rules(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name(), Some("rule-4"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rules_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":2}"#),
            json_response(
                200,
                r#"{"Rules":[{"Name":"a"},{"Name":"b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (rules, token) = client.list_rules(None, Some(2), None).await.unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name(), Some("a"));
        assert_eq!(rules[1].name(), Some("b"));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rules_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":10}"#),
                json_response(
                    200,
                    r#"{"Rules":[{"Name":"a"},{"Name":"b"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","Limit":8}"#),
                json_response(200, r#"{"Rules":[{"Name":"c"}]}"#),
            ),
        ]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (rules, token) = client.list_rules(None, Some(10), None).await.unwrap();

        assert_eq!(rules.len(), 3);
        assert_eq!(rules[2].name(), Some("c"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rules_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("ResourceNotFoundException", "event bus not found"),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let err = client.list_rules(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("ResourceNotFoundException"));
                assert_eq!(message, "event bus not found");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_targets_by_rule_lists_all_when_no_filter_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Rule":"my-rule"}"#),
            json_response(
                200,
                r#"{"Targets":[{"Id":"1","Arn":"arn:aws:lambda:us-east-1:1:function:fn1"},{"Id":"2","Arn":"arn:aws:sqs:us-east-1:1:queue2"}]}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (targets, token) = client
            .list_targets_by_rule("my-rule", None, None, None)
            .await
            .unwrap();

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id(), "1");
        assert_eq!(targets[1].id(), "2");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_targets_by_rule_passes_event_bus_name_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Rule":"my-rule","EventBusName":"custom-bus"}"#),
            json_response(200, r#"{"Targets":[{"Id":"3","Arn":"arn3"}]}"#),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (targets, _token) = client
            .list_targets_by_rule("my-rule", Some("custom-bus"), None, None)
            .await
            .unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id(), "3");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_targets_by_rule_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Rule":"my-rule","NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Targets":[{"Id":"4","Arn":"arn4"}]}"#),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (targets, token) = client
            .list_targets_by_rule("my-rule", None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id(), "4");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_targets_by_rule_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Rule":"my-rule","Limit":2}"#),
            json_response(
                200,
                r#"{"Targets":[{"Id":"a","Arn":"arn-a"},{"Id":"b","Arn":"arn-b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (targets, token) = client
            .list_targets_by_rule("my-rule", None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id(), "a");
        assert_eq!(targets[1].id(), "b");
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_targets_by_rule_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Rule":"my-rule","Limit":10}"#),
                json_response(
                    200,
                    r#"{"Targets":[{"Id":"a","Arn":"arn-a"},{"Id":"b","Arn":"arn-b"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Rule":"my-rule","NextToken":"p2","Limit":8}"#),
                json_response(200, r#"{"Targets":[{"Id":"c","Arn":"arn-c"}]}"#),
            ),
        ]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let (targets, token) = client
            .list_targets_by_rule("my-rule", None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(targets.len(), 3);
        assert_eq!(targets[2].id(), "c");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_targets_by_rule_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Rule":"missing-rule"}"#),
            json_error_response("ResourceNotFoundException", "rule not found"),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_targets_by_rule("missing-rule", None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("ResourceNotFoundException"));
                assert_eq!(message, "rule not found");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

