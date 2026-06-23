use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct SsmClient {
    inner: aws_sdk_ssm::Client,
}

impl SsmClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ssm::Client::new(config),
        }
    }

    pub async fn describe_instance_information(
        &self,
        instance_ids: Option<Vec<String>>,
        ping_status: Option<String>,
        platform_type: Option<String>,
    ) -> Result<Vec<aws_sdk_ssm::types::InstanceInformation>, VaporError> {
        let mut filters: Vec<aws_sdk_ssm::types::InstanceInformationStringFilter> = Vec::new();

        if let Some(ids) = instance_ids {
            if !ids.is_empty() {
                filters.push(
                    aws_sdk_ssm::types::InstanceInformationStringFilter::builder()
                        .key("InstanceIds")
                        .set_values(Some(ids))
                        .build()
                        .map_err(|e| VaporError::AwsSdk(e.to_string()))?,
                );
            }
        }

        if let Some(status) = ping_status {
            filters.push(
                aws_sdk_ssm::types::InstanceInformationStringFilter::builder()
                    .key("PingStatus")
                    .values(status)
                    .build()
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?,
            );
        }

        if let Some(platform) = platform_type {
            filters.push(
                aws_sdk_ssm::types::InstanceInformationStringFilter::builder()
                    .key("PlatformType")
                    .values(platform)
                    .build()
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?,
            );
        }

        let mut all_items: Vec<aws_sdk_ssm::types::InstanceInformation> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_instance_information();

            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_items.extend(output.instance_information_list().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }

    pub async fn get_parameters(
        &self,
        names: Vec<String>,
        with_decryption: bool,
    ) -> Result<Vec<aws_sdk_ssm::types::Parameter>, VaporError> {
        let output = self
            .inner
            .get_parameters()
            .set_names(Some(names))
            .with_decryption(with_decryption)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.parameters().to_vec())
    }

    pub async fn get_parameters_by_path(
        &self,
        path: String,
        recursive: bool,
        with_decryption: bool,
    ) -> Result<Vec<aws_sdk_ssm::types::Parameter>, VaporError> {
        let mut all_params: Vec<aws_sdk_ssm::types::Parameter> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self
                .inner
                .get_parameters_by_path()
                .path(&path)
                .recursive(recursive)
                .with_decryption(with_decryption);

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_params.extend(output.parameters().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_params)
    }

    pub async fn describe_parameters(
        &self,
        filters: Option<Vec<aws_sdk_ssm::types::ParameterStringFilter>>,
    ) -> Result<Vec<aws_sdk_ssm::types::ParameterMetadata>, VaporError> {
        let mut all_params: Vec<aws_sdk_ssm::types::ParameterMetadata> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self
                .inner
                .describe_parameters()
                .set_parameter_filters(filters.clone());

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_params.extend(output.parameters().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_params)
    }

    pub async fn get_parameter_tiers(
        &self,
        names: &[String],
    ) -> Result<std::collections::HashMap<String, aws_sdk_ssm::types::ParameterTier>, VaporError> {
        if names.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let filter = aws_sdk_ssm::types::ParameterStringFilter::builder()
            .key("Name")
            .option("Equals")
            .set_values(Some(names.to_vec()))
            .build()
            .expect("key is always provided");
        let metadata = self.describe_parameters(Some(vec![filter])).await?;
        Ok(metadata
            .into_iter()
            .filter_map(|m| {
                let name = m.name()?.to_string();
                let tier = m.tier()?.clone();
                Some((name, tier))
            })
            .collect())
    }

    pub async fn list_documents(
        &self,
        owner: Option<String>,
        document_type: Option<String>,
        name: Option<String>,
    ) -> Result<Vec<aws_sdk_ssm::types::DocumentIdentifier>, VaporError> {
        let mut filters: Vec<aws_sdk_ssm::types::DocumentKeyValuesFilter> = Vec::new();

        if let Some(owner_val) = owner {
            filters.push(
                aws_sdk_ssm::types::DocumentKeyValuesFilter::builder()
                    .key("Owner")
                    .values(owner_val)
                    .build(),
            );
        }

        if let Some(doc_type) = document_type {
            filters.push(
                aws_sdk_ssm::types::DocumentKeyValuesFilter::builder()
                    .key("DocumentType")
                    .values(doc_type)
                    .build(),
            );
        }

        if let Some(name_val) = name {
            filters.push(
                aws_sdk_ssm::types::DocumentKeyValuesFilter::builder()
                    .key("Name")
                    .values(name_val)
                    .build(),
            );
        }

        let mut all_docs: Vec<aws_sdk_ssm::types::DocumentIdentifier> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.list_documents();

            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_docs.extend(output.document_identifiers().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_docs)
    }
}
