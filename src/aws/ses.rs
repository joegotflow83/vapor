use aws_config::SdkConfig;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;

use crate::error::VaporError;

#[derive(Debug)]
pub struct SesIdentityInfo {
    pub identity: String,
    pub identity_type: Option<String>,
    pub sending_enabled: bool,
    pub dkim_signing_enabled: Option<bool>,
    pub dkim_status: Option<String>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct SesConfigSetDetail {
    pub name: String,
    pub sending_enabled: Option<bool>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct SesAccountInfo {
    pub sending_enabled: bool,
    pub sending_quota: Option<f64>,
    pub max_send_rate: Option<f64>,
    pub sent_last_24_hours: Option<f64>,
}

pub struct SesClient {
    inner: aws_sdk_sesv2::Client,
}

impl SesClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_sesv2::Client::new(config),
        }
    }

    /// Lists SES email identities, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `ListEmailIdentities` has both `page_size` (max-results-equivalent,
    /// mq-class naming) and `next_token` (verified against pinned
    /// `aws-sdk-sesv2` 1.123.0's
    /// `operation/list_email_identities/_list_email_identities_input.rs`), so
    /// `limit` is capped on the request itself; `.into_paginator()` dropped
    /// since it hides the token (kinesis/translate pattern).
    pub async fn list_email_identities(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<SesIdentityInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_email_identities();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.page_size(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            for info in output.email_identities.unwrap_or_default() {
                items.push(SesIdentityInfo {
                    identity: info.identity_name.unwrap_or_default(),
                    identity_type: info.identity_type.map(|t| t.as_str().to_string()),
                    sending_enabled: info.sending_enabled,
                    // DKIM details are only returned by get_email_identity, not the
                    // list_email_identities summary (IdentityInfo).
                    dkim_signing_enabled: None,
                    dkim_status: None,
                    tags: vec![],
                });
            }

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn get_email_identity(
        &self,
        identity: String,
    ) -> Result<Option<SesIdentityInfo>, VaporError> {
        let output = match self
            .inner
            .get_email_identity()
            .email_identity(&identity)
            .send()
            .await
        {
            Ok(o) => o,
            Err(e) => {
                if e.code() == Some("NotFoundException") {
                    return Ok(None);
                }
                return Err(crate::error::sdk_err(e));
            }
        };

        let tags: Vec<(String, String)> = output
            .tags()
            .iter()
            .map(|t| (t.key().to_string(), t.value().to_string()))
            .collect();

        Ok(Some(SesIdentityInfo {
            identity,
            identity_type: output.identity_type().map(|t| t.as_str().to_string()),
            // `GetEmailIdentityOutput` has no field equivalent to `IdentityInfo.sending_enabled`
            // (the one `list_email_identities` uses below) — `verified_for_sending_status` is
            // the closest available proxy but reflects identity verification, not whether
            // sending is currently suspended. See "Known limitation" in specs/ses.md.
            sending_enabled: output.verified_for_sending_status(),
            dkim_signing_enabled: output.dkim_attributes().map(|d| d.signing_enabled()),
            dkim_status: output
                .dkim_attributes()
                .and_then(|d| d.status())
                .map(|s| s.as_str().to_string()),
            tags,
        }))
    }

    /// Lists SES configuration sets, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListConfigurationSets`
    /// has both `page_size` and `next_token` (verified against pinned
    /// `aws-sdk-sesv2` 1.123.0), same pattern as `list_email_identities` above.
    /// The N+1 `get_configuration_set` fan-out stays scoped to one page
    /// (mq/control_tower precedent).
    pub async fn list_configuration_sets(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<SesConfigSetDetail>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_configuration_sets();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.page_size(l - names.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            names.extend(output.configuration_sets.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if names.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut result = Vec::with_capacity(names.len());
        for name in names {
            let detail = self
                .inner
                .get_configuration_set()
                .configuration_set_name(&name)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            let sending_enabled = detail.sending_options().map(|s| s.sending_enabled());
            let tags: Vec<(String, String)> = detail
                .tags()
                .iter()
                .map(|t| (t.key().to_string(), t.value().to_string()))
                .collect();
            result.push(SesConfigSetDetail {
                name,
                sending_enabled,
                tags,
            });
        }

        Ok((result, token))
    }

    /// Lists SES email templates, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListEmailTemplates`
    /// has both `page_size` and `next_token` (verified against pinned
    /// `aws-sdk-sesv2` 1.123.0), same pattern as `list_email_identities` above.
    pub async fn list_email_templates(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_sesv2::types::EmailTemplateMetadata>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_email_templates();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.page_size(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            items.extend(output.templates_metadata.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists SES suppressed destinations, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListSuppressedDestinations` has both `page_size` and `next_token`
    /// (verified against pinned `aws-sdk-sesv2` 1.123.0); the reasons/date
    /// filters are rebuilt each loop iteration so they're reapplied per page
    /// (fsx/transcribe/step_functions/translate precedent).
    pub async fn list_suppressed_destinations(
        &self,
        reasons: Option<Vec<String>>,
        start_date: Option<String>,
        end_date: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_sesv2::types::SuppressedDestinationSummary>,
            Option<String>,
        ),
        VaporError,
    > {
        let reason_vals: Option<Vec<aws_sdk_sesv2::types::SuppressionListReason>> =
            reasons.as_ref().map(|rs| {
                rs.iter()
                    .map(|r| aws_sdk_sesv2::types::SuppressionListReason::from(r.as_str()))
                    .collect()
            });
        let start_dt = start_date.as_deref().and_then(parse_datetime);
        let end_dt = end_date.as_deref().and_then(parse_datetime);

        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_suppressed_destinations();
            if let Some(ref reasons_list) = reason_vals {
                req = req.set_reasons(Some(reasons_list.clone()));
            }
            if let Some(dt) = start_dt {
                req = req.start_date(dt);
            }
            if let Some(dt) = end_dt {
                req = req.end_date(dt);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.page_size(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            items.extend(output.suppressed_destination_summaries.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn get_account(&self) -> Result<SesAccountInfo, VaporError> {
        let output = self
            .inner
            .get_account()
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        let quota = output.send_quota();
        Ok(SesAccountInfo {
            sending_enabled: output.sending_enabled(),
            sending_quota: quota.map(|q| q.max24_hour_send()),
            max_send_rate: quota.map(|q| q.max_send_rate()),
            sent_last_24_hours: quota.map(|q| q.sent_last24_hours()),
        })
    }
}

fn parse_datetime(s: &str) -> Option<aws_sdk_sesv2::primitives::DateTime> {
    let dt = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    Some(aws_sdk_sesv2::primitives::DateTime::from_secs_and_nanos(
        dt.timestamp(),
        dt.timestamp_subsec_nanos(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://email.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_email_identities_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/identities"), ""),
            json_response(
                200,
                r#"{"EmailIdentities":[{"IdentityType":"EMAIL_ADDRESS","IdentityName":"alice@example.com","SendingEnabled":true,"VerificationStatus":"SUCCESS"},{"IdentityType":"DOMAIN","IdentityName":"example.com","SendingEnabled":false,"VerificationStatus":"PENDING"}]}"#,
            ),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_email_identities(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let a1 = &items[0];
        assert_eq!(a1.identity, "alice@example.com");
        assert_eq!(a1.identity_type, Some("EMAIL_ADDRESS".to_string()));
        assert!(a1.sending_enabled);
        // list_email_identities' `IdentityInfo` summary has no DKIM fields at
        // all (only get_email_identity's `DkimAttributes` does) — always None.
        assert_eq!(a1.dkim_signing_enabled, None);
        assert_eq!(a1.dkim_status, None);
        assert!(a1.tags.is_empty());

        let a2 = &items[1];
        assert_eq!(a2.identity, "example.com");
        assert!(!a2.sending_enabled);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_email_identities_resumes_from_provided_next_token() {
        // `ListEmailIdentities` uses literal PascalCase query keys
        // (`NextToken`/`PageSize`), not kebab-case like quicksight.rs.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v2/email/identities?NextToken=cursor-a"),
                "",
            ),
            json_response(
                200,
                r#"{"EmailIdentities":[{"IdentityName":"carol@example.com"}]}"#,
            ),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_email_identities(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_email_identities_caps_at_limit_across_pages() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/identities?PageSize=3"), ""),
                json_response(
                    200,
                    r#"{"EmailIdentities":[{"IdentityName":"i1"},{"IdentityName":"i2"}],"NextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v2/email/identities?NextToken=cursor-b&PageSize=1"),
                    "",
                ),
                json_response(200, r#"{"EmailIdentities":[{"IdentityName":"i3"}]}"#),
            ),
        ]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_email_identities(Some(3), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_email_identities_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/identities"), ""),
            json_error_response("BadRequestException", "invalid parameter"),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let err = client.list_email_identities(None, None).await.unwrap_err();

        assert!(format!("{err:?}").contains("invalid parameter"));
    }

    #[tokio::test]
    async fn get_email_identity_returns_identity_details() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/identities/example.com"), ""),
            json_response(
                200,
                r#"{"IdentityType":"DOMAIN","FeedbackForwardingStatus":true,"VerifiedForSendingStatus":true,"DkimAttributes":{"SigningEnabled":true,"Status":"SUCCESS"},"Tags":[{"Key":"env","Value":"prod"}]}"#,
            ),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let identity = client
            .get_email_identity("example.com".to_string())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(identity.identity, "example.com");
        assert_eq!(identity.identity_type, Some("DOMAIN".to_string()));
        // `GetEmailIdentityOutput` has no `sending_enabled`-equivalent field —
        // the wrapper substitutes `verified_for_sending_status` (see the
        // "Known limitation" doc comment on `get_email_identity`).
        assert!(identity.sending_enabled);
        assert_eq!(identity.dkim_signing_enabled, Some(true));
        assert_eq!(identity.dkim_status, Some("SUCCESS".to_string()));
        assert_eq!(identity.tags, vec![("env".to_string(), "prod".to_string())]);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_email_identity_handles_missing_dkim_and_tags() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/identities/example.com"), ""),
            json_response(200, r#"{"IdentityType":"DOMAIN"}"#),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let identity = client
            .get_email_identity("example.com".to_string())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(identity.dkim_signing_enabled, None);
        assert_eq!(identity.dkim_status, None);
        assert!(identity.tags.is_empty());
        // No `*_correct_errors` fn touches `GetEmailIdentityOutput`'s bare
        // `bool` fields, so a missing key deserializes via `unwrap_or_default`
        // at the builder level, not a synthesized non-zero default.
        assert!(!identity.sending_enabled);
    }

    #[tokio::test]
    async fn get_email_identity_returns_none_on_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v2/email/identities/missing.example.com"),
                "",
            ),
            json_error_response("NotFoundException", "no such identity"),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let result = client
            .get_email_identity("missing.example.com".to_string())
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_email_identity_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/identities/example.com"), ""),
            json_error_response("BadRequestException", "malformed identity"),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_email_identity("example.com".to_string())
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("malformed identity"));
    }

    #[tokio::test]
    async fn list_configuration_sets_lists_all_and_fans_out_details() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/configuration-sets"), ""),
                json_response(200, r#"{"ConfigurationSets":["set-a","set-b"]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/configuration-sets/set-a"), ""),
                json_response(
                    200,
                    r#"{"SendingOptions":{"SendingEnabled":true},"Tags":[{"Key":"k","Value":"v"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/configuration-sets/set-b"), ""),
                json_response(200, r#"{"SendingOptions":{"SendingEnabled":false}}"#),
            ),
        ]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_configuration_sets(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "set-a");
        assert_eq!(items[0].sending_enabled, Some(true));
        assert_eq!(items[0].tags, vec![("k".to_string(), "v".to_string())]);
        assert_eq!(items[1].name, "set-b");
        assert_eq!(items[1].sending_enabled, Some(false));
        assert!(items[1].tags.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configuration_sets_caps_at_limit_and_fans_out_only_within_cap() {
        // The N+1 `get_configuration_set` fan-out stays scoped to one page
        // (mq/control_tower precedent, per the doc comment on
        // `list_configuration_sets`) — only 1 name is discovered when
        // `limit(1)` is satisfied by page 1, so only 1 fan-out call happens.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v2/email/configuration-sets?PageSize=1"),
                    "",
                ),
                json_response(
                    200,
                    r#"{"ConfigurationSets":["set-a"],"NextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/configuration-sets/set-a"), ""),
                json_response(200, r#"{"SendingOptions":{"SendingEnabled":true}}"#),
            ),
        ]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.list_configuration_sets(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configuration_sets_propagates_list_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/configuration-sets"), ""),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_configuration_sets(None, None)
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("bad request"));
    }

    #[tokio::test]
    async fn list_configuration_sets_propagates_fan_out_errors() {
        // Unlike some N+1 fan-outs in this sweep that swallow per-item
        // errors via `.ok()` (gotcha 10), `get_configuration_set` here uses
        // `?`, so a fan-out failure aborts the whole call.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/configuration-sets"), ""),
                json_response(200, r#"{"ConfigurationSets":["set-a"]}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/configuration-sets/set-a"), ""),
                json_error_response("BadRequestException", "no such configuration set"),
            ),
        ]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_configuration_sets(None, None)
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("no such configuration set"));
    }

    #[tokio::test]
    async fn list_email_templates_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/templates"), ""),
            json_response(
                200,
                r#"{"TemplatesMetadata":[{"TemplateName":"welcome","CreatedTimestamp":1700000000},{"TemplateName":"digest"}]}"#,
            ),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_email_templates(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].template_name(), Some("welcome"));
        assert!(items[0].created_timestamp().is_some());
        // No `*_correct_errors` fn touches `EmailTemplateMetadata` — a
        // missing `CreatedTimestamp` genuinely stays `None`.
        assert_eq!(items[1].template_name(), Some("digest"));
        assert_eq!(items[1].created_timestamp(), None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_email_templates_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/templates?NextToken=cursor-a"), ""),
            json_response(200, r#"{"TemplatesMetadata":[{"TemplateName":"t3"}]}"#),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_email_templates(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_email_templates_caps_at_limit_across_pages() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/templates?PageSize=2"), ""),
                json_response(
                    200,
                    r#"{"TemplatesMetadata":[{"TemplateName":"t1"}],"NextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v2/email/templates?NextToken=cursor-b&PageSize=1"),
                    "",
                ),
                json_response(200, r#"{"TemplatesMetadata":[{"TemplateName":"t2"}]}"#),
            ),
        ]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_email_templates(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_email_templates_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/templates"), ""),
            json_error_response("BadRequestException", "bad template request"),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let err = client.list_email_templates(None, None).await.unwrap_err();

        assert!(format!("{err:?}").contains("bad template request"));
    }

    #[tokio::test]
    async fn list_suppressed_destinations_lists_all_and_handles_missing_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/suppression/addresses"), ""),
            json_response(
                200,
                r#"{"SuppressedDestinationSummaries":[{"EmailAddress":"a@example.com","Reason":"BOUNCE","LastUpdateTime":1700000000},{}]}"#,
            ),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_suppressed_destinations(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].email_address(), "a@example.com");
        assert_eq!(items[0].reason().as_str(), "BOUNCE");

        // `suppressed_destination_summary_correct_errors` default-fills a
        // fully-missing object: empty-string email, an unknown
        // `"no value was set"` reason variant (gotcha 20), and epoch-0
        // `last_update_time` — never `None`, since these accessors are bare
        // (non-`Option`) types (gotcha 16).
        let defaulted = &items[1];
        assert_eq!(defaulted.email_address(), "");
        assert_eq!(defaulted.reason().as_str(), "no value was set");
        assert_eq!(defaulted.last_update_time().secs(), 0);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_suppressed_destinations_filters_by_reasons_and_dates() {
        // Query key order confirmed from the pinned SDK's `uri_query`:
        // TenantName, Reason (repeated), StartDate, EndDate, NextToken,
        // PageSize. Timestamps use `Format::DateTime` (RFC-3339), and `:` is
        // percent-encoded in query values (`aws_smithy_http::query::fmt_string`).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!(
                    "{BASE}/v2/email/suppression/addresses?Reason=BOUNCE&Reason=COMPLAINT&StartDate=2024-01-01T00%3A00%3A00Z&EndDate=2024-02-01T00%3A00%3A00Z"
                ),
                "",
            ),
            json_response(200, r#"{"SuppressedDestinationSummaries":[]}"#),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .list_suppressed_destinations(
                Some(vec!["BOUNCE".to_string(), "COMPLAINT".to_string()]),
                Some("2024-01-01T00:00:00Z".to_string()),
                Some("2024-02-01T00:00:00Z".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert!(items.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_suppressed_destinations_caps_at_limit_across_pages() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v2/email/suppression/addresses?PageSize=2"),
                    "",
                ),
                json_response(
                    200,
                    r#"{"SuppressedDestinationSummaries":[{"EmailAddress":"a@example.com","Reason":"BOUNCE","LastUpdateTime":1700000000}],"NextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v2/email/suppression/addresses?NextToken=cursor-b&PageSize=1"),
                    "",
                ),
                json_response(
                    200,
                    r#"{"SuppressedDestinationSummaries":[{"EmailAddress":"b@example.com","Reason":"COMPLAINT","LastUpdateTime":1700000001}]}"#,
                ),
            ),
        ]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_suppressed_destinations(None, None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_suppressed_destinations_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/suppression/addresses"), ""),
            json_error_response("BadRequestException", "bad suppression request"),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_suppressed_destinations(None, None, None, None, None)
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("bad suppression request"));
    }

    #[tokio::test]
    async fn get_account_returns_account_info() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/account"), ""),
            json_response(
                200,
                r#"{"SendingEnabled":true,"SendQuota":{"Max24HourSend":50000.0,"MaxSendRate":14.0,"SentLast24Hours":120.0}}"#,
            ),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let account = client.get_account().await.unwrap();

        assert!(account.sending_enabled);
        assert_eq!(account.sending_quota, Some(50000.0));
        assert_eq!(account.max_send_rate, Some(14.0));
        assert_eq!(account.sent_last_24_hours, Some(120.0));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_account_handles_missing_send_quota() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/account"), ""),
            json_response(200, r#"{"SendingEnabled":false}"#),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let account = client.get_account().await.unwrap();

        assert!(!account.sending_enabled);
        assert_eq!(account.sending_quota, None);
        assert_eq!(account.max_send_rate, None);
        assert_eq!(account.sent_last_24_hours, None);
    }

    #[tokio::test]
    async fn get_account_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/account"), ""),
            json_error_response("BadRequestException", "account lookup failed"),
        )]);
        let client = SesClient::new(&sdk_config(http_client.clone()));

        let err = client.get_account().await.unwrap_err();

        assert!(format!("{err:?}").contains("account lookup failed"));
    }
}
