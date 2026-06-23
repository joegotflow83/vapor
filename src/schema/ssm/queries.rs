use async_graphql::{Context, Object, Result};

use crate::aws::ssm::SsmClient;
use crate::schema::ssm::types::{
    ManagedInstance, Parameter, ParameterFilter, ParameterMeta, ParameterTier, ParameterType,
    PingStatus, PlatformType, SsmDocument,
};

#[derive(Default)]
pub struct SsmQuery;

#[Object]
impl SsmQuery {
    async fn managed_instances(
        &self,
        ctx: &Context<'_>,
        instance_ids: Option<Vec<String>>,
        ping_status: Option<PingStatus>,
        platform_type: Option<PlatformType>,
    ) -> Result<Vec<ManagedInstance>> {
        let ssm = ctx.data::<SsmClient>()?;

        let ping_str = ping_status.map(|p| match p {
            PingStatus::Online => "Online".to_string(),
            PingStatus::ConnectionLost => "ConnectionLost".to_string(),
            PingStatus::Inactive => "Inactive".to_string(),
        });

        let platform_str = platform_type.map(|p| match p {
            PlatformType::Windows => "Windows".to_string(),
            PlatformType::Linux => "Linux".to_string(),
            PlatformType::MacOs => "MacOS".to_string(),
        });

        let results = ssm
            .describe_instance_information(instance_ids, ping_str, platform_str)
            .await?;

        Ok(results.into_iter().map(ManagedInstance::from).collect())
    }

    async fn parameters(
        &self,
        ctx: &Context<'_>,
        names: Vec<String>,
        with_decryption: Option<bool>,
    ) -> Result<Vec<Parameter>> {
        let ssm = ctx.data::<SsmClient>()?;
        let decrypt = with_decryption.unwrap_or(false);

        let results = ssm.get_parameters(names, decrypt).await?;
        let mut params: Vec<Parameter> = results.into_iter().map(Parameter::from).collect();

        if !decrypt {
            for param in &mut params {
                if param.parameter_type == Some(ParameterType::SecureString) {
                    param.value = Some("***".to_string());
                }
            }
        }

        let param_names: Vec<String> = params.iter().filter_map(|p| p.name.clone()).collect();
        let tier_map = ssm.get_parameter_tiers(&param_names).await?;
        for param in &mut params {
            if let Some(name) = &param.name {
                param.tier = tier_map.get(name).map(ParameterTier::from_sdk);
            }
        }

        Ok(params)
    }

    async fn parameters_by_path(
        &self,
        ctx: &Context<'_>,
        path: String,
        recursive: Option<bool>,
        with_decryption: Option<bool>,
    ) -> Result<Vec<Parameter>> {
        let ssm = ctx.data::<SsmClient>()?;
        let decrypt = with_decryption.unwrap_or(false);

        let results = ssm
            .get_parameters_by_path(path, recursive.unwrap_or(true), decrypt)
            .await?;
        let mut params: Vec<Parameter> = results.into_iter().map(Parameter::from).collect();

        if !decrypt {
            for param in &mut params {
                if param.parameter_type == Some(ParameterType::SecureString) {
                    param.value = Some("***".to_string());
                }
            }
        }

        let param_names: Vec<String> = params.iter().filter_map(|p| p.name.clone()).collect();
        let tier_map = ssm.get_parameter_tiers(&param_names).await?;
        for param in &mut params {
            if let Some(name) = &param.name {
                param.tier = tier_map.get(name).map(ParameterTier::from_sdk);
            }
        }

        Ok(params)
    }

    async fn parameter_metadata(
        &self,
        ctx: &Context<'_>,
        filters: Option<Vec<ParameterFilter>>,
    ) -> Result<Vec<ParameterMeta>> {
        let ssm = ctx.data::<SsmClient>()?;

        let sdk_filters = filters.map(|fs| fs.iter().map(|f| f.to_sdk_filter()).collect());

        let results = ssm.describe_parameters(sdk_filters).await?;
        Ok(results.into_iter().map(ParameterMeta::from).collect())
    }

    async fn documents(
        &self,
        ctx: &Context<'_>,
        owner: Option<String>,
        document_type: Option<String>,
        name: Option<String>,
    ) -> Result<Vec<SsmDocument>> {
        let ssm = ctx.data::<SsmClient>()?;

        let results = ssm.list_documents(owner, document_type, name).await?;
        Ok(results.into_iter().map(SsmDocument::from).collect())
    }
}
