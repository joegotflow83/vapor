use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct SesIdentityInfo {
    pub identity: String,
    pub identity_type: Option<String>,
    pub sending_enabled: bool,
    pub dkim_signing_enabled: Option<bool>,
    pub dkim_status: Option<String>,
    pub tags: Vec<(String, String)>,
}

pub struct SesConfigSetDetail {
    pub name: String,
    pub sending_enabled: Option<bool>,
    pub tags: Vec<(String, String)>,
}

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

    pub async fn list_email_identities(
        &self,
        page_size: Option<i32>,
    ) -> Result<Vec<SesIdentityInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_email_identities();
            if let Some(size) = page_size {
                req = req.page_size(size);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for info in output.email_identities() {
                items.push(SesIdentityInfo {
                    identity: info.identity_name().unwrap_or_default().to_string(),
                    identity_type: info.identity_type().map(|t| t.as_str().to_string()),
                    sending_enabled: info.sending_enabled(),
                    // DKIM details are only returned by get_email_identity, not the
                    // list_email_identities summary (IdentityInfo).
                    dkim_signing_enabled: None,
                    dkim_status: None,
                    tags: vec![],
                });
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
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
                let msg = e.to_string();
                if msg.contains("NotFoundException") || msg.contains("Not Found") {
                    return Ok(None);
                }
                return Err(VaporError::AwsSdk(msg));
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
            sending_enabled: output.verified_for_sending_status(),
            dkim_signing_enabled: output.dkim_attributes().map(|d| d.signing_enabled()),
            dkim_status: output
                .dkim_attributes()
                .and_then(|d| d.status())
                .map(|s| s.as_str().to_string()),
            tags,
        }))
    }

    pub async fn list_configuration_sets(&self) -> Result<Vec<SesConfigSetDetail>, VaporError> {
        let mut names = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_configuration_sets();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            names.extend(output.configuration_sets().iter().map(|s| s.to_string()));
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
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
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
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

        Ok(result)
    }

    pub async fn list_email_templates(
        &self,
    ) -> Result<Vec<aws_sdk_sesv2::types::EmailTemplateMetadata>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_email_templates();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.templates_metadata().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_suppressed_destinations(
        &self,
        reasons: Option<Vec<String>>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<Vec<aws_sdk_sesv2::types::SuppressedDestinationSummary>, VaporError> {
        let reason_vals: Option<Vec<aws_sdk_sesv2::types::SuppressionListReason>> =
            reasons.map(|rs| {
                rs.iter()
                    .map(|r| aws_sdk_sesv2::types::SuppressionListReason::from(r.as_str()))
                    .collect()
            });
        let start_dt = start_date.as_deref().and_then(parse_datetime);
        let end_dt = end_date.as_deref().and_then(parse_datetime);

        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

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
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.suppressed_destination_summaries().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_account(&self) -> Result<SesAccountInfo, VaporError> {
        let output = self
            .inner
            .get_account()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
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
