use async_graphql::{Context, Object, Result};

use crate::aws::ram::RamClient;
use crate::schema::pagination::Page;
use crate::schema::ram::types::{RamPrincipal, RamResource, RamResourceShare};

#[derive(Default)]
pub struct RamQuery;

#[Object]
impl RamQuery {
    /// Lists resource shares, optionally capped at `limit` results (default unlimited).
    async fn ram_resource_shares(
        &self,
        ctx: &Context<'_>,
        resource_owner: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RamResourceShare>> {
        let client = ctx.data::<RamClient>()?;
        let (shares, next_token) = client
            .list_resource_shares(resource_owner.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: shares.into_iter().map(RamResourceShare::from).collect(),
            next_token,
        })
    }

    /// Lists resources, optionally capped at `limit` results (default unlimited).
    async fn ram_resources(
        &self,
        ctx: &Context<'_>,
        resource_owner: String,
        resource_share_arns: Option<Vec<String>>,
        resource_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RamResource>> {
        let client = ctx.data::<RamClient>()?;
        let (resources, next_token) = client
            .list_resources(
                &resource_owner,
                resource_share_arns,
                resource_type,
                limit,
                next_token,
            )
            .await?;
        Ok(Page {
            items: resources.into_iter().map(RamResource::from).collect(),
            next_token,
        })
    }

    /// Lists principals, optionally capped at `limit` results (default unlimited).
    async fn ram_principals(
        &self,
        ctx: &Context<'_>,
        resource_owner: String,
        resource_share_arns: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RamPrincipal>> {
        let client = ctx.data::<RamClient>()?;
        let (principals, next_token) = client
            .list_principals(&resource_owner, resource_share_arns, limit, next_token)
            .await?;
        Ok(Page {
            items: principals.into_iter().map(RamPrincipal::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `RamClient` method each (see `src/aws/ram.rs`'s own test module for the
// pagination/limit/error-mapping behavior) — only light smoke tests are
// needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::ram::RamClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::RamQuery;

    const BASE: &str = "https://ram.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn ram_resource_shares_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/getresourceshares"),
                r#"{"resourceOwner":"SELF","maxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"resourceShares":[{"resourceShareArn":"arn:aws:ram:us-east-1:111111111111:resource-share/rs-1","name":"share-1","owningAccountId":"111111111111","status":"ACTIVE","allowExternalPrincipals":true,"creationTime":1700000000,"lastUpdatedTime":1710000000}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(RamQuery)
            .data(RamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ramResourceShares(limit: 1) { items { resourceShareArn name ownerId status allowExternalPrincipals creationTime lastUpdatedTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ramResourceShares"]["items"];
        assert_eq!(
            items[0]["resourceShareArn"],
            "arn:aws:ram:us-east-1:111111111111:resource-share/rs-1"
        );
        assert_eq!(items[0]["name"], "share-1");
        assert_eq!(items[0]["ownerId"], "111111111111");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["allowExternalPrincipals"], true);
        assert_eq!(items[0]["creationTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(json["ramResourceShares"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ram_resource_shares_forwards_resource_owner() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/getresourceshares"),
                r#"{"resourceOwner":"OTHER-ACCOUNTS"}"#,
            ),
            json_response(200, r#"{"resourceShares":[]}"#),
        )]);
        let schema = build_query_schema(RamQuery)
            .data(RamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ramResourceShares(resourceOwner: "OTHER-ACCOUNTS") { items { name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["ramResourceShares"]["items"].as_array().unwrap().len(),
            0
        );
        assert!(json["ramResourceShares"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ram_resources_maps_items_and_forwards_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/listresources"),
                r#"{"resourceOwner":"SELF","resourceShareArns":["rs-1"],"resourceType":"ec2:subnet","maxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"resources":[{"arn":"arn:aws:ec2:us-east-1:111111111111:subnet/subnet-abc","type":"ec2:subnet","resourceShareArn":"rs-1","status":"AVAILABLE","creationTime":1700000000,"lastUpdatedTime":1710000000}]}"#,
            ),
        )]);
        let schema = build_query_schema(RamQuery)
            .data(RamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ramResources(resourceOwner: "SELF", resourceShareArns: ["rs-1"], resourceType: "ec2:subnet", limit: 1) { items { arn type resourceShareArn status creationTime lastUpdatedTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ramResources"]["items"];
        assert_eq!(
            items[0]["arn"],
            "arn:aws:ec2:us-east-1:111111111111:subnet/subnet-abc"
        );
        assert_eq!(items[0]["type"], "ec2:subnet");
        assert_eq!(items[0]["resourceShareArn"], "rs-1");
        assert_eq!(items[0]["status"], "AVAILABLE");
        assert!(json["ramResources"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ram_principals_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/listprincipals"),
                r#"{"resourceOwner":"SELF","maxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"principals":[{"id":"222222222222","resourceShareArn":"rs-1","creationTime":1700000000,"lastUpdatedTime":1710000000,"external":false}]}"#,
            ),
        )]);
        let schema = build_query_schema(RamQuery)
            .data(RamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ramPrincipals(resourceOwner: "SELF", limit: 1) { items { id resourceShareArn creationTime lastUpdatedTime external } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ramPrincipals"]["items"];
        assert_eq!(items[0]["id"], "222222222222");
        assert_eq!(items[0]["resourceShareArn"], "rs-1");
        assert_eq!(items[0]["external"], false);
        assert!(json["ramPrincipals"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
