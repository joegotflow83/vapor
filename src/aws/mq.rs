use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct MqBrokerInstanceInfo {
    pub console_url: Option<String>,
    pub endpoints: Vec<String>,
    pub ip_address: Option<String>,
}

pub struct MqBrokerInfo {
    pub broker_id: Option<String>,
    pub broker_arn: Option<String>,
    pub broker_name: Option<String>,
    pub broker_state: Option<String>,
    pub engine_type: Option<String>,
    pub engine_version: Option<String>,
    pub deployment_mode: Option<String>,
    pub host_instance_type: Option<String>,
    pub publicly_accessible: Option<bool>,
    pub broker_instances: Vec<MqBrokerInstanceInfo>,
    pub subnet_ids: Vec<String>,
    pub security_groups: Vec<String>,
    pub tags: Vec<(String, String)>,
}

pub struct MqConfigurationInfo {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub engine_type: Option<String>,
    pub engine_version: Option<String>,
    pub description: Option<String>,
    pub latest_revision: Option<i32>,
    pub created: Option<String>,
    pub tags: Vec<(String, String)>,
}

pub struct MqClient {
    inner: aws_sdk_mq::Client,
}

impl MqClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_mq::Client::new(config),
        }
    }

    pub async fn list_brokers(&self) -> Result<Vec<MqBrokerInfo>, VaporError> {
        let mut ids: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_brokers();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for summary in output.broker_summaries() {
                if let Some(id) = summary.broker_id() {
                    ids.push(id.to_string());
                }
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        let mut brokers = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(broker) = self.describe_broker(&id).await? {
                brokers.push(broker);
            }
        }
        Ok(brokers)
    }

    pub async fn describe_broker(&self, broker_id: &str) -> Result<Option<MqBrokerInfo>, VaporError> {
        let output = match self.inner.describe_broker().broker_id(broker_id).send().await {
            Ok(o) => o,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("NotFoundException") || msg.contains("ResourceNotFoundException") {
                    return Ok(None);
                }
                return Err(VaporError::AwsSdk(msg));
            }
        };

        let broker_instances = output
            .broker_instances()
            .iter()
            .map(|bi| MqBrokerInstanceInfo {
                console_url: bi.console_url().map(|s| s.to_string()),
                endpoints: bi.endpoints().to_vec(),
                ip_address: bi.ip_address().map(|s| s.to_string()),
            })
            .collect();

        let tags = output
            .tags()
            .map(|t| t.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        Ok(Some(MqBrokerInfo {
            broker_id: output.broker_id().map(|s| s.to_string()),
            broker_arn: output.broker_arn().map(|s| s.to_string()),
            broker_name: output.broker_name().map(|s| s.to_string()),
            broker_state: output.broker_state().map(|s| s.as_str().to_string()),
            engine_type: output.engine_type().map(|s| s.as_str().to_string()),
            engine_version: output.engine_version().map(|s| s.to_string()),
            deployment_mode: output.deployment_mode().map(|s| s.as_str().to_string()),
            host_instance_type: output.host_instance_type().map(|s| s.to_string()),
            publicly_accessible: output.publicly_accessible(),
            broker_instances,
            subnet_ids: output.subnet_ids().to_vec(),
            security_groups: output.security_groups().to_vec(),
            tags,
        }))
    }

    pub async fn list_configurations(&self) -> Result<Vec<MqConfigurationInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_configurations();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for cfg in output.configurations() {
                let tags = cfg
                    .tags()
                    .map(|t| t.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                items.push(MqConfigurationInfo {
                    id: cfg.id().map(|s| s.to_string()),
                    arn: cfg.arn().map(|s| s.to_string()),
                    name: cfg.name().map(|s| s.to_string()),
                    engine_type: cfg.engine_type().map(|s| s.as_str().to_string()),
                    engine_version: cfg.engine_version().map(|s| s.to_string()),
                    description: cfg.description().map(|s| s.to_string()),
                    latest_revision: cfg.latest_revision().and_then(|r| r.revision()),
                    created: cfg.created().map(|d| d.to_string()),
                    tags,
                });
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
