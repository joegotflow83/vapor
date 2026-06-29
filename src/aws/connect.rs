use aws_config::SdkConfig;
use aws_sdk_connect::types::{ContactFlowType, QueueType};

use crate::error::VaporError;

pub struct ConnectInstanceInfo {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub identity_management_type: Option<String>,
    pub instance_alias: Option<String>,
    pub created_time: Option<String>,
    pub service_role: Option<String>,
    pub instance_status: Option<String>,
    pub inbound_calls_enabled: Option<bool>,
    pub outbound_calls_enabled: Option<bool>,
}

pub struct ConnectQueueInfo {
    pub queue_id: Option<String>,
    pub queue_arn: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub queue_type: Option<String>,
    pub status: Option<String>,
}

pub struct ConnectContactFlowInfo {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub contact_flow_type: Option<String>,
    pub description: Option<String>,
}

pub struct ConnectUserInfo {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub username: Option<String>,
    pub routing_profile_id: Option<String>,
    pub hierarchy_group_id: Option<String>,
    pub security_profile_ids: Vec<String>,
}

pub struct ConnectClient {
    inner: aws_sdk_connect::Client,
}

impl ConnectClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_connect::Client::new(config),
        }
    }

    pub async fn list_instances(&self) -> Result<Vec<ConnectInstanceInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_instances();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for inst in output.instance_summary_list() {
                items.push(ConnectInstanceInfo {
                    id: inst.id().map(|s| s.to_string()),
                    arn: inst.arn().map(|s| s.to_string()),
                    identity_management_type: inst
                        .identity_management_type()
                        .map(|t| t.as_str().to_string()),
                    instance_alias: inst.instance_alias().map(|s| s.to_string()),
                    created_time: inst.created_time().map(|t| t.to_string()),
                    service_role: inst.service_role().map(|s| s.to_string()),
                    instance_status: inst.instance_status().map(|s| s.as_str().to_string()),
                    inbound_calls_enabled: inst.inbound_calls_enabled(),
                    outbound_calls_enabled: inst.outbound_calls_enabled(),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_queues(
        &self,
        instance_id: &str,
        queue_types: Option<Vec<String>>,
    ) -> Result<Vec<ConnectQueueInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        let sdk_types: Option<Vec<QueueType>> = queue_types.map(|types| {
            types
                .iter()
                .map(|s| QueueType::from(s.as_str()))
                .collect()
        });

        loop {
            let mut req = self.inner.list_queues().instance_id(instance_id);
            if let Some(ref types) = sdk_types {
                req = req.set_queue_types(Some(types.clone()));
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for queue in output.queue_summary_list() {
                let queue_id = queue.id().map(|s| s.to_string());

                let (description, status) = if let Some(ref qid) = queue_id {
                    match self
                        .inner
                        .describe_queue()
                        .instance_id(instance_id)
                        .queue_id(qid)
                        .send()
                        .await
                        .ok()
                        .and_then(|o| o.queue().cloned())
                    {
                        Some(q) => (
                            q.description().map(|s| s.to_string()),
                            q.status().map(|s| s.as_str().to_string()),
                        ),
                        None => (None, None),
                    }
                } else {
                    (None, None)
                };

                items.push(ConnectQueueInfo {
                    queue_id,
                    queue_arn: queue.arn().map(|s| s.to_string()),
                    name: queue.name().map(|s| s.to_string()),
                    description,
                    queue_type: queue.queue_type().map(|t| t.as_str().to_string()),
                    status,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_contact_flows(
        &self,
        instance_id: &str,
        contact_flow_types: Option<Vec<String>>,
    ) -> Result<Vec<ConnectContactFlowInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        let sdk_types: Option<Vec<ContactFlowType>> = contact_flow_types.map(|types| {
            types
                .iter()
                .map(|s| ContactFlowType::from(s.as_str()))
                .collect()
        });

        loop {
            let mut req = self.inner.list_contact_flows().instance_id(instance_id);
            if let Some(ref types) = sdk_types {
                req = req.set_contact_flow_types(Some(types.clone()));
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for flow in output.contact_flow_summary_list() {
                let flow_id = flow.id().map(|s| s.to_string());

                let description = if let Some(ref fid) = flow_id {
                    self.inner
                        .describe_contact_flow()
                        .instance_id(instance_id)
                        .contact_flow_id(fid)
                        .send()
                        .await
                        .ok()
                        .and_then(|o| o.contact_flow().cloned())
                        .and_then(|cf| cf.description().map(|s| s.to_string()))
                } else {
                    None
                };

                items.push(ConnectContactFlowInfo {
                    id: flow_id,
                    arn: flow.arn().map(|s| s.to_string()),
                    name: flow.name().map(|s| s.to_string()),
                    contact_flow_type: flow.contact_flow_type().map(|t| t.as_str().to_string()),
                    description,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_users(
        &self,
        instance_id: &str,
    ) -> Result<Vec<ConnectUserInfo>, VaporError> {
        let mut summaries = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_users().instance_id(instance_id);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for user in output.user_summary_list() {
                summaries.push((
                    user.id().map(|s| s.to_string()),
                    user.arn().map(|s| s.to_string()),
                    user.username().map(|s| s.to_string()),
                ));
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        let mut items = Vec::new();
        for (id, arn, username) in summaries {
            let (routing_profile_id, hierarchy_group_id, security_profile_ids) =
                if let Some(ref uid) = id {
                    match self
                        .inner
                        .describe_user()
                        .user_id(uid)
                        .instance_id(instance_id)
                        .send()
                        .await
                        .ok()
                        .and_then(|o| o.user().cloned())
                    {
                        Some(user) => (
                            user.routing_profile_id().map(|s| s.to_string()),
                            user.hierarchy_group_id().map(|s| s.to_string()),
                            user.security_profile_ids().to_vec(),
                        ),
                        None => (None, None, Vec::new()),
                    }
                } else {
                    (None, None, Vec::new())
                };

            items.push(ConnectUserInfo {
                id,
                arn,
                username,
                routing_profile_id,
                hierarchy_group_id,
                security_profile_ids,
            });
        }

        Ok(items)
    }
}
