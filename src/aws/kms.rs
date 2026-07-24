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

    /// Lists key summaries then describes each one, capped at `limit` total
    /// keys (default unlimited) and resumed from `next_token`. `ListKeys`
    /// has both `limit` (1-1000, no documented minimum) and `marker`
    /// (verified against pinned `aws-sdk-kms` 1.111.0's
    /// `operation/list_keys/_list_keys_{input,output}.rs`), so the wafv2
    /// pattern applies: request `limit(remaining)` each iteration, filter
    /// empty-string markers, truncate at the end. The N+1 `describe_key`
    /// fan-out only runs over the current page's key ids.
    pub async fn list_and_describe_keys(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_kms::types::KeyMetadata>, Option<String>), VaporError> {
        let mut key_ids: Vec<String> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_keys();
            if let Some(l) = limit {
                req = req.limit(l - key_ids.len() as i32);
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for key_entry in output.keys() {
                if let Some(key_id) = key_entry.key_id() {
                    key_ids.push(key_id.to_string());
                }
            }

            marker = match output.next_marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| key_ids.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            key_ids.truncate(l.max(0) as usize);
        }

        let mut metadata: Vec<aws_sdk_kms::types::KeyMetadata> = Vec::new();
        for key_id in key_ids {
            let output = self
                .inner
                .describe_key()
                .key_id(&key_id)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            if let Some(km) = output.key_metadata {
                metadata.push(km);
            }
        }

        Ok((metadata, marker))
    }

    /// Lists aliases, optionally filtered by `key_id`, capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListAliases` has both `limit` (1-100, no documented minimum) and
    /// `marker` (verified against pinned `aws-sdk-kms` 1.111.0's
    /// `operation/list_aliases/_list_aliases_{input,output}.rs`). Same
    /// wafv2 pattern as `list_and_describe_keys` above.
    pub async fn list_aliases(
        &self,
        key_id: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_kms::types::AliasListEntry>, Option<String>), VaporError> {
        let mut items: Vec<aws_sdk_kms::types::AliasListEntry> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_aliases();
            if let Some(kid) = key_id {
                req = req.key_id(kid);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            items.extend(output.aliases().iter().cloned());

            marker = match output.next_marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            items.truncate(l.max(0) as usize);
        }

        Ok((items, marker))
    }

    /// Lists policy names for a key, capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListKeyPolicies` has both
    /// `limit` (1-1000, no documented minimum) and `marker` (verified
    /// against pinned `aws-sdk-kms` 1.111.0's
    /// `operation/list_key_policies/_list_key_policies_{input,output}.rs`).
    /// Same wafv2 pattern as above (in practice AWS only ever returns one
    /// policy, "default", but the SDK op is genuinely paginated).
    pub async fn list_key_policy_names(
        &self,
        key_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items: Vec<String> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_key_policies().key_id(key_id);
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            items.extend(output.policy_names().iter().map(|s| s.to_string()));

            marker = match output.next_marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            items.truncate(l.max(0) as usize);
        }

        Ok((items, marker))
    }

    /// Returns `Some(bool)` for symmetric CMKs indicating whether automatic annual rotation
    /// is enabled. Returns `None` for keys that don't support rotation (asymmetric, HMAC,
    /// AWS-managed, or keys with imported material) — `UnsupportedOperationException` is
    /// treated as "not applicable" rather than an error.
    pub async fn get_key_rotation_status(&self, key_id: &str) -> Result<Option<bool>, VaporError> {
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
                    Err(crate::error::sdk_err(e))
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
            .map_err(crate::error::sdk_err)?;

        Ok(output.policy().map(|s| s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://kms.us-east-1.amazonaws.com/";

    // --- list_and_describe_keys ---

    #[tokio::test]
    async fn list_and_describe_keys_lists_all_when_no_limit() {
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
                    r#"{"KeyMetadata":{"KeyId":"key-1","Enabled":true,"KeyState":"Enabled"}}"#,
                ),
            ),
        ]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (metadata, token) = client.list_and_describe_keys(None, None).await.unwrap();

        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].key_id(), "key-1");
        assert!(metadata[0].enabled());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_and_describe_keys_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Marker":"cursor-a"}"#),
                json_response(200, r#"{"Keys":[{"KeyId":"key-2"}],"Truncated":false}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-2"}"#),
                json_response(200, r#"{"KeyMetadata":{"KeyId":"key-2","Enabled":false}}"#),
            ),
        ]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (metadata, token) = client
            .list_and_describe_keys(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].key_id(), "key-2");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_and_describe_keys_stops_at_limit_with_client_side_truncation() {
        // ListKeys can return more entries than requested via `Limit`; the
        // wrapper truncates client-side after the loop and only fans out
        // `describe_key` over the surviving (truncated) ids.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":1}"#),
                json_response(
                    200,
                    r#"{"Keys":[{"KeyId":"key-1"},{"KeyId":"key-2"}],"Truncated":true,"NextMarker":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
                json_response(200, r#"{"KeyMetadata":{"KeyId":"key-1","Enabled":true}}"#),
            ),
        ]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (metadata, token) = client.list_and_describe_keys(Some(1), None).await.unwrap();

        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].key_id(), "key-1");
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_and_describe_keys_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":10}"#),
                json_response(
                    200,
                    r#"{"Keys":[{"KeyId":"key-1"}],"Truncated":true,"NextMarker":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Marker":"p2","Limit":9}"#),
                json_response(200, r#"{"Keys":[{"KeyId":"key-2"}],"Truncated":false}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
                json_response(200, r#"{"KeyMetadata":{"KeyId":"key-1","Enabled":true}}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-2"}"#),
                json_response(200, r#"{"KeyMetadata":{"KeyId":"key-2","Enabled":false}}"#),
            ),
        ]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (metadata, token) = client.list_and_describe_keys(Some(10), None).await.unwrap();

        assert_eq!(metadata.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_and_describe_keys_propagates_list_keys_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("NotFoundException", "no such resource"),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_and_describe_keys(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFoundException".to_string()));
                assert_eq!(message, "no such resource");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_and_describe_keys_propagates_describe_key_errors() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"Keys":[{"KeyId":"key-1"}],"Truncated":false}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
                json_error_response("KMSInvalidStateException", "key is disabled"),
            ),
        ]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_and_describe_keys(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("KMSInvalidStateException".to_string()));
                assert_eq!(message, "key is disabled");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- list_aliases ---

    #[tokio::test]
    async fn list_aliases_lists_all_when_no_limit_no_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Aliases":[{"AliasName":"alias/a","TargetKeyId":"key-1"}],"Truncated":false}"#,
            ),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_aliases(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].alias_name(), Some("alias/a"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_aliases_filters_by_key_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
            json_response(
                200,
                r#"{"Aliases":[{"AliasName":"alias/a"}],"Truncated":false}"#,
            ),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .list_aliases(Some("key-1"), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_aliases_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Marker":"cursor-a"}"#),
            json_response(
                200,
                r#"{"Aliases":[{"AliasName":"alias/b"}],"Truncated":false}"#,
            ),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_aliases(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].alias_name(), Some("alias/b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_aliases_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":1}"#),
            json_response(
                200,
                r#"{"Aliases":[{"AliasName":"alias/a"},{"AliasName":"alias/b"}],"Truncated":true,"NextMarker":"page2"}"#,
            ),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_aliases(None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].alias_name(), Some("alias/a"));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_aliases_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":10}"#),
                json_response(
                    200,
                    r#"{"Aliases":[{"AliasName":"alias/a"}],"Truncated":true,"NextMarker":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Marker":"p2","Limit":9}"#),
                json_response(
                    200,
                    r#"{"Aliases":[{"AliasName":"alias/b"}],"Truncated":false}"#,
                ),
            ),
        ]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_aliases(None, Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_aliases_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidArnException", "bad arn"),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_aliases(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidArnException".to_string()));
                assert_eq!(message, "bad arn");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- list_key_policy_names ---

    #[tokio::test]
    async fn list_key_policy_names_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
            json_response(200, r#"{"PolicyNames":["default"],"Truncated":false}"#),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_key_policy_names("key-1", None, None)
            .await
            .unwrap();

        assert_eq!(items, vec!["default".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_key_policy_names_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1","Marker":"cursor-a"}"#),
            json_response(200, r#"{"PolicyNames":["default"],"Truncated":false}"#),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_key_policy_names("key-1", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items, vec!["default".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_key_policy_names_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1","Limit":1}"#),
            json_response(
                200,
                r#"{"PolicyNames":["default","other"],"Truncated":true,"NextMarker":"page2"}"#,
            ),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_key_policy_names("key-1", Some(1), None)
            .await
            .unwrap();

        assert_eq!(items, vec!["default".to_string()]);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_key_policy_names_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
            json_error_response("NotFoundException", "key not found"),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_key_policy_names("key-1", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFoundException".to_string()));
                assert_eq!(message, "key not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- get_key_rotation_status ---

    #[tokio::test]
    async fn get_key_rotation_status_returns_true_when_enabled() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
            json_response(200, r#"{"KeyRotationEnabled":true,"KeyId":"key-1"}"#),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let enabled = client.get_key_rotation_status("key-1").await.unwrap();

        assert_eq!(enabled, Some(true));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_key_rotation_status_returns_none_for_unsupported_operation() {
        // Asymmetric/HMAC/AWS-managed keys don't support rotation; AWS
        // signals this via `UnsupportedOperationException`, which the
        // wrapper treats as "not applicable" rather than an error.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
            json_error_response(
                "UnsupportedOperationException",
                "not supported for asymmetric keys",
            ),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let enabled = client.get_key_rotation_status("key-1").await.unwrap();

        assert_eq!(enabled, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_key_rotation_status_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1"}"#),
            json_error_response("NotFoundException", "no such key"),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let err = client.get_key_rotation_status("key-1").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFoundException".to_string()));
                assert_eq!(message, "no such key");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // --- get_key_policy ---

    #[tokio::test]
    async fn get_key_policy_returns_policy() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1","PolicyName":"default"}"#),
            json_response(
                200,
                r#"{"Policy":"{\"Version\":\"2012-10-17\"}","PolicyName":"default"}"#,
            ),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let policy = client.get_key_policy("key-1", "default").await.unwrap();

        assert_eq!(policy, Some(r#"{"Version":"2012-10-17"}"#.to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_key_policy_returns_none_when_policy_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1","PolicyName":"default"}"#),
            json_response(200, r#"{"PolicyName":"default"}"#),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let policy = client.get_key_policy("key-1", "default").await.unwrap();

        assert_eq!(policy, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_key_policy_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"KeyId":"key-1","PolicyName":"default"}"#),
            json_error_response("NotFoundException", "no such key"),
        )]);
        let client = KmsClient::new(&sdk_config(http_client.clone()));

        let err = client.get_key_policy("key-1", "default").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFoundException".to_string()));
                assert_eq!(message, "no such key");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
