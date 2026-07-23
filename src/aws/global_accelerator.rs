use aws_config::SdkConfig;
use aws_sdk_globalaccelerator::types::{Accelerator, EndpointGroup, Listener};

use crate::error::VaporError;

pub struct GlobalAcceleratorClient {
    inner: aws_sdk_globalaccelerator::Client,
}

impl GlobalAcceleratorClient {
    pub fn new(config: &SdkConfig) -> Self {
        let ga_config = aws_sdk_globalaccelerator::config::Builder::from(config)
            .region(aws_sdk_globalaccelerator::config::Region::new("us-west-2"))
            .build();
        Self {
            inner: aws_sdk_globalaccelerator::Client::from_conf(ga_config),
        }
    }

    /// Lists accelerators, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListAccelerators` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-globalaccelerator` 1.104.0's
    /// `operation/list_accelerators/_list_accelerators_input.rs`), so `limit`
    /// is capped to the remaining budget on the request itself, matching
    /// `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_accelerators(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Accelerator>, Option<String>), VaporError> {
        let mut accelerators = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_accelerators();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - accelerators.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            accelerators.extend(output.accelerators.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if accelerators.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((accelerators, token))
    }

    /// Lists listeners for an accelerator, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListListeners` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-globalaccelerator` 1.104.0's
    /// `operation/list_listeners/_list_listeners_input.rs`), same pattern.
    pub async fn list_listeners(
        &self,
        accelerator_arn: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Listener>, Option<String>), VaporError> {
        let mut listeners = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_listeners().accelerator_arn(accelerator_arn);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - listeners.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            listeners.extend(output.listeners.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if listeners.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((listeners, token))
    }

    /// Lists endpoint groups for a listener, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListEndpointGroups` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-globalaccelerator` 1.104.0's
    /// `operation/list_endpoint_groups/_list_endpoint_groups_input.rs`), same
    /// pattern.
    pub async fn list_endpoint_groups(
        &self,
        listener_arn: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<EndpointGroup>, Option<String>), VaporError> {
        let mut groups = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_endpoint_groups().listener_arn(listener_arn);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - groups.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            groups.extend(output.endpoint_groups.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if groups.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((groups, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://globalaccelerator.us-west-2.amazonaws.com/";

    #[tokio::test]
    async fn list_accelerators_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Accelerators":[{"AcceleratorArn":"arn:aws:globalaccelerator::111111111111:accelerator/acc-a","Name":"acc-a-name"},{"AcceleratorArn":"arn:aws:globalaccelerator::111111111111:accelerator/acc-b","Name":"acc-b-name"}]}"#,
            ),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (accelerators, token) = client.list_accelerators(None, None).await.unwrap();

        assert_eq!(accelerators.len(), 2);
        assert_eq!(
            accelerators[0].accelerator_arn(),
            Some("arn:aws:globalaccelerator::111111111111:accelerator/acc-a")
        );
        assert_eq!(
            accelerators[1].accelerator_arn(),
            Some("arn:aws:globalaccelerator::111111111111:accelerator/acc-b")
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_accelerators_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"tok-1"}"#),
            json_response(
                200,
                r#"{"Accelerators":[{"AcceleratorArn":"arn:aws:globalaccelerator::111111111111:accelerator/acc-b"}]}"#,
            ),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (accelerators, token) = client
            .list_accelerators(None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(accelerators.len(), 1);
        assert_eq!(
            accelerators[0].accelerator_arn(),
            Some("arn:aws:globalaccelerator::111111111111:accelerator/acc-b")
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_accelerators_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Accelerators":[{"AcceleratorArn":"arn:aws:globalaccelerator::111111111111:accelerator/acc-a"},{"AcceleratorArn":"arn:aws:globalaccelerator::111111111111:accelerator/acc-b"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (accelerators, token) = client.list_accelerators(Some(2), None).await.unwrap();

        assert_eq!(accelerators.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_accelerators_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"Accelerators":[{"AcceleratorArn":"arn:a"},{"AcceleratorArn":"arn:b"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"tok-3","MaxResults":1}"#),
                json_response(200, r#"{"Accelerators":[{"AcceleratorArn":"arn:c"}]}"#),
            ),
        ]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (accelerators, token) = client.list_accelerators(Some(3), None).await.unwrap();

        assert_eq!(accelerators.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_accelerators_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidArgumentException", "invalid argument"),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let err = client.list_accelerators(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidArgumentException"));
                assert_eq!(message, "invalid argument");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_listeners_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AcceleratorArn":"acc-1"}"#),
            json_response(
                200,
                r#"{"Listeners":[{"ListenerArn":"arn:listener-a"},{"ListenerArn":"arn:listener-b"}]}"#,
            ),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (listeners, token) = client.list_listeners("acc-1", None, None).await.unwrap();

        assert_eq!(listeners.len(), 2);
        assert_eq!(listeners[0].listener_arn(), Some("arn:listener-a"));
        assert_eq!(listeners[1].listener_arn(), Some("arn:listener-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_listeners_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AcceleratorArn":"acc-1","NextToken":"tok-1"}"#),
            json_response(200, r#"{"Listeners":[{"ListenerArn":"arn:listener-b"}]}"#),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (listeners, token) = client
            .list_listeners("acc-1", None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(listeners.len(), 1);
        assert_eq!(listeners[0].listener_arn(), Some("arn:listener-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_listeners_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AcceleratorArn":"acc-1","MaxResults":2}"#),
            json_response(
                200,
                r#"{"Listeners":[{"ListenerArn":"arn:listener-a"},{"ListenerArn":"arn:listener-b"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (listeners, token) = client.list_listeners("acc-1", Some(2), None).await.unwrap();

        assert_eq!(listeners.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_listeners_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"AcceleratorArn":"acc-1","MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"Listeners":[{"ListenerArn":"arn:a"},{"ListenerArn":"arn:b"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"AcceleratorArn":"acc-1","NextToken":"tok-3","MaxResults":1}"#,
                ),
                json_response(200, r#"{"Listeners":[{"ListenerArn":"arn:c"}]}"#),
            ),
        ]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (listeners, token) = client.list_listeners("acc-1", Some(3), None).await.unwrap();

        assert_eq!(listeners.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_listeners_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AcceleratorArn":"acc-1"}"#),
            json_error_response("InvalidArgumentException", "invalid argument"),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_listeners("acc-1", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidArgumentException"));
                assert_eq!(message, "invalid argument");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoint_groups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ListenerArn":"listener-1"}"#),
            json_response(
                200,
                r#"{"EndpointGroups":[{"EndpointGroupArn":"arn:group-a","EndpointGroupRegion":"us-east-1"},{"EndpointGroupArn":"arn:group-b","EndpointGroupRegion":"us-west-2"}]}"#,
            ),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .list_endpoint_groups("listener-1", None, None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].endpoint_group_arn(), Some("arn:group-a"));
        assert_eq!(groups[1].endpoint_group_arn(), Some("arn:group-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoint_groups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ListenerArn":"listener-1","NextToken":"tok-1"}"#),
            json_response(
                200,
                r#"{"EndpointGroups":[{"EndpointGroupArn":"arn:group-b"}]}"#,
            ),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .list_endpoint_groups("listener-1", None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].endpoint_group_arn(), Some("arn:group-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoint_groups_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ListenerArn":"listener-1","MaxResults":2}"#),
            json_response(
                200,
                r#"{"EndpointGroups":[{"EndpointGroupArn":"arn:group-a"},{"EndpointGroupArn":"arn:group-b"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .list_endpoint_groups("listener-1", Some(2), None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoint_groups_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ListenerArn":"listener-1","MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"EndpointGroups":[{"EndpointGroupArn":"arn:a"},{"EndpointGroupArn":"arn:b"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"ListenerArn":"listener-1","NextToken":"tok-3","MaxResults":1}"#,
                ),
                json_response(200, r#"{"EndpointGroups":[{"EndpointGroupArn":"arn:c"}]}"#),
            ),
        ]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .list_endpoint_groups("listener-1", Some(3), None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoint_groups_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ListenerArn":"listener-1"}"#),
            json_error_response("InvalidArgumentException", "invalid argument"),
        )]);
        let client = GlobalAcceleratorClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_endpoint_groups("listener-1", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidArgumentException"));
                assert_eq!(message, "invalid argument");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
