use aws_config::SdkConfig;
use aws_sdk_eventbridge::types::{EventBus, Rule, Target};

use crate::error::VaporError;

pub struct EventBridgeClient {
    inner: aws_sdk_eventbridge::Client,
}

impl EventBridgeClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_eventbridge::Client::new(config),
        }
    }

    pub async fn list_event_buses(&self) -> Result<Vec<EventBus>, VaporError> {
        let mut buses: Vec<EventBus> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_event_buses();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for b in output.event_buses() {
                buses.push(b.clone());
            }
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(buses)
    }

    pub async fn list_rules(
        &self,
        event_bus_name: Option<&str>,
    ) -> Result<Vec<Rule>, VaporError> {
        let mut rules: Vec<Rule> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_rules();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            if let Some(bus) = event_bus_name {
                req = req.event_bus_name(bus);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for r in output.rules() {
                rules.push(r.clone());
            }
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(rules)
    }

    pub async fn list_targets_by_rule(
        &self,
        rule_name: &str,
        event_bus_name: Option<&str>,
    ) -> Result<Vec<Target>, VaporError> {
        let mut targets: Vec<Target> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_targets_by_rule().rule(rule_name);
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            if let Some(bus) = event_bus_name {
                req = req.event_bus_name(bus);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for t in output.targets() {
                targets.push(t.clone());
            }
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(targets)
    }
}
