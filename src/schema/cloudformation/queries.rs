use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::cloudformation::CloudFormationClient;
use crate::schema::cloudformation::types::{CfnExport, CfnStack, CfnStackResource};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CloudFormationQuery;

#[Object]
impl CloudFormationQuery {
    /// List CloudFormation stacks. Optionally filter by names and/or status strings.
    /// Without arguments returns all non-DELETE_COMPLETE stacks.
    /// `limit` caps the number of stacks returned (default unlimited).
    /// When `names` is given, each name is fetched independently (a targeted lookup, not a
    /// single AWS-side stream) so the result is not resumable (`next_token` is always `None`);
    /// otherwise `next_token` resumes the underlying `DescribeStacks` pagination.
    async fn cfn_stacks(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
        status_filter: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CfnStack>> {
        let client = ctx.data::<CloudFormationClient>()?;
        let statuses = status_filter.unwrap_or_default();

        match names {
            Some(ref ns) if !ns.is_empty() => {
                let futs = ns.iter().map(|n| client.describe_stack(n.as_str()));
                let results = join_all(futs).await;
                let mut raw_stacks = results
                    .into_iter()
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                if !statuses.is_empty() {
                    raw_stacks.retain(|s| {
                        s.stack_status()
                            .map(|st| statuses.contains(&st.as_str().to_string()))
                            .unwrap_or(false)
                    });
                }

                let mut items: Vec<CfnStack> = raw_stacks.iter().map(CfnStack::from).collect();
                if let Some(limit) = limit {
                    items.truncate(limit.max(0) as usize);
                }
                Ok(Page {
                    items,
                    next_token: None,
                })
            }
            _ => {
                let (raw_stacks, next_token) = client
                    .describe_all_stacks(&statuses, limit, next_token)
                    .await?;
                Ok(Page {
                    items: raw_stacks.iter().map(CfnStack::from).collect(),
                    next_token,
                })
            }
        }
    }

    /// List all resources in a CloudFormation stack. `limit` caps the number of resources
    /// returned (default unlimited) and resumed from `next_token`.
    async fn cfn_stack_resources(
        &self,
        ctx: &Context<'_>,
        stack_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CfnStackResource>> {
        let client = ctx.data::<CloudFormationClient>()?;
        let (resources, next_token) = client
            .list_stack_resources(&stack_name, limit, next_token)
            .await?;
        Ok(Page {
            items: resources.iter().map(CfnStackResource::from).collect(),
            next_token,
        })
    }

    /// List all CloudFormation exports (cross-stack references) in the account/region.
    /// `limit` caps the number of exports returned (default unlimited) and resumed from
    /// `next_token`.
    async fn cfn_exports(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CfnExport>> {
        let client = ctx.data::<CloudFormationClient>()?;
        let (exports, next_token) = client.list_exports(limit, next_token).await?;
        Ok(Page {
            items: exports.iter().map(CfnExport::from).collect(),
            next_token,
        })
    }
}

// `cfn_stacks` has real resolver-level logic beyond a bare passthrough (the
// `names`-given-vs-discovery branch: per-name fan-out via `join_all`, plus
// client-side status filtering and `limit` truncation independent of
// `describe_all_stacks`'s own filtering/truncation), so it gets bespoke
// coverage per the resolver-layer sweep's stated scope; `cfn_stack_resources`
// and `cfn_exports` are bare 1:1 passthroughs to a single already-tested
// `CloudFormationClient` method each (see `src/aws/cloudformation.rs`'s own
// test module for pagination/limit/error-mapping) and get only a light smoke
// test.
#[cfg(test)]
mod tests {
    use crate::aws::cloudformation::CloudFormationClient;
    use crate::aws::test_util::{
        request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CloudFormationQuery;

    const ENDPOINT: &str = "https://cloudformation.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn cfn_stacks_with_names_fans_out_and_flattens_missing_stacks() {
        // Fan-out order under `StaticReplayClient` matches `names`' iteration
        // order (acm.rs precedent — nothing here truly suspends the
        // executor), so replay events must mirror that order exactly.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeStacks&Version=2010-05-15&StackName=stack-1",
                ),
                xml_response(
                    200,
                    "<DescribeStacksResponse><DescribeStacksResult><Stacks><member>\
                     <StackName>stack-1</StackName><StackStatus>CREATE_COMPLETE</StackStatus>\
                     </member></Stacks></DescribeStacksResult></DescribeStacksResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeStacks&Version=2010-05-15&StackName=missing",
                ),
                xml_error_response("ValidationError", "Stack with id missing does not exist"),
            ),
        ]);
        let schema = build_query_schema(CloudFormationQuery)
            .data(CloudFormationClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cfnStacks(names: ["stack-1", "missing"]) { items { stackName } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["cfnStacks"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["stackName"], "stack-1");
        // The names path is a targeted lookup, not a single AWS-side stream
        // — never resumable, regardless of how many names matched.
        assert!(json["cfnStacks"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cfn_stacks_with_names_applies_status_filter_then_limit_client_side() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeStacks&Version=2010-05-15&StackName=s1",
                ),
                xml_response(
                    200,
                    "<DescribeStacksResponse><DescribeStacksResult><Stacks><member>\
                     <StackName>s1</StackName><StackStatus>CREATE_COMPLETE</StackStatus>\
                     </member></Stacks></DescribeStacksResult></DescribeStacksResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeStacks&Version=2010-05-15&StackName=s2",
                ),
                xml_response(
                    200,
                    "<DescribeStacksResponse><DescribeStacksResult><Stacks><member>\
                     <StackName>s2</StackName><StackStatus>UPDATE_COMPLETE</StackStatus>\
                     </member></Stacks></DescribeStacksResult></DescribeStacksResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeStacks&Version=2010-05-15&StackName=s3",
                ),
                xml_response(
                    200,
                    "<DescribeStacksResponse><DescribeStacksResult><Stacks><member>\
                     <StackName>s3</StackName><StackStatus>CREATE_COMPLETE</StackStatus>\
                     </member></Stacks></DescribeStacksResult></DescribeStacksResponse>",
                ),
            ),
        ]);
        let schema = build_query_schema(CloudFormationQuery)
            .data(CloudFormationClient::new(&sdk_config(http_client.clone())))
            .finish();

