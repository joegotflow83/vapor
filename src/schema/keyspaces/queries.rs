use async_graphql::{Context, Object, Result};

use crate::aws::keyspaces::KeyspacesClient;
use crate::schema::keyspaces::types::{KeyspacesKeyspace, KeyspacesTable};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct KeyspacesQuery;

#[Object]
impl KeyspacesQuery {
    /// Lists keyspaces, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn keyspaces_keyspaces(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<KeyspacesKeyspace>> {
        let client = ctx.data::<KeyspacesClient>()?;
        let (keyspaces, next_token) = client.list_keyspaces(limit, next_token).await?;
        Ok(Page {
            items: keyspaces.into_iter().map(KeyspacesKeyspace::from).collect(),
            next_token,
        })
    }

    /// Lists tables, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn keyspaces_tables(
        &self,
        ctx: &Context<'_>,
        keyspace_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<KeyspacesTable>> {
        let client = ctx.data::<KeyspacesClient>()?;
        let (tables, next_token) = client.list_tables(&keyspace_name, limit, next_token).await?;
        Ok(Page {
            items: tables.into_iter().map(KeyspacesTable::from).collect(),
            next_token,
        })
    }

    async fn keyspaces_table(
        &self,
        ctx: &Context<'_>,
        keyspace_name: String,
        table_name: String,
    ) -> Result<Option<KeyspacesTable>> {
        let client = ctx.data::<KeyspacesClient>()?;
        let table = client.get_table(&keyspace_name, &table_name).await?;
        Ok(table.map(KeyspacesTable::from))
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::keyspaces::KeyspacesClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::KeyspacesQuery;

    const ENDPOINT: &str = "https://cassandra.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn keyspaces_keyspaces_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"keyspaces":[{"keyspaceName":"ks1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks1","replicationStrategy":"MULTI_REGION","replicationRegions":["us-east-1","eu-west-1"]}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(KeyspacesQuery)
            .data(KeyspacesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ keyspacesKeyspaces(limit: 1) { items { keyspaceName resourceArn replicationStrategy replicationRegions } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["keyspacesKeyspaces"]["items"];
        assert_eq!(items[0]["keyspaceName"], "ks1");
        assert_eq!(
            items[0]["resourceArn"],
            "arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks1"
        );
        assert_eq!(items[0]["replicationStrategy"], "MULTI_REGION");
        assert_eq!(items[0]["replicationRegions"], serde_json::json!(["us-east-1", "eu-west-1"]));
        assert_eq!(json["keyspacesKeyspaces"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn keyspaces_tables_maps_items_and_next_token() {
        // `list_tables` fans out into a per-table `GetTable` call inside
        // `KeyspacesClient` itself (not the resolver body), same as iam/iot's
        // discovery+fan-out resolvers — this just proves the resolver's own
        // field mapping end-to-end, the fan-out loop is covered in
        // `src/aws/keyspaces.rs`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"keyspaceName":"mykeyspace","maxResults":1}"#),
                json_response(
                    200,
                    r#"{"tables":[{"keyspaceName":"mykeyspace","tableName":"t1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t1"}],"nextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"keyspaceName":"mykeyspace","tableName":"t1"}"#),
                json_response(
                    200,
                    r#"{"keyspaceName":"mykeyspace","tableName":"t1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t1","status":"ACTIVE","creationTimestamp":1704067200,"capacitySpecification":{"throughputMode":"PAY_PER_REQUEST"},"encryptionSpecification":{"type":"AWS_OWNED_KMS_KEY"},"pointInTimeRecovery":{"status":"ENABLED"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(KeyspacesQuery)
            .data(KeyspacesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ keyspacesTables(keyspaceName: "mykeyspace", limit: 1) { items { keyspaceName tableName resourceArn status creationTimestamp capacitySpecification { throughputMode } encryptionSpecification { type kmsKeyIdentifier } pointInTimeRecovery } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["keyspacesTables"]["items"];
        assert_eq!(items[0]["keyspaceName"], "mykeyspace");
        assert_eq!(items[0]["tableName"], "t1");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["creationTimestamp"], "2024-01-01T00:00:00+00:00");
        assert_eq!(items[0]["capacitySpecification"]["throughputMode"], "PAY_PER_REQUEST");
        assert_eq!(items[0]["encryptionSpecification"]["type"], "AWS_OWNED_KMS_KEY");
        assert!(items[0]["encryptionSpecification"]["kmsKeyIdentifier"].is_null());
        assert_eq!(items[0]["pointInTimeRecovery"], true);
        assert_eq!(json["keyspacesTables"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn keyspaces_table_maps_single_table() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"keyspaceName":"mykeyspace","tableName":"t1"}"#),
            json_response(
                200,
                r#"{"keyspaceName":"mykeyspace","tableName":"t1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t1","status":"ACTIVE"}"#,
            ),
        )]);
        let schema = build_query_schema(KeyspacesQuery)
            .data(KeyspacesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ keyspacesTable(keyspaceName: "mykeyspace", tableName: "t1") { keyspaceName tableName status } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["keyspacesTable"]["keyspaceName"], "mykeyspace");
        assert_eq!(json["keyspacesTable"]["tableName"], "t1");
        assert_eq!(json["keyspacesTable"]["status"], "ACTIVE");
        http_client.relaxed_requests_match();
    }
}
