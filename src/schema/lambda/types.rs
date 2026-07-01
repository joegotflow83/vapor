use async_graphql::SimpleObject;
use std::collections::HashMap;

use crate::schema::common::types::Tag;

/// VPC configuration for a Lambda function.
#[derive(SimpleObject, Clone)]
pub struct LambdaVpcConfig {
    pub subnet_ids: Vec<String>,
    pub security_group_ids: Vec<String>,
    pub vpc_id: Option<String>,
}

/// A Lambda function with metadata. Environment variable values are intentionally
/// omitted (only key names exposed) to avoid leaking secrets through GraphQL.
#[derive(SimpleObject, Clone)]
pub struct LambdaFunction {
    pub function_name: String,
    pub function_arn: Option<String>,
    /// Runtime identifier (e.g. "nodejs20.x", "python3.12").
    pub runtime: Option<String>,
    pub handler: Option<String>,
    pub code_size: Option<i64>,
    pub description: Option<String>,
    pub timeout: Option<i32>,
    pub memory_size: Option<i32>,
    pub last_modified: Option<String>,
    pub code_sha256: Option<String>,
    pub role: Option<String>,
    pub vpc_config: Option<LambdaVpcConfig>,
    /// Environment variable key names only — values are not exposed.
    pub environment_keys: Vec<String>,
    /// ARNs of Lambda layers attached to this function.
    pub layers: Vec<String>,
    /// Function state (Active, Inactive, Pending, Failed).
    pub state: Option<String>,
    pub state_reason: Option<String>,
    /// CPU architecture (x86_64 or arm64).
    pub architecture: Option<String>,
    pub tags: Vec<Tag>,
}

impl LambdaFunction {
    pub fn from_config_and_tags(
        cfg: aws_sdk_lambda::types::FunctionConfiguration,
        raw_tags: HashMap<String, String>,
    ) -> Self {
        let vpc_config = cfg.vpc_config().map(|v| LambdaVpcConfig {
            subnet_ids: v.subnet_ids().iter().map(|s| s.to_string()).collect(),
            security_group_ids: v
                .security_group_ids()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            vpc_id: v.vpc_id().map(|s| s.to_string()),
        });

        let environment_keys = cfg
            .environment()
            .and_then(|e| e.variables())
            .map(|vars| vars.keys().map(|k| k.to_string()).collect())
            .unwrap_or_default();

        let layers = cfg
            .layers()
            .iter()
            .filter_map(|l| l.arn().map(|a| a.to_string()))
            .collect();

        let architecture = cfg
            .architectures()
            .first()
            .map(|a| a.as_str().to_string());

        let tags = raw_tags
            .into_iter()
            .map(|(k, v)| Tag { key: k, value: v })
            .collect();

        Self {
            function_name: cfg.function_name().unwrap_or_default().to_string(),
            function_arn: cfg.function_arn().map(|s| s.to_string()),
            runtime: cfg.runtime().map(|r| r.as_str().to_string()),
            handler: cfg.handler().map(|s| s.to_string()),
            code_size: Some(cfg.code_size()),
            description: cfg.description().map(|s| s.to_string()),
            timeout: cfg.timeout(),
            memory_size: cfg.memory_size(),
            last_modified: cfg.last_modified().map(|s| s.to_string()),
            code_sha256: cfg.code_sha256().map(|s| s.to_string()),
            role: cfg.role().map(|s| s.to_string()),
            vpc_config,
            environment_keys,
            layers,
            state: cfg.state().map(|s| s.as_str().to_string()),
            state_reason: cfg.state_reason().map(|s| s.to_string()),
            architecture,
            tags,
        }
    }
}

/// A Lambda function alias pointing to a specific version.
#[derive(SimpleObject, Clone)]
pub struct LambdaAlias {
    pub name: Option<String>,
    pub alias_arn: Option<String>,
    /// The function version the alias points to.
    pub function_version: Option<String>,
    pub description: Option<String>,
}

impl From<aws_sdk_lambda::types::AliasConfiguration> for LambdaAlias {
    fn from(a: aws_sdk_lambda::types::AliasConfiguration) -> Self {
        Self {
            name: a.name().map(|s| s.to_string()),
            alias_arn: a.alias_arn().map(|s| s.to_string()),
            function_version: a.function_version().map(|s| s.to_string()),
            description: a.description().map(|s| s.to_string()),
        }
    }
}

/// A mapping between an event source (e.g. SQS, Kinesis, DynamoDB stream) and a Lambda function.
#[derive(SimpleObject, Clone)]
pub struct LambdaEventSourceMapping {
    pub uuid: Option<String>,
    pub event_source_arn: Option<String>,
    pub function_arn: Option<String>,
    /// Current state of the mapping (Enabled, Disabled, Creating, ...).
    pub state: Option<String>,
    pub batch_size: Option<i32>,
    pub starting_position: Option<String>,
    pub last_modified: Option<String>,
}

