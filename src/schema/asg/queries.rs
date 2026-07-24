use async_graphql::{Context, Object, Result};

use crate::aws::autoscaling::AutoscalingClient;
use crate::schema::asg::types::{AutoScalingGroup, ScalingActivity};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AsgQuery;

#[Object]
impl AsgQuery {
    async fn auto_scaling_groups(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AutoScalingGroup>> {
        let client = ctx.data::<AutoscalingClient>()?;
        let (results, next_token) = client
            .describe_auto_scaling_groups(names, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(AutoScalingGroup::from).collect(),
            next_token,
        })
    }

    async fn scaling_activities(
        &self,
        ctx: &Context<'_>,
        auto_scaling_group_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ScalingActivity>> {
        let client = ctx.data::<AutoscalingClient>()?;
        let (results, next_token) = client
            .describe_scaling_activities(auto_scaling_group_name, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(ScalingActivity::from).collect(),
            next_token,
        })
    }
}

// Both resolvers are 1:1 passthroughs to a single already-tested
// `AutoscalingClient` method (see `src/aws/autoscaling.rs`'s own test
// module for the pagination/error-mapping behavior) — only light smoke
// tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::autoscaling::AutoscalingClient;
    use crate::aws::test_util::{
        request, sdk_config, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::AsgQuery;

    const ENDPOINT: &str = "https://autoscaling.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn auto_scaling_groups_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeAutoScalingGroups&Version=2011-01-01&MaxRecords=1"),
            xml_response(
                200,
                "<DescribeAutoScalingGroupsResponse><DescribeAutoScalingGroupsResult><AutoScalingGroups>\
                 <member><AutoScalingGroupName>my-asg</AutoScalingGroupName><MinSize>1</MinSize>\
                 <MaxSize>5</MaxSize><DesiredCapacity>2</DesiredCapacity></member>\
                 </AutoScalingGroups><NextToken>next-page</NextToken></DescribeAutoScalingGroupsResult>\
                 </DescribeAutoScalingGroupsResponse>",
            ),
        )]);
        let schema = build_query_schema(AsgQuery)
            .data(AutoscalingClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ autoScalingGroups(limit: 1) { items { name minSize maxSize desiredCapacity } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["autoScalingGroups"]["items"];
        assert_eq!(items[0]["name"], "my-asg");
        assert_eq!(items[0]["minSize"], 1);
        assert_eq!(items[0]["maxSize"], 5);
        assert_eq!(items[0]["desiredCapacity"], 2);
        assert_eq!(json["autoScalingGroups"]["nextToken"], "next-page");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn scaling_activities_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeScalingActivities&Version=2011-01-01&MaxRecords=1"),
            xml_response(
                200,
                "<DescribeScalingActivitiesResponse><DescribeScalingActivitiesResult><Activities>\
                 <member><ActivityId>activity-1</ActivityId><AutoScalingGroupName>my-asg</AutoScalingGroupName>\
                 <StatusCode>Successful</StatusCode></member></Activities>\
                 <NextToken>act-page2</NextToken></DescribeScalingActivitiesResult>\
                 </DescribeScalingActivitiesResponse>",
            ),
        )]);
        let schema = build_query_schema(AsgQuery)
            .data(AutoscalingClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ scalingActivities(limit: 1) { items { activityId autoScalingGroupName statusCode } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["scalingActivities"]["items"];
        assert_eq!(items[0]["activityId"], "activity-1");
        assert_eq!(items[0]["autoScalingGroupName"], "my-asg");
        assert_eq!(items[0]["statusCode"], "SUCCESSFUL");
        assert_eq!(json["scalingActivities"]["nextToken"], "act-page2");
        http_client.relaxed_requests_match();
    }
}
