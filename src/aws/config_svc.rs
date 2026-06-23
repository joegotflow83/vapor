use aws_config::SdkConfig;
use aws_sdk_config as config_sdk;

use crate::error::VaporError;

pub struct AwsConfigClient {
    inner: config_sdk::Client,
}

impl AwsConfigClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: config_sdk::Client::new(config),
        }
    }

    pub async fn describe_config_rules(
        &self,
        names: Option<Vec<String>>,
    ) -> Result<Vec<config_sdk::types::ConfigRule>, VaporError> {
        let mut rules = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_config_rules();
            if let Some(ref n) = names {
                req = req.set_config_rule_names(Some(n.clone()));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            rules.extend(output.config_rules().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(rules)
    }

    pub async fn describe_compliance_by_config_rule(
        &self,
        rule_names: Option<Vec<String>>,
        compliance_types: Option<Vec<String>>,
    ) -> Result<Vec<config_sdk::types::ComplianceByConfigRule>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;

        let ct: Option<Vec<config_sdk::types::ComplianceType>> = compliance_types.map(|types| {
            types
                .iter()
                .filter_map(|s| s.parse::<config_sdk::types::ComplianceType>().ok())
                .collect()
        });

        loop {
            let mut req = self.inner.describe_compliance_by_config_rule();
            if let Some(ref names) = rule_names {
                req = req.set_config_rule_names(Some(names.clone()));
            }
            if let Some(ref types) = ct {
                req = req.set_compliance_types(Some(types.clone()));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.compliance_by_config_rules().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(results)
    }

    pub async fn describe_compliance_by_resource(
        &self,
        resource_type: Option<String>,
        compliance_types: Option<Vec<String>>,
    ) -> Result<Vec<config_sdk::types::ComplianceByResource>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;

        let ct: Option<Vec<config_sdk::types::ComplianceType>> = compliance_types.map(|types| {
            types
                .iter()
                .filter_map(|s| s.parse::<config_sdk::types::ComplianceType>().ok())
                .collect()
        });

        loop {
            let mut req = self.inner.describe_compliance_by_resource();
            if let Some(ref rt) = resource_type {
                req = req.resource_type(rt);
            }
            if let Some(ref types) = ct {
                req = req.set_compliance_types(Some(types.clone()));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.compliance_by_resources().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(results)
    }
}
