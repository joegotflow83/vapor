use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct PinpointAppInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub arn: Option<String>,
    pub creation_date: Option<String>,
    pub tags: Vec<(String, String)>,
}

pub struct PinpointCampaignInfo {
    pub id: Option<String>,
    pub application_id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub creation_date: Option<String>,
    pub last_modified_date: Option<String>,
}

pub struct PinpointSegmentInfo {
    pub id: Option<String>,
    pub application_id: Option<String>,
    pub name: Option<String>,
    pub segment_type: Option<String>,
    pub creation_date: Option<String>,
    pub last_modified_date: Option<String>,
}

pub struct PinpointClient {
    inner: aws_sdk_pinpoint::Client,
}

impl PinpointClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_pinpoint::Client::new(config),
        }
    }

    pub async fn get_apps(&self) -> Result<Vec<PinpointAppInfo>, VaporError> {
        let mut items = Vec::new();
        let mut token: Option<String> = None;

        loop {
            let mut req = self.inner.get_apps().page_size("100");
            if let Some(ref tok) = token {
                req = req.token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            let apps_response = match output.applications_response() {
                Some(r) => r,
                None => break,
            };

            for app in apps_response.item() {
                items.push(PinpointAppInfo {
                    id: app.id().map(|s| s.to_string()),
                    name: app.name().map(|s| s.to_string()),
                    arn: app.arn().map(|s| s.to_string()),
                    creation_date: app.creation_date().map(|s| s.to_string()),
                    tags: app
                        .tags()
                        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default(),
                });
            }

            match apps_response.next_token() {
                Some(tok) if !tok.is_empty() => token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_campaigns(
        &self,
        application_id: &str,
    ) -> Result<Vec<PinpointCampaignInfo>, VaporError> {
        let mut items = Vec::new();
        let mut token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .get_campaigns()
                .application_id(application_id)
                .page_size("100");
            if let Some(ref tok) = token {
                req = req.token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            let campaigns_response = match output.campaigns_response() {
                Some(r) => r,
                None => break,
            };

            for campaign in campaigns_response.item() {
                items.push(PinpointCampaignInfo {
                    id: campaign.id().map(|s| s.to_string()),
                    application_id: campaign.application_id().map(|s| s.to_string()),
                    name: campaign.name().map(|s| s.to_string()),
                    status: campaign
                        .state()
                        .and_then(|s| s.campaign_status())
                        .map(|s| s.as_str().to_string()),
                    creation_date: campaign.creation_date().map(|s| s.to_string()),
                    last_modified_date: campaign.last_modified_date().map(|s| s.to_string()),
                });
            }

            match campaigns_response.next_token() {
                Some(tok) if !tok.is_empty() => token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_segments(
        &self,
        application_id: &str,
    ) -> Result<Vec<PinpointSegmentInfo>, VaporError> {
        let mut items = Vec::new();
        let mut token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .get_segments()
                .application_id(application_id)
                .page_size("100");
            if let Some(ref tok) = token {
                req = req.token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            let segments_response = match output.segments_response() {
                Some(r) => r,
                None => break,
            };

            for segment in segments_response.item() {
                items.push(PinpointSegmentInfo {
                    id: segment.id().map(|s| s.to_string()),
                    application_id: segment.application_id().map(|s| s.to_string()),
                    name: segment.name().map(|s| s.to_string()),
                    segment_type: segment.segment_type().map(|t| t.as_str().to_string()),
                    creation_date: segment.creation_date().map(|s| s.to_string()),
                    last_modified_date: segment.last_modified_date().map(|s| s.to_string()),
                });
            }

            match segments_response.next_token() {
                Some(tok) if !tok.is_empty() => token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
