#[cfg(feature = "autoscaling")]
use aws_config::SdkConfig;

#[cfg(feature = "autoscaling")]
use crate::error::VaporError;

#[cfg(feature = "autoscaling")]
pub struct AutoscalingClient {
    inner: aws_sdk_autoscaling::Client,
}

impl AutoscalingClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_autoscaling::Client::new(config),
        }
    }

    /// Describes Auto Scaling groups, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `DescribeAutoScalingGroups`
    /// has both `max_records` and `next_token` (verified against pinned
    /// `aws-sdk-autoscaling` 1.120.0's
    /// `operation/describe_auto_scaling_groups/_describe_auto_scaling_groups_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself,
    /// matching `kinesis.rs`'s `list_streams` pattern.
    pub async fn describe_auto_scaling_groups(
        &self,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_autoscaling::types::AutoScalingGroup>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self.inner.describe_auto_scaling_groups();
            if let Some(ref ns) = names {
                if !ns.is_empty() {
                    request = request.set_auto_scaling_group_names(Some(ns.clone()));
                }
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_records(l - items.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;

            items.extend(output.auto_scaling_groups.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Describes Auto Scaling activities, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `DescribeScalingActivities` has both `max_records` and `next_token`
    /// (verified against pinned `aws-sdk-autoscaling` 1.120.0's
    /// `operation/describe_scaling_activities/_describe_scaling_activities_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself,
    /// matching `kinesis.rs`'s `list_streams` pattern.
    pub async fn describe_scaling_activities(
        &self,
        group_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_autoscaling::types::Activity>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self.inner.describe_scaling_activities();
            if let Some(ref name) = group_name {
                request = request.auto_scaling_group_name(name.clone());
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_records(l - items.len() as i32);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;

            items.extend(output.activities.unwrap_or_default());
            token = output.next_token;

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
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://autoscaling.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_auto_scaling_groups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeAutoScalingGroups&Version=2011-01-01&AutoScalingGroupNames.member.1=my-asg",
            ),
            xml_response(
                200,
                "<DescribeAutoScalingGroupsResponse><DescribeAutoScalingGroupsResult><AutoScalingGroups>\
                 <member><AutoScalingGroupName>my-asg</AutoScalingGroupName><MinSize>1</MinSize>\
                 <MaxSize>5</MaxSize><DesiredCapacity>2</DesiredCapacity></member>\
                 </AutoScalingGroups></DescribeAutoScalingGroupsResult></DescribeAutoScalingGroupsResponse>",
            ),
        )]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_auto_scaling_groups(Some(vec!["my-asg".to_string()]), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].auto_scaling_group_name(), Some("my-asg"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_auto_scaling_groups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeAutoScalingGroups&Version=2011-01-01&NextToken=resume-tok",
            ),
            xml_response(
                200,
                "<DescribeAutoScalingGroupsResponse><DescribeAutoScalingGroupsResult><AutoScalingGroups>\
                 <member><AutoScalingGroupName>asg-2</AutoScalingGroupName></member>\
                 </AutoScalingGroups></DescribeAutoScalingGroupsResult></DescribeAutoScalingGroupsResponse>",
            ),
        )]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_auto_scaling_groups(None, None, Some("resume-tok".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_auto_scaling_groups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeAutoScalingGroups&Version=2011-01-01&MaxRecords=1"),
            xml_response(
                200,
                "<DescribeAutoScalingGroupsResponse><DescribeAutoScalingGroupsResult><AutoScalingGroups>\
                 <member><AutoScalingGroupName>asg-1</AutoScalingGroupName></member>\
                 </AutoScalingGroups><NextToken>next-page</NextToken></DescribeAutoScalingGroupsResult>\
                 </DescribeAutoScalingGroupsResponse>",
            ),
        )]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_auto_scaling_groups(None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("next-page".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_auto_scaling_groups_pages_through_until_exhausted_when_limit_not_reached() {
        // Also verifies field order: `NextToken` is written *before* `MaxRecords`
        // in this op's `QueryWriter` prefix order (opposite of
        // `describe_scaling_activities`, verified below) — order follows the
        // SDK's generated `ser_*_input` codegen, not call order in this file.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeAutoScalingGroups&Version=2011-01-01&MaxRecords=10"),
                xml_response(
                    200,
                    "<DescribeAutoScalingGroupsResponse><DescribeAutoScalingGroupsResult><AutoScalingGroups>\
                     <member><AutoScalingGroupName>asg-1</AutoScalingGroupName></member>\
                     <member><AutoScalingGroupName>asg-2</AutoScalingGroupName></member>\
                     <member><AutoScalingGroupName>asg-3</AutoScalingGroupName></member>\
                     </AutoScalingGroups><NextToken>page2</NextToken></DescribeAutoScalingGroupsResult>\
                     </DescribeAutoScalingGroupsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeAutoScalingGroups&Version=2011-01-01&NextToken=page2&MaxRecords=7",
                ),
                xml_response(
                    200,
                    "<DescribeAutoScalingGroupsResponse><DescribeAutoScalingGroupsResult><AutoScalingGroups>\
                     <member><AutoScalingGroupName>asg-4</AutoScalingGroupName></member>\
                     <member><AutoScalingGroupName>asg-5</AutoScalingGroupName></member>\
                     </AutoScalingGroups></DescribeAutoScalingGroupsResult></DescribeAutoScalingGroupsResponse>",
                ),
            ),
        ]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_auto_scaling_groups(None, Some(10), None).await.unwrap();

        assert_eq!(items.len(), 5);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_auto_scaling_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeAutoScalingGroups&Version=2011-01-01"),
            xml_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_auto_scaling_groups(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_scaling_activities_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeScalingActivities&Version=2011-01-01&AutoScalingGroupName=my-asg",
            ),
            xml_response(
                200,
                "<DescribeScalingActivitiesResponse><DescribeScalingActivitiesResult><Activities>\
                 <member><ActivityId>activity-1</ActivityId><AutoScalingGroupName>my-asg</AutoScalingGroupName>\
                 <StatusCode>Successful</StatusCode></member></Activities></DescribeScalingActivitiesResult>\
                 </DescribeScalingActivitiesResponse>",
            ),
        )]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_scaling_activities(Some("my-asg".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].activity_id(), Some("activity-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_scaling_activities_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeScalingActivities&Version=2011-01-01&MaxRecords=1"),
            xml_response(
                200,
                "<DescribeScalingActivitiesResponse><DescribeScalingActivitiesResult><Activities>\
                 <member><ActivityId>activity-1</ActivityId></member></Activities>\
                 <NextToken>act-page2</NextToken></DescribeScalingActivitiesResult>\
                 </DescribeScalingActivitiesResponse>",
            ),
        )]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_scaling_activities(None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("act-page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_scaling_activities_writes_max_records_before_next_token() {
        // File-specific gotcha: unlike `describe_auto_scaling_groups` (NextToken
        // before MaxRecords), this op's `ser_describe_scaling_activities_input_
        // input_input` codegen writes MaxRecords *before* NextToken — confirms
        // "field order follows each op's own codegen, not a file-wide convention".
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeScalingActivities&Version=2011-01-01&MaxRecords=3&NextToken=resume-tok",
            ),
            xml_response(
                200,
                "<DescribeScalingActivitiesResponse><DescribeScalingActivitiesResult><Activities>\
                 <member><ActivityId>activity-9</ActivityId></member></Activities>\
                 </DescribeScalingActivitiesResult></DescribeScalingActivitiesResponse>",
            ),
        )]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_scaling_activities(None, Some(3), Some("resume-tok".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_scaling_activities_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeScalingActivities&Version=2011-01-01"),
            xml_error_response("ResourceContention", "concurrent update in progress"),
        )]);
        let client = AutoscalingClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_scaling_activities(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceContention".to_string()));
                assert_eq!(message, "concurrent update in progress");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

