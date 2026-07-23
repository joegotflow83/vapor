use async_graphql::{Context, Object, Result};

use crate::aws::redshift_serverless::RedshiftServerlessClient;
use crate::schema::pagination::Page;
use crate::schema::redshift_serverless::types::{
    RedshiftServerlessNamespace, RedshiftServerlessWorkgroup,
};

#[derive(Default)]
pub struct RedshiftServerlessQuery;

#[Object]
impl RedshiftServerlessQuery {
    /// Lists Redshift Serverless namespaces, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn redshift_serverless_namespaces(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RedshiftServerlessNamespace>> {
        let client = ctx.data::<RedshiftServerlessClient>()?;
        let (namespaces, next_token) = client.list_namespaces(limit, next_token).await?;
        Ok(Page {
            items: namespaces
                .into_iter()
                .map(RedshiftServerlessNamespace::from)
                .collect(),
            next_token,
        })
    }

    /// Lists Redshift Serverless workgroups, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn redshift_serverless_workgroups(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RedshiftServerlessWorkgroup>> {
        let client = ctx.data::<RedshiftServerlessClient>()?;
        let (workgroups, next_token) = client.list_workgroups(limit, next_token).await?;
        Ok(Page {
            items: workgroups
                .into_iter()
                .map(RedshiftServerlessWorkgroup::from)
                .collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    // Crate name (`aws-sdk-redshiftserverless`) doesn't match the endpoint
    // hostname (`redshift-serverless.*`) — memory gotcha 3, same note as
    // `src/aws/redshift_serverless.rs`'s own test module.
    const BASE: &str = "https://redshift-serverless.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn redshift_serverless_namespaces_maps_items() {
        // Mocked response carries a `nextToken`, so `limit: 1` is required
        // (gotcha 29) to stop `list_namespaces`'s internal pagination loop
        // after one page instead of chasing an unmocked second request.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"namespaces":[{"namespaceArn":"arn:aws:redshift-serverless:us-east-1:111111111111:namespace/ns-1","namespaceName":"ns-1","status":"AVAILABLE"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(RedshiftServerlessQuery)
            .data(RedshiftServerlessClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ redshiftServerlessNamespaces(limit: 1) \
                 { items { namespaceName namespaceArn status } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["redshiftServerlessNamespaces"]["items"];
        assert_eq!(items[0]["namespaceName"], "ns-1");
        assert_eq!(
            items[0]["namespaceArn"],
            "arn:aws:redshift-serverless:us-east-1:111111111111:namespace/ns-1"
        );
        assert_eq!(items[0]["status"], "AVAILABLE");
        assert_eq!(json["redshiftServerlessNamespaces"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn redshift_serverless_workgroups_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"workgroups":[{"workgroupArn":"arn:aws:redshift-serverless:us-east-1:111111111111:workgroup/wg-1","workgroupName":"wg-1","namespaceName":"ns-1","status":"AVAILABLE"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(RedshiftServerlessQuery)
            .data(RedshiftServerlessClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ redshiftServerlessWorkgroups(limit: 1) \
                 { items { workgroupName workgroupArn namespaceName status } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["redshiftServerlessWorkgroups"]["items"];
        assert_eq!(items[0]["workgroupName"], "wg-1");
        assert_eq!(
            items[0]["workgroupArn"],
            "arn:aws:redshift-serverless:us-east-1:111111111111:workgroup/wg-1"
        );
        assert_eq!(items[0]["namespaceName"], "ns-1");
        assert_eq!(items[0]["status"], "AVAILABLE");
        assert_eq!(json["redshiftServerlessWorkgroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
