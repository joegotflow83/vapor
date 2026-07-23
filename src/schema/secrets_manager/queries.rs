use async_graphql::{Context, Object, Result};

use crate::aws::secrets_manager::SecretsManagerClient;
use crate::schema::pagination::Page;
use crate::schema::secrets_manager::types::{Secret, SecretValue};

#[derive(Default)]
pub struct SecretsManagerQuery;

#[Object]
impl SecretsManagerQuery {
    /// List all Secrets Manager secrets with their metadata.
    /// `limit` optionally caps the number of results (default unlimited)
    /// and `next_token` resumes from a prior page.
    async fn secrets_list(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Secret>> {
        let client = ctx.data::<SecretsManagerClient>()?;
        let (entries, next_token) = client.list_secrets(limit, next_token).await?;
        Ok(Page {
            items: entries.into_iter().map(Secret::from).collect(),
            next_token,
        })
    }

    /// Describe a single secret by ID or ARN.
    async fn secret_describe(
        &self,
        ctx: &Context<'_>,
        secret_id: String,
    ) -> Result<Option<Secret>> {
        let client = ctx.data::<SecretsManagerClient>()?;
        let output = client.describe_secret(&secret_id).await?;
        Ok(Some(Secret::from(output)))
    }

    /// Retrieve the value of a secret by ID or ARN.
    async fn secret_value(
        &self,
        ctx: &Context<'_>,
        secret_id: String,
    ) -> Result<Option<SecretValue>> {
        let client = ctx.data::<SecretsManagerClient>()?;
        let output = client.get_secret_value(&secret_id).await?;
        Ok(Some(SecretValue::from(output)))
    }

    /// Fetch the resource-based policy document attached to a secret.
    /// Returns the raw JSON policy string, or null if no policy is attached.
    /// Reveals cross-account access grants beyond the secret owner — use to audit
    /// which principals (accounts, services, roles) can access credentials or API keys.
    async fn secret_resource_policy(
        &self,
        ctx: &Context<'_>,
        secret_id: String,
    ) -> Result<Option<String>> {
        let client = ctx.data::<SecretsManagerClient>()?;
        Ok(client.get_resource_policy(&secret_id).await?)
    }
}

// All 4 resolvers are 1:1 passthroughs to a single already-tested
// `SecretsManagerClient` method each (see `src/aws/secrets_manager.rs`'s own
// test module for the pagination/limit/error-mapping behavior) — only light
// smoke tests are needed here per the resolver-layer sweep's stated scope.
// `types.rs` already has its own `From` impl unit tests, no gap there.
#[cfg(test)]
mod tests {
    use crate::aws::secrets_manager::SecretsManagerClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::SecretsManagerQuery;

    const BASE: &str = "https://secretsmanager.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn secrets_list_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"SecretList":[{"ARN":"arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1","Name":"s-1","RotationEnabled":true,"PrimaryRegion":"us-east-1"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(SecretsManagerQuery)
            .data(SecretsManagerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ secretsList(limit: 1) { items { arn name rotationEnabled primaryRegion } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["secretsList"]["items"];
        assert_eq!(
            items[0]["arn"],
            "arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1"
        );
        assert_eq!(items[0]["name"], "s-1");
        assert_eq!(items[0]["rotationEnabled"], true);
        assert_eq!(items[0]["primaryRegion"], "us-east-1");
        assert_eq!(json["secretsList"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn secret_describe_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"s-1"}"#),
            json_response(
                200,
                r#"{"ARN":"arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1","Name":"s-1","Description":"a test secret","RotationEnabled":false}"#,
            ),
        )]);
        let schema = build_query_schema(SecretsManagerQuery)
            .data(SecretsManagerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ secretDescribe(secretId: "s-1") { arn name description rotationEnabled } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["secretDescribe"]["arn"],
            "arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1"
        );
        assert_eq!(json["secretDescribe"]["name"], "s-1");
        assert_eq!(json["secretDescribe"]["description"], "a test secret");
        assert_eq!(json["secretDescribe"]["rotationEnabled"], false);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn secret_value_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"s-1"}"#),
            json_response(
                200,
                r#"{"ARN":"arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1","Name":"s-1","SecretString":"super-secret-value","VersionId":"v-1","VersionStages":["AWSCURRENT"]}"#,
            ),
        )]);
        let schema = build_query_schema(SecretsManagerQuery)
            .data(SecretsManagerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ secretValue(secretId: "s-1") { arn name versionId secretString versionStages } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["secretValue"]["arn"],
            "arn:aws:secretsmanager:us-east-1:111111111111:secret:s-1"
        );
        assert_eq!(json["secretValue"]["name"], "s-1");
        assert_eq!(json["secretValue"]["versionId"], "v-1");
        assert_eq!(json["secretValue"]["secretString"], "super-secret-value");
        assert_eq!(
            json["secretValue"]["versionStages"],
            serde_json::json!(["AWSCURRENT"])
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn secret_resource_policy_returns_policy() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"SecretId":"s-1"}"#),
            json_response(
                200,
                r#"{"ARN":"arn:s-1","Name":"s-1","ResourcePolicy":"{\"Version\":\"2012-10-17\"}"}"#,
            ),
        )]);
        let schema = build_query_schema(SecretsManagerQuery)
            .data(SecretsManagerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ secretResourcePolicy(secretId: "s-1") }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["secretResourcePolicy"],
            r#"{"Version":"2012-10-17"}"#
        );
        http_client.relaxed_requests_match();
    }
}
