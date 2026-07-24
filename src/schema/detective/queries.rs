use async_graphql::{Context, Object, Result};

use crate::aws::detective::DetectiveClient;
use crate::schema::detective::types::{
    DetectiveDatasourcePackage, DetectiveGraph, DetectiveMember,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct DetectiveQuery;

#[Object]
impl DetectiveQuery {
    /// List Detective behavior graphs the account administers. `limit` caps
    /// the total number of results across pages (default: unlimited);
    /// `next_token` resumes from a previous page.
    async fn detective_graphs(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DetectiveGraph>> {
        let client = ctx.data::<DetectiveClient>()?;
        let (graphs, next_token) = client.list_graphs(limit, next_token).await?;
        Ok(Page {
            items: graphs.into_iter().map(DetectiveGraph::from).collect(),
            next_token,
        })
    }

    /// List member accounts of a Detective behavior graph. `limit` caps the
    /// total number of results across pages (default: unlimited);
    /// `next_token` resumes from a previous page.
    async fn detective_members(
        &self,
        ctx: &Context<'_>,
        graph_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DetectiveMember>> {
        let client = ctx.data::<DetectiveClient>()?;
        let (members, next_token) = client.list_members(graph_arn, limit, next_token).await?;
        Ok(Page {
            items: members.into_iter().map(DetectiveMember::from).collect(),
            next_token,
        })
    }

    /// List data source packages enabled for a Detective behavior graph.
    /// `limit` caps the total number of results across pages (default:
    /// unlimited); `next_token` resumes from a previous page.
    async fn detective_datasource_packages(
        &self,
        ctx: &Context<'_>,
        graph_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DetectiveDatasourcePackage>> {
        let client = ctx.data::<DetectiveClient>()?;
        let (packages, next_token) = client
            .list_datasource_packages(graph_arn, limit, next_token)
            .await?;
        Ok(Page {
            items: packages
                .into_iter()
                .map(DetectiveDatasourcePackage::from)
                .collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::detective::DetectiveClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::DetectiveQuery;

    const LIST_GRAPHS: &str = "https://api.detective.us-east-1.amazonaws.com/graphs/list";
    const LIST_MEMBERS: &str = "https://api.detective.us-east-1.amazonaws.com/graph/members/list";
    const LIST_DATASOURCE_PACKAGES: &str =
        "https://api.detective.us-east-1.amazonaws.com/graph/datasources/list";

    #[tokio::test]
    async fn detective_graphs_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_GRAPHS, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"GraphList":[{"Arn":"arn:aws:detective:us-east-1:1:graph:abc","CreatedTime":"2023-11-14T22:13:20.000Z"}],"NextToken":"cursor-a"}"#,
            ),
        )]);
        let schema = build_query_schema(DetectiveQuery)
            .data(DetectiveClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ detectiveGraphs(limit: 1) { items { arn createdTime } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["detectiveGraphs"]["items"];
        assert_eq!(items[0]["arn"], "arn:aws:detective:us-east-1:1:graph:abc");
        assert_eq!(items[0]["createdTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(json["detectiveGraphs"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn detective_members_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LIST_MEMBERS, r#"{"GraphArn":"arn:graph:1","MaxResults":1}"#),
            json_response(
                200,
                r#"{"MemberDetails":[{"AccountId":"111111111111","GraphArn":"arn:graph:1","EmailAddress":"user@example.com","Status":"ENABLED","InvitedTime":"2023-11-14T22:13:20.000Z","UpdatedTime":"2023-11-14T22:15:00.000Z","AdministratorId":"222222222222"}],"NextToken":"cursor-b"}"#,
            ),
        )]);
        let schema = build_query_schema(DetectiveQuery)
            .data(DetectiveClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ detectiveMembers(graphArn: "arn:graph:1", limit: 1) { items { accountId graphArn emailAddress status invitedTime updatedTime administratorId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["detectiveMembers"]["items"];
        assert_eq!(items[0]["accountId"], "111111111111");
        assert_eq!(items[0]["graphArn"], "arn:graph:1");
        assert_eq!(items[0]["emailAddress"], "user@example.com");
        assert_eq!(items[0]["status"], "ENABLED");
        assert_eq!(items[0]["invitedTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["updatedTime"], "2023-11-14T22:15:00+00:00");
        assert_eq!(items[0]["administratorId"], "222222222222");
        assert_eq!(json["detectiveMembers"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn detective_datasource_packages_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                LIST_DATASOURCE_PACKAGES,
                r#"{"GraphArn":"arn:graph:1","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"DatasourcePackages":{"DETECTIVE_CORE":{"DatasourcePackageIngestState":"STARTED"}},"NextToken":"cursor-c"}"#,
            ),
        )]);
        let schema = build_query_schema(DetectiveQuery)
            .data(DetectiveClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ detectiveDatasourcePackages(graphArn: "arn:graph:1", limit: 1) { items { datasourcePackage ingestState } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["detectiveDatasourcePackages"]["items"];
        assert_eq!(items[0]["datasourcePackage"], "DETECTIVE_CORE");
        assert_eq!(items[0]["ingestState"], "STARTED");
        assert_eq!(json["detectiveDatasourcePackages"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }
}
