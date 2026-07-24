use aws_config::SdkConfig;
use aws_sdk_cloudformation::types::{Export, Stack, StackResourceSummary};
use aws_smithy_types::error::metadata::ProvideErrorMetadata;

use crate::aws::pagination::apply_limit;
use crate::error::VaporError;

pub struct CloudFormationClient {
    inner: aws_sdk_cloudformation::Client,
}

impl CloudFormationClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cloudformation::Client::new(config),
        }
    }

    /// Returns full details for a single named stack, or `None` if it doesn't exist.
    pub async fn describe_stack(&self, stack_name: &str) -> Result<Option<Stack>, VaporError> {
        match self
            .inner
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await
        {
            Ok(output) => Ok(output.stacks.unwrap_or_default().into_iter().next()),
            Err(e) => {
                let not_found = e.message().unwrap_or_default().contains("does not exist");
                if not_found {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }

    /// Returns full details for all non-DELETE_COMPLETE stacks, optionally filtered client-side
    /// by `status_filter` (`DescribeStacks` has no server-side status parameter — that's
    /// `ListStacks`, which only returns lighter `StackSummary`s lacking parameters/outputs/tags),
    /// capped at `limit` results (default unlimited) and resumed from `next_token`.
    /// `DescribeStacks` has no `max_results`-equivalent input field (only a bare `next_token`,
    /// confirmed against pinned `aws-sdk-cloudformation` 1.116.0's
    /// `operation/describe_stacks/_describe_stacks_input.rs`), so `limit` can only be enforced
    /// via client-side `apply_limit` truncation — same caveat class as `xray.rs::get_groups`.
    pub async fn describe_all_stacks(
        &self,
        status_filter: &[String],
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Stack>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_stacks();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            let page = output.stacks.unwrap_or_default();
            if status_filter.is_empty() {
                items.extend(page);
            } else {
                items.extend(page.into_iter().filter(|s| {
                    s.stack_status()
                        .map(|st| status_filter.contains(&st.as_str().to_string()))
                        .unwrap_or(false)
                }));
            }

            token = output.next_token;
            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    /// Returns all resources for a given stack, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListStackResources` has no
    /// `max_results`-equivalent input field (same caveat class as `describe_all_stacks` above).
    pub async fn list_stack_resources(
        &self,
        stack_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<StackResourceSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_stack_resources().stack_name(stack_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            items.extend(output.stack_resource_summaries.unwrap_or_default());
            token = output.next_token;
            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    /// Returns all CloudFormation exports (cross-stack references), optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`. `ListExports` has no
    /// `max_results`-equivalent input field (same caveat class as `describe_all_stacks` above).
    pub async fn list_exports(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Export>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_exports();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            items.extend(output.exports.unwrap_or_default());
            token = output.next_token;
            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient,
    };
    use aws_sdk_cloudformation::types::StackStatus;

    // Query/ec2Query protocol (verified against pinned `aws-sdk-cloudformation`
    // 1.116.0's `protocol_serde::shape_*_input` codegen — `QueryWriter::new(&mut
    // out, "<Op>", "2010-05-15")`) — same family as autoscaling.rs, so
    // `StaticReplayClient` does exact byte-for-byte request-body comparison
    // (`application/x-www-form-urlencoded`, not JSON), and field order is fixed
    // per-op by that op's own codegen, not call order.
    const ENDPOINT: &str = "https://cloudformation.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_stack_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeStacks&Version=2010-05-15&StackName=my-stack"),
            xml_response(
                200,
                "<DescribeStacksResponse><DescribeStacksResult><Stacks><member>\
                 <StackId>arn:aws:cloudformation:us-east-1:111111111111:stack/my-stack/abc</StackId>\
                 <StackName>my-stack</StackName><StackStatus>CREATE_COMPLETE</StackStatus>\
                 <CreationTime>2024-01-15T10:30:00Z</CreationTime>\
                 </member></Stacks></DescribeStacksResult></DescribeStacksResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let stack = client.describe_stack("my-stack").await.unwrap().unwrap();

        assert_eq!(stack.stack_name(), Some("my-stack"));
        assert_eq!(stack.stack_status(), Some(&StackStatus::CreateComplete));
        assert!(stack.creation_time().is_some());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_stack_returns_none_when_stack_does_not_exist() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeStacks&Version=2010-05-15&StackName=missing-stack",
            ),
            xml_error_response(
                "ValidationError",
                "Stack with id missing-stack does not exist",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let stack = client.describe_stack("missing-stack").await.unwrap();

        assert!(stack.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_stack_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeStacks&Version=2010-05-15&StackName=my-stack",
            ),
            xml_error_response("AccessDenied", "not authorized"),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_stack("my-stack").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDenied".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_all_stacks_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeStacks&Version=2010-05-15"),
            xml_response(
                200,
                "<DescribeStacksResponse><DescribeStacksResult><Stacks>\
                 <member><StackName>stack-1</StackName><StackStatus>CREATE_COMPLETE</StackStatus></member>\
                 <member><StackName>stack-2</StackName><StackStatus>UPDATE_COMPLETE</StackStatus></member>\
                 </Stacks></DescribeStacksResult></DescribeStacksResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (stacks, token) = client.describe_all_stacks(&[], None, None).await.unwrap();

        assert_eq!(stacks.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_all_stacks_applies_client_side_status_filter() {
        // `DescribeStacks` has no server-side status parameter (that's
        // `ListStacks`), so the request is identical to the no-filter case —
        // filtering happens entirely client-side after the page is fetched.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeStacks&Version=2010-05-15"),
            xml_response(
                200,
                "<DescribeStacksResponse><DescribeStacksResult><Stacks>\
                 <member><StackName>stack-1</StackName><StackStatus>CREATE_COMPLETE</StackStatus></member>\
                 <member><StackName>stack-2</StackName><StackStatus>DELETE_COMPLETE</StackStatus></member>\
                 </Stacks></DescribeStacksResult></DescribeStacksResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (stacks, _token) = client
            .describe_all_stacks(&["CREATE_COMPLETE".to_string()], None, None)
            .await
            .unwrap();

        assert_eq!(stacks.len(), 1);
        assert_eq!(stacks[0].stack_name(), Some("stack-1"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_all_stacks_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeStacks&Version=2010-05-15&NextToken=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeStacksResponse><DescribeStacksResult><Stacks>\
                 </Stacks></DescribeStacksResult></DescribeStacksResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (stacks, token) = client
            .describe_all_stacks(&[], None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(stacks.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_all_stacks_stops_at_limit_and_returns_resume_token() {
        // `DescribeStacks` has no `max_results`-equivalent input field, so
        // `limit` is enforced entirely via client-side `apply_limit`
        // truncation — the request never carries the limit, unlike
        // autoscaling's `MaxRecords`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeStacks&Version=2010-05-15"),
            xml_response(
                200,
                "<DescribeStacksResponse><DescribeStacksResult><Stacks>\
                 <member><StackName>stack-1</StackName></member>\
                 <member><StackName>stack-2</StackName></member>\
                 <member><StackName>stack-3</StackName></member>\
                 </Stacks><NextToken>page2</NextToken></DescribeStacksResult></DescribeStacksResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (stacks, token) = client
            .describe_all_stacks(&[], Some(2), None)
            .await
            .unwrap();

        assert_eq!(stacks.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_all_stacks_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeStacks&Version=2010-05-15"),
                xml_response(
                    200,
                    "<DescribeStacksResponse><DescribeStacksResult><Stacks>\
                     <member><StackName>stack-1</StackName></member>\
                     </Stacks><NextToken>p2</NextToken></DescribeStacksResult></DescribeStacksResponse>",
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeStacks&Version=2010-05-15&NextToken=p2"),
                xml_response(
                    200,
                    "<DescribeStacksResponse><DescribeStacksResult><Stacks>\
                     <member><StackName>stack-2</StackName></member>\
                     </Stacks></DescribeStacksResult></DescribeStacksResponse>",
                ),
            ),
        ]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (stacks, token) = client
            .describe_all_stacks(&[], Some(10), None)
            .await
            .unwrap();

        assert_eq!(stacks.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_all_stacks_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeStacks&Version=2010-05-15"),
            xml_error_response("ValidationError", "malformed request"),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_all_stacks(&[], None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationError".to_string()));
                assert_eq!(message, "malformed request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stack_resources_happy_path() {
        let http_client =
            StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListStackResources&Version=2010-05-15&StackName=my-stack"),
            xml_response(
                200,
                "<ListStackResourcesResponse><ListStackResourcesResult><StackResourceSummaries>\
                 <member><LogicalResourceId>MyBucket</LogicalResourceId>\
                 <PhysicalResourceId>my-bucket-abc123</PhysicalResourceId>\
                 <ResourceType>AWS::S3::Bucket</ResourceType>\
                 <ResourceStatus>CREATE_COMPLETE</ResourceStatus></member>\
                 </StackResourceSummaries></ListStackResourcesResult></ListStackResourcesResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client
            .list_stack_resources("my-stack", None, None)
            .await
            .unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].logical_resource_id(), Some("MyBucket"));
        assert_eq!(
            resources[0].physical_resource_id(),
            Some("my-bucket-abc123")
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stack_resources_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListStackResources&Version=2010-05-15&StackName=my-stack"),
            xml_response(
                200,
                "<ListStackResourcesResponse><ListStackResourcesResult><StackResourceSummaries>\
                 <member><LogicalResourceId>Res1</LogicalResourceId></member>\
                 <member><LogicalResourceId>Res2</LogicalResourceId></member>\
                 <member><LogicalResourceId>Res3</LogicalResourceId></member>\
                 </StackResourceSummaries><NextToken>next-res</NextToken></ListStackResourcesResult>\
                 </ListStackResourcesResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client
            .list_stack_resources("my-stack", Some(2), None)
            .await
            .unwrap();

        assert_eq!(resources.len(), 2);
        assert_eq!(token, Some("next-res".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_exports_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListExports&Version=2010-05-15"),
            xml_response(
                200,
                "<ListExportsResponse><ListExportsResult><Exports>\
                 <member><ExportingStackId>arn:aws:cloudformation:us-east-1:111111111111:stack/my-stack/abc</ExportingStackId>\
                 <Name>my-export</Name><Value>exported-value</Value></member>\
                 </Exports></ListExportsResult></ListExportsResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (exports, token) = client.list_exports(None, None).await.unwrap();

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name(), Some("my-export"));
        assert_eq!(exports[0].value(), Some("exported-value"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_exports_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListExports&Version=2010-05-15"),
            xml_response(
                200,
                "<ListExportsResponse><ListExportsResult><Exports>\
                 <member><Name>export-1</Name></member>\
                 <member><Name>export-2</Name></member>\
                 <member><Name>export-3</Name></member>\
                 </Exports><NextToken>next-exp</NextToken></ListExportsResult></ListExportsResponse>",
            ),
        )]);
        let client = CloudFormationClient::new(&sdk_config(http_client.clone()));

        let (exports, token) = client.list_exports(Some(2), None).await.unwrap();

        assert_eq!(exports.len(), 2);
        assert_eq!(token, Some("next-exp".to_string()));
        http_client.relaxed_requests_match();
    }
}
