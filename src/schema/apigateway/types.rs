use async_graphql::SimpleObject;

use crate::schema::common::types::Tag;

fn tags_from_map(map: Option<&std::collections::HashMap<String, String>>) -> Vec<Tag> {
    map.map(|m| m.iter().map(|(k, v)| Tag { key: k.clone(), value: v.clone() }).collect())
        .unwrap_or_default()
}

// ── REST API (v1) types ───────────────────────────────────────────────────────

/// Endpoint configuration for a REST API (EDGE | REGIONAL | PRIVATE).
#[derive(SimpleObject, Clone)]
pub struct ApigwEndpointConfiguration {
    pub types: Vec<String>,
    pub vpc_endpoint_ids: Vec<String>,
}

impl From<&aws_sdk_apigateway::types::EndpointConfiguration> for ApigwEndpointConfiguration {
    fn from(ec: &aws_sdk_apigateway::types::EndpointConfiguration) -> Self {
        Self {
            types: ec.types().iter().map(|t| t.as_str().to_string()).collect(),
            vpc_endpoint_ids: ec.vpc_endpoint_ids().iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// A REST API Gateway API (v1).
#[derive(SimpleObject, Clone)]
pub struct ApigwRestApi {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_date: Option<String>,
    pub version: Option<String>,
    pub binary_media_types: Vec<String>,
    pub minimum_compression_size: Option<i32>,
    /// HEADER | AUTHORIZER
    pub api_key_source: Option<String>,
    pub endpoint_configuration: Option<ApigwEndpointConfiguration>,
    /// Raw JSON policy document.
    pub policy: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_apigateway::types::RestApi> for ApigwRestApi {
    fn from(api: aws_sdk_apigateway::types::RestApi) -> Self {
        Self {
            id: api.id().map(|s| s.to_string()),
            name: api.name().map(|s| s.to_string()),
            description: api.description().map(|s| s.to_string()),
            created_date: api.created_date().map(|d| d.to_string()),
            version: api.version().map(|s| s.to_string()),
            binary_media_types: api.binary_media_types().iter().map(|s| s.to_string()).collect(),
            minimum_compression_size: api.minimum_compression_size(),
            api_key_source: api.api_key_source().map(|s| s.as_str().to_string()),
            endpoint_configuration: api
                .endpoint_configuration()
                .map(ApigwEndpointConfiguration::from),
            policy: api.policy().map(|s| s.to_string()),
            tags: tags_from_map(api.tags()),
        }
    }
}

/// Throttling settings extracted from a REST stage's method_settings[\"*/*\"].
/// Defined for future per-method exposure; not currently returned from queries.
#[derive(SimpleObject, Clone)]
pub struct ApigwMethodThrottlingSettings {
    pub burst_limit: Option<i32>,
    pub rate_limit: Option<f64>,
}

/// Access log destination and format for a REST stage.
#[derive(SimpleObject, Clone)]
pub struct ApigwAccessLogSettings {
    pub format: Option<String>,
    pub destination_arn: Option<String>,
}

impl From<&aws_sdk_apigateway::types::AccessLogSettings> for ApigwAccessLogSettings {
    fn from(als: &aws_sdk_apigateway::types::AccessLogSettings) -> Self {
        Self {
            format: als.format().map(|s| s.to_string()),
            destination_arn: als.destination_arn().map(|s| s.to_string()),
        }
    }
}

/// A deployment stage for a REST API (v1).
///
/// `throttling_burst_limit` and `throttling_rate_limit` come from the `*/*`
/// (default) entry in `method_settings`, if present.
#[derive(SimpleObject, Clone)]
pub struct ApigwRestStage {
    pub stage_name: Option<String>,
    pub deployment_id: Option<String>,
    pub description: Option<String>,
    pub created_date: Option<String>,
    pub last_updated_date: Option<String>,
    pub throttling_burst_limit: Option<i32>,
    pub throttling_rate_limit: Option<f64>,
    pub tracing_enabled: Option<bool>,
    pub cache_cluster_enabled: Option<bool>,
    /// e.g. "0.5" | "1.6" | ...
    pub cache_cluster_size: Option<String>,
    pub access_log_settings: Option<ApigwAccessLogSettings>,
    pub documentation_version: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_apigateway::types::Stage> for ApigwRestStage {
    fn from(stage: aws_sdk_apigateway::types::Stage) -> Self {
        // Extract default method throttling from the "*/*" key.
        let (throttling_burst_limit, throttling_rate_limit) = {
            let default_ms = stage.method_settings().and_then(|m| m.get("*/*"));
            (
                default_ms.map(|ms| ms.throttling_burst_limit()),
                default_ms.map(|ms| ms.throttling_rate_limit()),
            )
        };
        Self {
            stage_name: stage.stage_name().map(|s| s.to_string()),
            deployment_id: stage.deployment_id().map(|s| s.to_string()),
            description: stage.description().map(|s| s.to_string()),
            created_date: stage.created_date().map(|d| d.to_string()),
            last_updated_date: stage.last_updated_date().map(|d| d.to_string()),
            throttling_burst_limit,
            throttling_rate_limit,
            tracing_enabled: Some(stage.tracing_enabled()),
            cache_cluster_enabled: Some(stage.cache_cluster_enabled()),
            cache_cluster_size: stage.cache_cluster_size().map(|cs| cs.as_str().to_string()),
            access_log_settings: stage
                .access_log_settings()
                .map(ApigwAccessLogSettings::from),
            documentation_version: stage.documentation_version().map(|s| s.to_string()),
            tags: tags_from_map(stage.tags()),
        }
    }
}

/// A resource (path node) in a REST API, with the HTTP methods defined on it.
#[derive(SimpleObject, Clone)]
pub struct ApigwResource {
    pub id: Option<String>,
    pub parent_id: Option<String>,
    pub path_part: Option<String>,
    pub path: Option<String>,
    /// Keys of the `resource_methods` map, e.g. ["GET", "POST"].
    pub http_methods: Vec<String>,
}

impl From<aws_sdk_apigateway::types::Resource> for ApigwResource {
    fn from(resource: aws_sdk_apigateway::types::Resource) -> Self {
        Self {
            id: resource.id().map(|s| s.to_string()),
            parent_id: resource.parent_id().map(|s| s.to_string()),
            path_part: resource.path_part().map(|s| s.to_string()),
            path: resource.path().map(|s| s.to_string()),
            http_methods: resource
                .resource_methods()
                .map(|m| m.keys().map(|k| k.to_string()).collect())
                .unwrap_or_default(),
        }
    }
}

/// A deployment of a REST API.
#[derive(SimpleObject, Clone)]
pub struct ApigwDeployment {
    pub id: Option<String>,
    pub description: Option<String>,
    pub created_date: Option<String>,
}

impl From<aws_sdk_apigateway::types::Deployment> for ApigwDeployment {
    fn from(dep: aws_sdk_apigateway::types::Deployment) -> Self {
        Self {
            id: dep.id().map(|s| s.to_string()),
            description: dep.description().map(|s| s.to_string()),
            created_date: dep.created_date().map(|d| d.to_string()),
        }
    }
}

// HTTP/WebSocket API (v2) support lives in the dedicated `apigatewayv2` schema
// module (`ApiV2*` types), not here — this module covers REST APIs (v1) only.

// ── Unit Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apigw_endpoint_configuration_from() {
        let ec = aws_sdk_apigateway::types::EndpointConfiguration::builder()
            .types(aws_sdk_apigateway::types::EndpointType::Regional)
            .vpc_endpoint_ids("vpce-abc123")
            .build();
        let result = ApigwEndpointConfiguration::from(&ec);
        assert_eq!(result.types, vec!["REGIONAL"]);
        assert_eq!(result.vpc_endpoint_ids, vec!["vpce-abc123"]);
    }

    #[test]
    fn test_apigw_rest_api_from() {
        let api = aws_sdk_apigateway::types::RestApi::builder()
            .id("abc123")
            .name("My API")
            .description("Test API")
            .version("1.0")
            .binary_media_types("application/octet-stream")
            .minimum_compression_size(1024)
            .api_key_source(aws_sdk_apigateway::types::ApiKeySourceType::Header)
            .policy("{}")
            .build();
        let result = ApigwRestApi::from(api);
        assert_eq!(result.id, Some("abc123".to_string()));
        assert_eq!(result.name, Some("My API".to_string()));
        assert_eq!(result.description, Some("Test API".to_string()));
        assert_eq!(result.version, Some("1.0".to_string()));
        assert_eq!(result.binary_media_types, vec!["application/octet-stream"]);
        assert_eq!(result.minimum_compression_size, Some(1024));
        assert_eq!(result.api_key_source, Some("HEADER".to_string()));
        assert_eq!(result.policy, Some("{}".to_string()));
        assert!(result.tags.is_empty());
        assert!(result.endpoint_configuration.is_none());
    }

    #[test]
    fn test_apigw_rest_api_tags() {
        let mut tags = std::collections::HashMap::new();
        tags.insert("env".to_string(), "prod".to_string());
        let api = aws_sdk_apigateway::types::RestApi::builder()
            .set_tags(Some(tags))
            .build();
        let result = ApigwRestApi::from(api);
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_apigw_access_log_settings_from() {
        let als = aws_sdk_apigateway::types::AccessLogSettings::builder()
            .format("$context.requestId")
            .destination_arn("arn:aws:logs:us-east-1:123:log-group:my-group")
            .build();
        let result = ApigwAccessLogSettings::from(&als);
        assert_eq!(result.format, Some("$context.requestId".to_string()));
        assert_eq!(
            result.destination_arn,
            Some("arn:aws:logs:us-east-1:123:log-group:my-group".to_string())
        );
    }

    #[test]
    fn test_apigw_rest_stage_from() {
        let stage = aws_sdk_apigateway::types::Stage::builder()
            .stage_name("prod")
            .deployment_id("dep-abc")
            .description("Production stage")
            .tracing_enabled(true)
            .cache_cluster_enabled(false)
            .build();
        let result = ApigwRestStage::from(stage);
        assert_eq!(result.stage_name, Some("prod".to_string()));
        assert_eq!(result.deployment_id, Some("dep-abc".to_string()));
        assert_eq!(result.description, Some("Production stage".to_string()));
        assert_eq!(result.tracing_enabled, Some(true));
        assert_eq!(result.cache_cluster_enabled, Some(false));
        assert!(result.throttling_burst_limit.is_none());
        assert!(result.throttling_rate_limit.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_apigw_rest_stage_method_settings_default() {
        let mut method_settings = std::collections::HashMap::new();
        method_settings.insert(
            "*/*".to_string(),
            aws_sdk_apigateway::types::MethodSetting::builder()
                .throttling_burst_limit(100)
                .throttling_rate_limit(50.0)
                .build(),
        );
        let stage = aws_sdk_apigateway::types::Stage::builder()
            .stage_name("dev")
            .set_method_settings(Some(method_settings))
            .build();
        let result = ApigwRestStage::from(stage);
        assert_eq!(result.throttling_burst_limit, Some(100));
        assert_eq!(result.throttling_rate_limit, Some(50.0));
    }

    #[test]
    fn test_apigw_resource_from() {
        let mut methods = std::collections::HashMap::new();
        methods.insert(
            "GET".to_string(),
            aws_sdk_apigateway::types::Method::builder().build(),
        );
        methods.insert(
            "POST".to_string(),
            aws_sdk_apigateway::types::Method::builder().build(),
        );
        let resource = aws_sdk_apigateway::types::Resource::builder()
            .id("res-123")
            .parent_id("res-root")
            .path_part("items")
            .path("/items")
            .set_resource_methods(Some(methods))
            .build();
        let result = ApigwResource::from(resource);
        assert_eq!(result.id, Some("res-123".to_string()));
        assert_eq!(result.parent_id, Some("res-root".to_string()));
        assert_eq!(result.path_part, Some("items".to_string()));
        assert_eq!(result.path, Some("/items".to_string()));
        let mut http_methods = result.http_methods.clone();
        http_methods.sort();
        assert_eq!(http_methods, vec!["GET", "POST"]);
    }

    #[test]
    fn test_apigw_resource_no_methods() {
        let resource = aws_sdk_apigateway::types::Resource::builder()
            .id("res-root")
            .path("/")
            .build();
        let result = ApigwResource::from(resource);
        assert_eq!(result.id, Some("res-root".to_string()));
        assert!(result.http_methods.is_empty());
    }

    #[test]
    fn test_apigw_deployment_from() {
        let dep = aws_sdk_apigateway::types::Deployment::builder()
            .id("dep-abc")
            .description("Initial deployment")
            .build();
        let result = ApigwDeployment::from(dep);
        assert_eq!(result.id, Some("dep-abc".to_string()));
        assert_eq!(result.description, Some("Initial deployment".to_string()));
        assert!(result.created_date.is_none());
    }
}
