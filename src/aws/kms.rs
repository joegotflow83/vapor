use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct KmsClient {
    inner: aws_sdk_kms::Client,
}

impl KmsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_kms::Client::new(config),
        }
    }

    pub async fn list_and_describe_keys(
        &self,
    ) -> Result<Vec<aws_sdk_kms::types::KeyMetadata>, VaporError> {
        let mut key_ids: Vec<String> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_keys();
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for key_entry in output.keys() {
                if let Some(key_id) = key_entry.key_id() {
                    key_ids.push(key_id.to_string());
                }
            }

            if output.truncated() {
                next_marker = output.next_marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        let mut metadata: Vec<aws_sdk_kms::types::KeyMetadata> = Vec::new();
        for key_id in key_ids {
            let output = self
                .inner
                .describe_key()
                .key_id(&key_id)
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            if let Some(km) = output.key_metadata {
                metadata.push(km);
            }
        }

        Ok(metadata)
    }

    pub async fn list_aliases(
        &self,
        key_id: Option<&str>,
    ) -> Result<Vec<aws_sdk_kms::types::AliasListEntry>, VaporError> {
        let mut all_aliases: Vec<aws_sdk_kms::types::AliasListEntry> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_aliases();
            if let Some(kid) = key_id {
                req = req.key_id(kid);
            }
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_aliases.extend(output.aliases().iter().cloned());

            if output.truncated() {
                next_marker = output.next_marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(all_aliases)
    }

    pub async fn list_key_policy_names(
        &self,
        key_id: &str,
    ) -> Result<Vec<String>, VaporError> {
        let mut all_names: Vec<String> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_key_policies().key_id(key_id);
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_names.extend(output.policy_names().iter().map(|s| s.to_string()));

            if output.truncated() {
                next_marker = output.next_marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(all_names)
    }

    /// Returns `Some(bool)` for symmetric CMKs indicating whether automatic annual rotation
    /// is enabled. Returns `None` for keys that don't support rotation (asymmetric, HMAC,
    /// AWS-managed, or keys with imported material) — `UnsupportedOperationException` is
    /// treated as "not applicable" rather than an error.
    pub async fn get_key_rotation_status(
        &self,
        key_id: &str,
    ) -> Result<Option<bool>, VaporError> {
        match self
            .inner
            .get_key_rotation_status()
            .key_id(key_id)
            .send()
            .await
        {
            Ok(output) => Ok(Some(output.key_rotation_enabled())),
            Err(e) => {
                if e.as_service_error()
                    .map(|se| se.is_unsupported_operation_exception())
                    .unwrap_or(false)
                {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(e.to_string()))
                }
            }
        }
    }

    pub async fn get_key_policy(
        &self,
        key_id: &str,
        policy_name: &str,
    ) -> Result<Option<String>, VaporError> {
        let output = self
            .inner
            .get_key_policy()
            .key_id(key_id)
            .policy_name(policy_name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.policy().map(|s| s.to_string()))
    }
}
