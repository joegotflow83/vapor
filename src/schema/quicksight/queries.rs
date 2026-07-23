use async_graphql::{Context, Object, Result};

use crate::aws::quicksight::QuickSightClient;
use crate::schema::pagination::Page;
use crate::schema::quicksight::types::{
    QuickSightDashboard, QuickSightDataSet, QuickSightDataSource, QuickSightUser,
};

#[derive(Default)]
pub struct QuickSightQuery;

#[Object]
impl QuickSightQuery {
    /// Lists QuickSight users, optionally capped at `limit` results (default unlimited).
    async fn quick_sight_users(
        &self,
        ctx: &Context<'_>,
        aws_account_id: String,
        namespace: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<QuickSightUser>> {
        let client = ctx.data::<QuickSightClient>()?;
        let (items, next_token) = client
            .list_users(aws_account_id, namespace, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(QuickSightUser::from).collect(),
            next_token,
        })
    }

    /// Lists QuickSight dashboards, optionally capped at `limit` results (default unlimited).
    async fn quick_sight_dashboards(
        &self,
        ctx: &Context<'_>,
        aws_account_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<QuickSightDashboard>> {
        let client = ctx.data::<QuickSightClient>()?;
        let (items, next_token) = client
            .list_dashboards(aws_account_id, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(QuickSightDashboard::from).collect(),
            next_token,
        })
    }

    /// Lists QuickSight data sets, optionally capped at `limit` results (default unlimited).
    async fn quick_sight_data_sets(
        &self,
        ctx: &Context<'_>,
        aws_account_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<QuickSightDataSet>> {
        let client = ctx.data::<QuickSightClient>()?;
        let (items, next_token) = client
            .list_data_sets(aws_account_id, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(QuickSightDataSet::from).collect(),
            next_token,
        })
    }

    /// Lists QuickSight data sources, optionally capped at `limit` results (default unlimited).
    async fn quick_sight_data_sources(
        &self,
        ctx: &Context<'_>,
        aws_account_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<QuickSightDataSource>> {
        let client = ctx.data::<QuickSightClient>()?;
        let (items, next_token) = client
            .list_data_sources(aws_account_id, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(QuickSightDataSource::from).collect(),
            next_token,
        })
    }
}

// All four resolvers are 1:1 passthroughs to a single already-tested
// `QuickSightClient` method each (see `src/aws/quicksight.rs`'s own test
// module for the pagination/limit/error-mapping behavior) — only light
// smoke tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::quicksight::QuickSightClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::QuickSightQuery;

    const BASE: &str = "https://quicksight.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn quick_sight_users_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/accounts/123456789012/namespaces/default/users?max-results=1"),
                "",
            ),
            json_response(
                200,
                r#"{"UserList":[{"UserName":"alice","Arn":"arn:aws:quicksight:us-east-1:123456789012:user/default/alice","Email":"alice@example.com","Role":"ADMIN","IdentityType":"IAM","Active":true,"PrincipalId":"p-1"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(QuickSightQuery)
            .data(QuickSightClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ quickSightUsers(awsAccountId: "123456789012", limit: 1) { items { userName arn email role identityType active principalId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["quickSightUsers"]["items"];
        assert_eq!(items[0]["userName"], "alice");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:quicksight:us-east-1:123456789012:user/default/alice"
        );
        assert_eq!(items[0]["email"], "alice@example.com");
        assert_eq!(items[0]["role"], "ADMIN");
        assert_eq!(items[0]["identityType"], "IAM");
        assert_eq!(items[0]["active"], true);
        assert_eq!(items[0]["principalId"], "p-1");
        assert_eq!(json["quickSightUsers"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn quick_sight_dashboards_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/dashboards"), ""),
            json_response(
                200,
                r#"{"DashboardSummaryList":[{"DashboardId":"d-1","Arn":"arn:aws:quicksight:us-east-1:123456789012:dashboard/d-1","Name":"Sales","CreatedTime":1704067200,"LastUpdatedTime":1704153600,"PublishedVersionNumber":3,"LastPublishedTime":1704067200}]}"#,
            ),
        )]);
        let schema = build_query_schema(QuickSightQuery)
            .data(QuickSightClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ quickSightDashboards(awsAccountId: "123456789012") { items { dashboardId arn name createdTime lastUpdatedTime publishedVersionNumber lastPublishedTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["quickSightDashboards"]["items"];
        assert_eq!(items[0]["dashboardId"], "d-1");
        assert_eq!(items[0]["name"], "Sales");
        assert_eq!(items[0]["createdTime"], "2024-01-01T00:00:00+00:00");
        assert_eq!(items[0]["publishedVersionNumber"], 3);
        assert!(json["quickSightDashboards"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn quick_sight_data_sets_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/data-sets"), ""),
            json_response(
                200,
                r#"{"DataSetSummaries":[{"DataSetId":"ds-1","Arn":"arn:aws:quicksight:us-east-1:123456789012:dataset/ds-1","Name":"Revenue","CreatedTime":1704067200,"LastUpdatedTime":1704153600,"ImportMode":"SPICE"}]}"#,
            ),
        )]);
        let schema = build_query_schema(QuickSightQuery)
            .data(QuickSightClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ quickSightDataSets(awsAccountId: "123456789012") { items { dataSetId arn name importMode } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["quickSightDataSets"]["items"];
        assert_eq!(items[0]["dataSetId"], "ds-1");
        assert_eq!(items[0]["name"], "Revenue");
        assert_eq!(items[0]["importMode"], "SPICE");
        assert!(json["quickSightDataSets"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn quick_sight_data_sources_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/accounts/123456789012/data-sources"), ""),
            json_response(
                200,
                r#"{"DataSources":[{"DataSourceId":"src-1","Arn":"arn:aws:quicksight:us-east-1:123456789012:datasource/src-1","Name":"Athena Source","Type":"ATHENA","Status":"CREATION_SUCCESSFUL","CreatedTime":1704067200,"LastUpdatedTime":1704153600}]}"#,
            ),
        )]);
        let schema = build_query_schema(QuickSightQuery)
            .data(QuickSightClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ quickSightDataSources(awsAccountId: "123456789012") { items { dataSourceId arn name type status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["quickSightDataSources"]["items"];
        assert_eq!(items[0]["dataSourceId"], "src-1");
        assert_eq!(items[0]["name"], "Athena Source");
        assert_eq!(items[0]["type"], "ATHENA");
        assert_eq!(items[0]["status"], "CREATION_SUCCESSFUL");
        assert!(json["quickSightDataSources"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