impl From<aws_sdk_lambda::types::EventSourceMappingConfiguration> for LambdaEventSourceMapping {
    fn from(m: aws_sdk_lambda::types::EventSourceMappingConfiguration) -> Self {
        Self {
            uuid: m.uuid().map(|s| s.to_string()),
            event_source_arn: m.event_source_arn().map(|s| s.to_string()),
            function_arn: m.function_arn().map(|s| s.to_string()),
            state: m.state().map(|s| s.to_string()),
            batch_size: m.batch_size(),
            starting_position: m.starting_position().map(|s| s.as_str().to_string()),
            last_modified: m.last_modified().map(|d| d.to_string()),
        }
    }
}

/// A specific version of a Lambda layer.
#[derive(SimpleObject, Clone)]
pub struct LambdaLayerVersion {
    pub layer_version_arn: Option<String>,
    pub version: Option<i64>,
    pub description: Option<String>,
    pub created_date: Option<String>,
    pub compatible_runtimes: Vec<String>,
    pub compatible_architectures: Vec<String>,
}

impl From<&aws_sdk_lambda::types::LayerVersionsListItem> for LambdaLayerVersion {
    fn from(v: &aws_sdk_lambda::types::LayerVersionsListItem) -> Self {
        Self {
            layer_version_arn: v.layer_version_arn().map(|s| s.to_string()),
            version: Some(v.version()),
            description: v.description().map(|s| s.to_string()),
            created_date: v.created_date().map(|s| s.to_string()),
            compatible_runtimes: v
                .compatible_runtimes()
                .iter()
                .map(|r| r.as_str().to_string())
                .collect(),
            compatible_architectures: v
                .compatible_architectures()
                .iter()
                .map(|a| a.as_str().to_string())
                .collect(),
        }
    }
}

/// A Lambda layer with its latest published version metadata.
#[derive(SimpleObject, Clone)]
pub struct LambdaLayer {
    pub layer_name: Option<String>,
    pub layer_arn: Option<String>,
    pub latest_matching_version: Option<LambdaLayerVersion>,
}

impl From<aws_sdk_lambda::types::LayersListItem> for LambdaLayer {
    fn from(l: aws_sdk_lambda::types::LayersListItem) -> Self {
        Self {
            layer_name: l.layer_name().map(|s| s.to_string()),
            layer_arn: l.layer_arn().map(|s| s.to_string()),
            latest_matching_version: l.latest_matching_version().map(LambdaLayerVersion::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lambda_function_minimal() {
        let cfg = aws_sdk_lambda::types::FunctionConfiguration::builder().build();
        let func = LambdaFunction::from_config_and_tags(cfg, HashMap::new());
        assert_eq!(func.function_name, "");
        assert!(func.function_arn.is_none());
        assert_eq!(func.code_size, Some(0));
        assert!(func.environment_keys.is_empty());
        assert!(func.layers.is_empty());
        assert!(func.vpc_config.is_none());
        assert!(func.tags.is_empty());
    }

    #[test]
    fn test_lambda_function_environment_keys_only() {
        use aws_sdk_lambda::types::EnvironmentResponse;
        let mut vars = HashMap::new();
        vars.insert("SECRET_KEY".to_string(), "super_secret_value".to_string());
        vars.insert("LOG_LEVEL".to_string(), "info".to_string());
        let env = EnvironmentResponse::builder()
            .set_variables(Some(vars))
            .build();
        let cfg = aws_sdk_lambda::types::FunctionConfiguration::builder()
            .environment(env)
            .build();
        let func = LambdaFunction::from_config_and_tags(cfg, HashMap::new());
        // Keys must be present; values must NOT be exposed
        assert_eq!(func.environment_keys.len(), 2);
        assert!(func.environment_keys.contains(&"SECRET_KEY".to_string()));
        assert!(func.environment_keys.contains(&"LOG_LEVEL".to_string()));
    }

    #[test]
    fn test_lambda_function_tags_conversion() {
        let cfg = aws_sdk_lambda::types::FunctionConfiguration::builder().build();
        let mut raw_tags = HashMap::new();
        raw_tags.insert("env".to_string(), "prod".to_string());
        let func = LambdaFunction::from_config_and_tags(cfg, raw_tags);
        assert_eq!(func.tags.len(), 1);
        assert_eq!(func.tags[0].key, "env");
        assert_eq!(func.tags[0].value, "prod");
    }

    #[test]
    fn test_lambda_alias_from() {
        let alias = aws_sdk_lambda::types::AliasConfiguration::builder()
            .name("live")
            .alias_arn("arn:aws:lambda:us-east-1:123:function:my-fn:live")
            .function_version("3")
            .description("Production alias")
            .build();
        let la = LambdaAlias::from(alias);
        assert_eq!(la.name, Some("live".to_string()));
        assert_eq!(la.function_version, Some("3".to_string()));
    }

    #[test]
    fn test_lambda_layer_from() {
        let item = aws_sdk_lambda::types::LayersListItem::builder()
            .layer_name("my-layer")
            .layer_arn("arn:aws:lambda:us-east-1:123:layer:my-layer")
            .build();
        let layer = LambdaLayer::from(item);
        assert_eq!(layer.layer_name, Some("my-layer".to_string()));
        assert!(layer.latest_matching_version.is_none());
    }
}
