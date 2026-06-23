use aws_config::SdkConfig;
use aws_sdk_health::types::{Event, EventFilter, EventStatusCode};

use crate::error::VaporError;

pub struct HealthClient {
    inner: aws_sdk_health::Client,
}

impl HealthClient {
    pub fn new(config: &SdkConfig) -> Self {
        let health_config = aws_sdk_health::config::Builder::from(config)
            .region(aws_sdk_health::config::Region::new("us-east-1"))
            .build();
        Self {
            inner: aws_sdk_health::Client::from_conf(health_config),
        }
    }

    pub async fn describe_events(
        &self,
        status_codes: Option<Vec<String>>,
        services: Option<Vec<String>>,
    ) -> Result<Vec<Event>, VaporError> {
        let parsed_codes: Vec<EventStatusCode> = status_codes
            .unwrap_or_default()
            .iter()
            .filter_map(|s| match s.as_str() {
                "open" => Some(EventStatusCode::Open),
                "closed" => Some(EventStatusCode::Closed),
                "upcoming" => Some(EventStatusCode::Upcoming),
                _ => None,
            })
            .collect();

        let services = services.unwrap_or_default();

        let mut events = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut fb = EventFilter::builder();
            if !parsed_codes.is_empty() {
                fb = fb.set_event_status_codes(Some(parsed_codes.clone()));
            }
            if !services.is_empty() {
                fb = fb.set_services(Some(services.clone()));
            }

            let mut req = self.inner.describe_events().filter(fb.build());

            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            events.extend(output.events().to_vec());

            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(events)
    }
}