        // s2 is filtered out by status; of the remaining [s1, s3], `limit: 1`
        // then truncates to just s1 — proving both the resolver's own
        // client-side status filter and its own `limit` truncation run on
        // the fan-out results, independent of `describe_all_stacks`.
        let res = schema
            .execute(
                r#"{ cfnStacks(names: ["s1", "s2", "s3"], statusFilter: ["CREATE_COMPLETE"], limit: 1) { items { stackName } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["cfnStacks"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["stackName"], "s1");
        assert!(json["cfnStacks"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cfn_stacks_with_names_propagates_non_not_found_describe_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeStacks&Version=2010-05-15&StackName=s1",
                ),
                xml_response(
                    200,
                    "<DescribeStacksResponse><DescribeStacksResult><Stacks><member>\
                     <StackName>s1</StackName></member></Stacks></DescribeStacksResult>\
                     </DescribeStacksResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeStacks&Version=2010-05-15&StackName=bad",
                ),
                xml_error_response("AccessDenied", "not authorized"),
            ),
        ]);
        let schema = build_query_schema(CloudFormationQuery)
            .data(CloudFormationClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ cfnStacks(names: ["s1", "bad"]) { items { stackName } } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("not authorized"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cfn_stacks_without_names_delegates_to_discovery_call() {
        // No `names` given → the `_` branch, a bare passthrough to
        // `describe_all_stacks` (itself fully tested in `src/aws/
        // cloudformation.rs`) — this test only proves the resolver forwards
        // `next_token` through and maps the discovery call's own `next_token`
        // back out (unlike the names path, which always returns `None`).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeStacks&Version=2010-05-15&NextToken=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeStacksResponse><DescribeStacksResult><Stacks><member>\
                 <StackName>stack-1</StackName><StackStatus>CREATE_COMPLETE</StackStatus>\
                 </member></Stacks><NextToken>page2</NextToken></DescribeStacksResult>\
                 </DescribeStacksResponse>",
            ),
        )]);
        let schema = build_query_schema(CloudFormationQuery)
            .data(CloudFormationClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cfnStacks(nextToken: "cursor-a", limit: 1) { items { stackName } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["cfnStacks"]["items"][0]["stackName"], "stack-1");
        assert_eq!(json["cfnStacks"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cfn_stack_resources_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListStackResources&Version=2010-05-15&StackName=my-stack",
            ),
            xml_response(
                200,
                "<ListStackResourcesResponse><ListStackResourcesResult><StackResourceSummaries>\
                 <member><LogicalResourceId>MyBucket</LogicalResourceId>\
                 <ResourceType>AWS::S3::Bucket</ResourceType></member>\
                 </StackResourceSummaries><NextToken>next-res</NextToken></ListStackResourcesResult>\
                 </ListStackResourcesResponse>",
            ),
        )]);
        let schema = build_query_schema(CloudFormationQuery)
            .data(CloudFormationClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cfnStackResources(stackName: "my-stack", limit: 1) { items { logicalResourceId resourceType } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["cfnStackResources"]["items"][0]["logicalResourceId"],
            "MyBucket"
        );
        assert_eq!(
            json["cfnStackResources"]["items"][0]["resourceType"],
            "AWS::S3::Bucket"
        );
        assert_eq!(json["cfnStackResources"]["nextToken"], "next-res");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cfn_exports_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListExports&Version=2010-05-15"),
            xml_response(
                200,
                "<ListExportsResponse><ListExportsResult><Exports><member>\
                 <Name>my-export</Name><Value>exported-value</Value></member>\
                 </Exports><NextToken>next-exp</NextToken></ListExportsResult></ListExportsResponse>",
            ),
        )]);
        let schema = build_query_schema(CloudFormationQuery)
            .data(CloudFormationClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ cfnExports(limit: 1) { items { name value } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["cfnExports"]["items"][0]["name"], "my-export");
        assert_eq!(json["cfnExports"]["items"][0]["value"], "exported-value");
        assert_eq!(json["cfnExports"]["nextToken"], "next-exp");
        http_client.relaxed_requests_match();
    }
}
