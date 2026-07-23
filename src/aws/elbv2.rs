use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct Elbv2Client {
    inner: aws_sdk_elasticloadbalancingv2::Client,
}

impl Elbv2Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_elasticloadbalancingv2::Client::new(config),
        }
    }

    /// Lists load balancers, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeLoadBalancers` has
    /// both `page_size` and `marker`/`next_marker` (verified against pinned
    /// `aws-sdk-elasticloadbalancingv2` 1.116.0's
    /// `operation/describe_load_balancers/_describe_load_balancers_{input,output}.rs`),
    /// with no documented minimum on `page_size`, so no clamping is needed.
    pub async fn describe_load_balancers(
        &self,
        arns: Option<Vec<String>>,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_elasticloadbalancingv2::types::LoadBalancer>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut request = self
                .inner
                .describe_load_balancers()
                .set_load_balancer_arns(arns.clone())
                .set_names(names.clone());

            if let Some(l) = limit {
                request = request.page_size(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.load_balancers().iter().cloned());

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

    /// Lists target groups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same page_size/marker shape
    /// as `describe_load_balancers` above.
    pub async fn describe_target_groups(
        &self,
        arns: Option<Vec<String>>,
        load_balancer_arn: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_elasticloadbalancingv2::types::TargetGroup>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut request = self
                .inner
                .describe_target_groups()
                .set_target_group_arns(arns.clone());

            if let Some(ref lb_arn) = load_balancer_arn {
                request = request.load_balancer_arn(lb_arn);
            }
            if let Some(l) = limit {
                request = request.page_size(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.target_groups().iter().cloned());

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

    pub async fn describe_target_health(
        &self,
        target_group_arn: String,
    ) -> Result<Vec<aws_sdk_elasticloadbalancingv2::types::TargetHealthDescription>, VaporError>
    {
        let output = self
            .inner
            .describe_target_health()
            .target_group_arn(target_group_arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output.target_health_descriptions().to_vec())
    }

    /// Lists listeners for a load balancer, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`. Same
    /// page_size/marker shape as `describe_load_balancers` above.
    pub async fn describe_listeners(
        &self,
        load_balancer_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_elasticloadbalancingv2::types::Listener>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut request = self
                .inner
                .describe_listeners()
                .load_balancer_arn(&load_balancer_arn);

            if let Some(l) = limit {
                request = request.page_size(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.listeners().iter().cloned());

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

    /// Lists rules for a listener, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. Same
    /// page_size/marker shape as `describe_load_balancers` above.
    pub async fn describe_rules(
        &self,
        listener_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_elasticloadbalancingv2::types::Rule>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut request = self.inner.describe_rules().listener_arn(&listener_arn);

            if let Some(l) = limit {
                request = request.page_size(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.rules().iter().cloned());

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
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://elasticloadbalancing.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_load_balancers_happy_path_passes_arns_and_names() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLoadBalancers&Version=2015-12-01&LoadBalancerArns.member.1=lb-arn-1&Names.member.1=my-lb",
            ),
            xml_response(
                200,
                "<DescribeLoadBalancersResponse><DescribeLoadBalancersResult><LoadBalancers>\
                 <member><LoadBalancerArn>lb-arn-1</LoadBalancerArn><LoadBalancerName>my-lb</LoadBalancerName>\
                 <Type>application</Type></member>\
                 </LoadBalancers></DescribeLoadBalancersResult></DescribeLoadBalancersResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (lbs, marker) = client
            .describe_load_balancers(
                Some(vec!["lb-arn-1".to_string()]),
                Some(vec!["my-lb".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(lbs.len(), 1);
        assert_eq!(lbs[0].load_balancer_arn(), Some("lb-arn-1"));
        assert_eq!(lbs[0].load_balancer_name(), Some("my-lb"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_load_balancers_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLoadBalancers&Version=2015-12-01&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeLoadBalancersResponse><DescribeLoadBalancersResult><LoadBalancers>\
                 </LoadBalancers></DescribeLoadBalancersResult></DescribeLoadBalancersResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (lbs, marker) = client
            .describe_load_balancers(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(lbs.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_load_balancers_stops_at_limit_and_surfaces_resume_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLoadBalancers&Version=2015-12-01&PageSize=2",
            ),
            xml_response(
                200,
                "<DescribeLoadBalancersResponse><DescribeLoadBalancersResult><LoadBalancers>\
                 <member><LoadBalancerArn>a</LoadBalancerArn></member>\
                 <member><LoadBalancerArn>b</LoadBalancerArn></member>\
                 <member><LoadBalancerArn>c</LoadBalancerArn></member>\
                 </LoadBalancers><NextMarker>page2</NextMarker></DescribeLoadBalancersResult></DescribeLoadBalancersResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (lbs, marker) = client.describe_load_balancers(None, None, Some(2), None).await.unwrap();

        assert_eq!(lbs.len(), 2);
        assert_eq!(lbs[0].load_balancer_arn(), Some("a"));
        assert_eq!(lbs[1].load_balancer_arn(), Some("b"));
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_load_balancers_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeLoadBalancers&Version=2015-12-01"),
                xml_response(
                    200,
                    "<DescribeLoadBalancersResponse><DescribeLoadBalancersResult><LoadBalancers>\
                     <member><LoadBalancerArn>a</LoadBalancerArn></member>\
                     </LoadBalancers><NextMarker>p2</NextMarker></DescribeLoadBalancersResult></DescribeLoadBalancersResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeLoadBalancers&Version=2015-12-01&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeLoadBalancersResponse><DescribeLoadBalancersResult><LoadBalancers>\
                     <member><LoadBalancerArn>b</LoadBalancerArn></member>\
                     </LoadBalancers></DescribeLoadBalancersResult></DescribeLoadBalancersResponse>",
                ),
            ),
        ]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (lbs, marker) = client.describe_load_balancers(None, None, None, None).await.unwrap();

        assert_eq!(lbs.len(), 2);
        assert_eq!(lbs[0].load_balancer_arn(), Some("a"));
        assert_eq!(lbs[1].load_balancer_arn(), Some("b"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_load_balancers_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeLoadBalancers&Version=2015-12-01"),
            xml_error_response("LoadBalancerNotFound", "no such load balancer"),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let err = client.describe_load_balancers(None, None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("LoadBalancerNotFound"));
                assert_eq!(message, "no such load balancer");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_target_groups_happy_path_filters_by_load_balancer_arn() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeTargetGroups&Version=2015-12-01&LoadBalancerArn=lb-arn-1",
            ),
            xml_response(
                200,
                "<DescribeTargetGroupsResponse><DescribeTargetGroupsResult><TargetGroups>\
                 <member><TargetGroupArn>tg-arn-1</TargetGroupArn><TargetGroupName>my-tg</TargetGroupName></member>\
                 </TargetGroups></DescribeTargetGroupsResult></DescribeTargetGroupsResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (tgs, marker) = client
            .describe_target_groups(None, Some("lb-arn-1".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(tgs.len(), 1);
        assert_eq!(tgs[0].target_group_arn(), Some("tg-arn-1"));
        assert_eq!(tgs[0].target_group_name(), Some("my-tg"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_target_groups_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeTargetGroups&Version=2015-12-01&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeTargetGroupsResponse><DescribeTargetGroupsResult><TargetGroups>\
                 </TargetGroups></DescribeTargetGroupsResult></DescribeTargetGroupsResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (tgs, marker) = client
            .describe_target_groups(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(tgs.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_target_groups_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeTargetGroups&Version=2015-12-01"),
            xml_error_response("TargetGroupNotFound", "no such target group"),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let err = client.describe_target_groups(None, None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("TargetGroupNotFound"));
                assert_eq!(message, "no such target group");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_target_health_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeTargetHealth&Version=2015-12-01&TargetGroupArn=tg-arn-1",
            ),
            xml_response(
                200,
                "<DescribeTargetHealthResponse><DescribeTargetHealthResult><TargetHealthDescriptions>\
                 <member><Target><Id>i-1234</Id><Port>80</Port></Target>\
                 <TargetHealth><State>healthy</State></TargetHealth></member>\
                 </TargetHealthDescriptions></DescribeTargetHealthResult></DescribeTargetHealthResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let descriptions = client
            .describe_target_health("tg-arn-1".to_string())
            .await
            .unwrap();

        assert_eq!(descriptions.len(), 1);
        assert_eq!(descriptions[0].target().and_then(|t| t.id()), Some("i-1234"));
        assert_eq!(
            descriptions[0].target_health().and_then(|h| h.state()).map(|s| s.as_str()),
            Some("healthy")
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_target_health_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeTargetHealth&Version=2015-12-01&TargetGroupArn=tg-arn-1",
            ),
            xml_error_response("InvalidTarget", "target not in vpc"),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_target_health("tg-arn-1".to_string())
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidTarget"));
                assert_eq!(message, "target not in vpc");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_listeners_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeListeners&Version=2015-12-01&LoadBalancerArn=lb-arn-1",
            ),
            xml_response(
                200,
                "<DescribeListenersResponse><DescribeListenersResult><Listeners>\
                 <member><ListenerArn>listener-arn-1</ListenerArn><Port>443</Port></member>\
                 </Listeners></DescribeListenersResult></DescribeListenersResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (listeners, marker) = client
            .describe_listeners("lb-arn-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(listeners.len(), 1);
        assert_eq!(listeners[0].listener_arn(), Some("listener-arn-1"));
        assert_eq!(listeners[0].port(), Some(443));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_listeners_stops_at_limit_and_surfaces_resume_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeListeners&Version=2015-12-01&LoadBalancerArn=lb-arn-1&PageSize=1",
            ),
            xml_response(
                200,
                "<DescribeListenersResponse><DescribeListenersResult><Listeners>\
                 <member><ListenerArn>listener-arn-1</ListenerArn></member>\
                 <member><ListenerArn>listener-arn-2</ListenerArn></member>\
                 </Listeners><NextMarker>page2</NextMarker></DescribeListenersResult></DescribeListenersResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (listeners, marker) = client
            .describe_listeners("lb-arn-1".to_string(), Some(1), None)
            .await
            .unwrap();

        assert_eq!(listeners.len(), 1);
        assert_eq!(listeners[0].listener_arn(), Some("listener-arn-1"));
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_listeners_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeListeners&Version=2015-12-01&LoadBalancerArn=lb-arn-1",
            ),
            xml_error_response("ListenerNotFound", "no such listener"),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_listeners("lb-arn-1".to_string(), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("ListenerNotFound"));
                assert_eq!(message, "no such listener");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_rules_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeRules&Version=2015-12-01&ListenerArn=listener-arn-1",
            ),
            xml_response(
                200,
                "<DescribeRulesResponse><DescribeRulesResult><Rules>\
                 <member><RuleArn>rule-arn-1</RuleArn><Priority>1</Priority><IsDefault>false</IsDefault></member>\
                 </Rules></DescribeRulesResult></DescribeRulesResponse>",
            ),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (rules, marker) = client
            .describe_rules("listener-arn-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].rule_arn(), Some("rule-arn-1"));
        assert_eq!(rules[0].priority(), Some("1"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_rules_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeRules&Version=2015-12-01&ListenerArn=listener-arn-1",
                ),
                xml_response(
                    200,
                    "<DescribeRulesResponse><DescribeRulesResult><Rules>\
                     <member><RuleArn>rule-arn-1</RuleArn></member>\
                     </Rules><NextMarker>p2</NextMarker></DescribeRulesResult></DescribeRulesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeRules&Version=2015-12-01&ListenerArn=listener-arn-1&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeRulesResponse><DescribeRulesResult><Rules>\
                     <member><RuleArn>rule-arn-2</RuleArn></member>\
                     </Rules></DescribeRulesResult></DescribeRulesResponse>",
                ),
            ),
        ]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let (rules, marker) = client
            .describe_rules("listener-arn-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].rule_arn(), Some("rule-arn-1"));
        assert_eq!(rules[1].rule_arn(), Some("rule-arn-2"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_rules_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeRules&Version=2015-12-01&ListenerArn=listener-arn-1",
            ),
            xml_error_response("RuleNotFound", "no such rule"),
        )]);
        let client = Elbv2Client::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_rules("listener-arn-1".to_string(), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("RuleNotFound"));
                assert_eq!(message, "no such rule");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
