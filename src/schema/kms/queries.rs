use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::kms::KmsClient;
use crate::schema::kms::types::{KmsAlias, KmsKey, KmsKeyPolicy};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct KmsQuery;

#[Object]
impl KmsQuery {
    /// List KMS customer master keys with full metadata including rotation status (CIS 3.8).
    /// `limit` caps the total number of results (default unlimited); pass
    /// `nextToken` from a prior page to resume.
    async fn kms_keys(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<KmsKey>> {
        let kms = ctx.data::<KmsClient>()?;
        let (metadata_list, next_token) = kms.list_and_describe_keys(limit, next_token).await?;

        // Fan-out: fetch rotation status for each key in parallel.
        // Keys that don't support rotation (asymmetric, HMAC, external) return None.
        let rotation_futures: Vec<_> = metadata_list
            .iter()
            .map(|meta| {
                let key_id = meta.key_id().to_string();
                async move { kms.get_key_rotation_status(&key_id).await }
            })
            .collect();
        let rotations = join_all(rotation_futures).await;

        let items = metadata_list
            .iter()
            .zip(rotations.iter())
            .map(|(meta, rotation_result)| {
                let rotation_enabled = rotation_result.as_ref().ok().and_then(|r| *r);
                KmsKey::from_sdk(meta, rotation_enabled)
            })
            .collect();

        Ok(Page { items, next_token })
    }

    /// List KMS aliases. Optionally filter by keyId. `limit` caps the total
    /// number of results (default unlimited); pass `nextToken` from a prior
    /// page to resume.
    async fn kms_aliases(
        &self,
        ctx: &Context<'_>,
        key_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<KmsAlias>> {
        let kms = ctx.data::<KmsClient>()?;
        let (results, next_token) = kms
            .list_aliases(key_id.as_deref(), limit, next_token)
            .await?;
        let items = results.into_iter().map(KmsAlias::from).collect();
        Ok(Page { items, next_token })
    }

    /// List policy names for a KMS key (typically just "default"). `limit`
    /// caps the total number of results (default unlimited); pass
    /// `nextToken` from a prior page to resume.
    async fn kms_key_policy_names(
        &self,
        ctx: &Context<'_>,
        key_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<String>> {
        let kms = ctx.data::<KmsClient>()?;
        let (items, next_token) = kms
            .list_key_policy_names(&key_id, limit, next_token)
            .await?;
        Ok(Page { items, next_token })
    }

    /// Get a specific KMS key policy by key ID and policy name.
    async fn kms_key_policy(
        &self,
        ctx: &Context<'_>,
        key_id: String,
        policy_name: String,
    ) -> Result<Option<KmsKeyPolicy>> {
        let kms = ctx.data::<KmsClient>()?;
        let policy = kms.get_key_policy(&key_id, &policy_name).await?;
        Ok(policy.map(|p| KmsKeyPolicy {
            key_id,
            policy_name,
            policy: p,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::kms::KmsClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::KmsQuery;

    const ENDPOINT: &str = "https://kms.us-east-1.amazonaws.com/";

    // --- kms_keys (bespoke: list+describe, then a rotation-status fan-out) ---

    #[tokio::test]
    async fn kms_keys_maps_full_metadata_with_rotation_status() {
        // Sequential per `list_and_describe_keys`'s own for-loop (list, then
        // describe), followed by the resolver's own `get_key_rotation_status`
        // fan-out — with a single key, `join_all` resolves in the same
        // declared order under `StaticReplayClient`'s synchronous connector.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"Keys":[{"KeyId":"key-1","KeyArn":"arn:aws:kms:us-east-1:123456789012:key/key-1"}],"Truncated":false}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
                json_response(
                    200,
                    r#"{"KeyMetadata":{"KeyId":"key-1","Arn":"arn:aws:kms:us-east-1:123456789012:key/key-1","Description":"my key","KeyState":"Enabled","KeyUsage":"ENCRYPT_DECRYPT","KeySpec":"SYMMETRIC_DEFAULT","Origin":"AWS_KMS","MultiRegion":false,"Enabled":true,"CreationDate":1700000000}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
                json_response(200, r#"{"KeyRotationEnabled":true,"KeyId":"key-1"}"#),
            ),
        ]);
        let schema = build_query_schema(KmsQuery)
            .data(KmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ kmsKeys { items { keyId arn description keyState keyUsage keySpec origin \
                 multiRegion enabled creationDate rotationEnabled } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["kmsKeys"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["keyId"], "key-1");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:kms:us-east-1:123456789012:key/key-1"
        );
        assert_eq!(items[0]["description"], "my key");
        assert_eq!(items[0]["keyState"], "Enabled");
        assert_eq!(items[0]["keyUsage"], "ENCRYPT_DECRYPT");
        assert_eq!(items[0]["keySpec"], "SYMMETRIC_DEFAULT");
        assert_eq!(items[0]["origin"], "AWS_KMS");
        assert_eq!(items[0]["multiRegion"], false);
        assert_eq!(items[0]["enabled"], true);
        assert!(!items[0]["creationDate"].is_null());
        assert_eq!(items[0]["rotationEnabled"], true);
        assert!(json["kmsKeys"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn kms_keys_rotation_not_applicable_maps_to_null() {
        // `UnsupportedOperationException` from `get_key_rotation_status` is
        // handled at the aws-layer as "not applicable", not an error — the
        // resolver surfaces it as `rotationEnabled: null`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"Keys":[{"KeyId":"key-asym"}],"Truncated":false}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-asym"}"#),
                json_response(
                    200,
                    r#"{"KeyMetadata":{"KeyId":"key-asym","KeySpec":"RSA_2048"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-asym"}"#),
                json_error_response(
                    "UnsupportedOperationException",
                    "not supported for asymmetric keys",
                ),
            ),
        ]);
        let schema = build_query_schema(KmsQuery)
            .data(KmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ kmsKeys { items { keyId rotationEnabled } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["kmsKeys"]["items"][0]["keyId"], "key-asym");
        assert!(json["kmsKeys"]["items"][0]["rotationEnabled"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn kms_keys_passes_limit_and_next_token_to_discovery_call() {
        // No keys returned, so no describe/rotation fan-out calls follow —
        // isolates the discovery-call argument-passthrough behavior.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":5,"Marker":"cursor-a"}"#),
            json_response(200, r#"{"Keys":[],"Truncated":false}"#),
        )]);
        let schema = build_query_schema(KmsQuery)
            .data(KmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ kmsKeys(limit: 5, nextToken: "cursor-a") { items { keyId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        http_client.relaxed_requests_match();
    }

    // --- kms_aliases (bare passthrough) ---

    #[tokio::test]
    async fn kms_aliases_filters_by_key_id_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
            json_response(
                200,
                r#"{"Aliases":[{"AliasName":"alias/my-key","AliasArn":"arn:aws:kms:us-east-1:123456789012:alias/my-key","TargetKeyId":"key-1"}],"Truncated":false}"#,
            ),
        )]);
        let schema = build_query_schema(KmsQuery)
            .data(KmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ kmsAliases(keyId: "key-1") { items { aliasName aliasArn targetKeyId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["kmsAliases"]["items"][0]["aliasName"], "alias/my-key");
        assert_eq!(json["kmsAliases"]["items"][0]["targetKeyId"], "key-1");
        http_client.relaxed_requests_match();
    }

    // --- kms_key_policy_names (bare passthrough) ---

    #[tokio::test]
    async fn kms_key_policy_names_lists_names() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
            json_response(200, r#"{"PolicyNames":["default"],"Truncated":false}"#),
        )]);
        let schema = build_query_schema(KmsQuery)
            .data(KmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ kmsKeyPolicyNames(keyId: "key-1") { items nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["kmsKeyPolicyNames"]["items"][0], "default");
        http_client.relaxed_requests_match();
    }

    // --- kms_key_policy (single lookup, Option mapping) ---

    #[tokio::test]
    async fn kms_key_policy_returns_policy_when_present() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1","PolicyName":"default"}"#),
            json_response(
                200,
                r#"{"Policy":"{\"Version\":\"2012-10-17\"}","PolicyName":"default"}"#,
            ),
        )]);
        let schema = build_query_schema(KmsQuery)
            .data(KmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ kmsKeyPolicy(keyId: "key-1", policyName: "default") { keyId policyName policy } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["kmsKeyPolicy"]["keyId"], "key-1");
        assert_eq!(json["kmsKeyPolicy"]["policyName"], "default");
        assert_eq!(
            json["kmsKeyPolicy"]["policy"],
            r#"{"Version":"2012-10-17"}"#
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn kms_key_policy_returns_none_when_policy_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1","PolicyName":"default"}"#),
            json_response(200, r#"{"PolicyName":"default"}"#),
        )]);
        let schema = build_query_schema(KmsQuery)
            .data(KmsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ kmsKeyPolicy(keyId: "key-1", policyName: "default") { policy } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["kmsKeyPolicy"].is_null());
        http_client.relaxed_requests_match();
    }
}
