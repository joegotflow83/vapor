use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::kms::KmsClient;
use crate::schema::kms::types::{KmsAlias, KmsKey, KmsKeyPolicy};

#[derive(Default)]
pub struct KmsQuery;

#[Object]
impl KmsQuery {
    /// List all KMS customer master keys with full metadata including rotation status (CIS 3.8).
    async fn kms_keys(&self, ctx: &Context<'_>) -> Result<Vec<KmsKey>> {
        let kms = ctx.data::<KmsClient>()?;
        let metadata_list = kms.list_and_describe_keys().await?;

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

        let keys = metadata_list
            .iter()
            .zip(rotations.iter())
            .map(|(meta, rotation_result)| {
                let rotation_enabled = rotation_result.as_ref().ok().and_then(|r| *r);
                KmsKey::from_sdk(meta, rotation_enabled)
            })
            .collect();

        Ok(keys)
    }

    /// List KMS aliases. Optionally filter by keyId.
    async fn kms_aliases(
        &self,
        ctx: &Context<'_>,
        key_id: Option<String>,
    ) -> Result<Vec<KmsAlias>> {
        let kms = ctx.data::<KmsClient>()?;
        let results = kms.list_aliases(key_id.as_deref()).await?;
        Ok(results.into_iter().map(KmsAlias::from).collect())
    }

    /// List policy names for a KMS key (typically just "default").
    async fn kms_key_policy_names(
        &self,
        ctx: &Context<'_>,
        key_id: String,
    ) -> Result<Vec<String>> {
        let kms = ctx.data::<KmsClient>()?;
        Ok(kms.list_key_policy_names(&key_id).await?)
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
