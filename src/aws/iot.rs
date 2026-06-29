use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct IotTagPair {
    pub key: String,
    pub value: String,
}

pub struct IotThingInfo {
    pub thing_name: Option<String>,
    pub thing_arn: Option<String>,
    pub thing_type_name: Option<String>,
    pub attributes: Vec<IotTagPair>,
    pub version: Option<i64>,
}

pub struct IotThingGroupInfo {
    pub group_name: Option<String>,
    pub group_arn: Option<String>,
    pub group_id: Option<String>,
    pub status: Option<String>,
}

pub struct IotPolicyInfo {
    pub policy_name: Option<String>,
    pub policy_arn: Option<String>,
}

pub struct IotCertificateInfo {
    pub certificate_id: Option<String>,
    pub certificate_arn: Option<String>,
    pub status: Option<String>,
    pub creation_date: Option<String>,
}

pub struct IotTopicRuleInfo {
    pub rule_name: Option<String>,
    pub topic_pattern: Option<String>,
    pub created_at: Option<String>,
    pub rule_disabled: Option<bool>,
    pub rule_arn: Option<String>,
}

pub struct IotClient {
    inner: aws_sdk_iot::Client,
}

impl IotClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_iot::Client::new(config),
        }
    }

    pub async fn list_things(
        &self,
        thing_type_name: Option<String>,
        attribute_name: Option<String>,
        attribute_value: Option<String>,
    ) -> Result<Vec<IotThingInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_things();
            if let Some(ref t) = thing_type_name {
                req = req.thing_type_name(t);
            }
            if let Some(ref n) = attribute_name {
                req = req.attribute_name(n);
            }
            if let Some(ref v) = attribute_value {
                req = req.attribute_value(v);
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for thing in output.things() {
                let attributes: Vec<IotTagPair> = thing
                    .attributes()
                    .map(|m| {
                        m.iter()
                            .map(|(k, v)| IotTagPair {
                                key: k.clone(),
                                value: v.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                items.push(IotThingInfo {
                    thing_name: thing.thing_name().map(|s| s.to_string()),
                    thing_arn: thing.thing_arn().map(|s| s.to_string()),
                    thing_type_name: thing.thing_type_name().map(|s| s.to_string()),
                    attributes,
                    version: Some(thing.version()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_thing_groups(
        &self,
        parent_group: Option<String>,
    ) -> Result<Vec<IotThingGroupInfo>, VaporError> {
        let mut groups: Vec<(Option<String>, Option<String>)> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_thing_groups();
            if let Some(ref pg) = parent_group {
                req = req.parent_group(pg);
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for g in output.thing_groups() {
                groups.push((
                    g.group_name().map(|s| s.to_string()),
                    g.group_arn().map(|s| s.to_string()),
                ));
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        let mut items = Vec::new();
        for (group_name, group_arn) in groups {
            let (group_id, status) = if let Some(ref name) = group_name {
                let result = self
                    .inner
                    .describe_thing_group()
                    .thing_group_name(name)
                    .send()
                    .await;
                match result {
                    Ok(output) => (
                        output.thing_group_id().map(|s| s.to_string()),
                        output.status().map(|s| s.as_str().to_string()),
                    ),
                    Err(_) => (None, None),
                }
            } else {
                (None, None)
            };
            items.push(IotThingGroupInfo {
                group_name,
                group_arn,
                group_id,
                status,
            });
        }

        Ok(items)
    }

    pub async fn list_policies(&self) -> Result<Vec<IotPolicyInfo>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_policies();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for policy in output.policies() {
                items.push(IotPolicyInfo {
                    policy_name: policy.policy_name().map(|s| s.to_string()),
                    policy_arn: policy.policy_arn().map(|s| s.to_string()),
                });
            }

            match output.next_marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_certificates(
        &self,
        ascending_order: Option<bool>,
    ) -> Result<Vec<IotCertificateInfo>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_certificates();
            if let Some(asc) = ascending_order {
                req = req.ascending_order(asc);
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for cert in output.certificates() {
                items.push(IotCertificateInfo {
                    certificate_id: cert.certificate_id().map(|s| s.to_string()),
                    certificate_arn: cert.certificate_arn().map(|s| s.to_string()),
                    status: cert.status().map(|s| s.as_str().to_string()),
                    creation_date: cert.creation_date().map(|t| t.to_string()),
                });
            }

            match output.next_marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_topic_rules(
        &self,
        topic_rule_disabled: Option<bool>,
    ) -> Result<Vec<IotTopicRuleInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_topic_rules();
            if let Some(disabled) = topic_rule_disabled {
                req = req.rule_disabled(disabled);
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for rule in output.rules() {
                items.push(IotTopicRuleInfo {
                    rule_name: rule.rule_name().map(|s| s.to_string()),
                    topic_pattern: rule.topic_pattern().map(|s| s.to_string()),
                    created_at: rule.created_at().map(|t| t.to_string()),
                    rule_disabled: rule.rule_disabled(),
                    rule_arn: rule.rule_arn().map(|s| s.to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
