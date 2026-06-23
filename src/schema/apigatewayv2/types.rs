use async_graphql::SimpleObject;
use aws_sdk_apigatewayv2::types::{
    DomainName as SdkDomainName,
    DomainNameConfiguration as SdkDomainNameConfig,
    Route as SdkRoute,
    RouteSettings as SdkRouteSettings,
    Stage as SdkStage,
    VpcLink as SdkVpcLink,
};

fn tags_from_map(map: Option<&std::collections::HashMap<String, String>>) -> Vec<ApiV2Tag> {
    map.map(|m| {
        m.iter()
            .map(|(k, v)| ApiV2Tag { key: k.clone(), value: v.clone() })
            .collect()
    })
    .unwrap_or_default()
}

#[derive(SimpleObject, Clone)]
pub struct ApiV2Tag {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct ApiV2RouteSettings {
    pub throttling_burst_limit: Option<i32>,
    pub throttling_rate_limit: Option<f64>,
    pub logging_level: Option<String>,
    pub data_trace_enabled: Option<bool>,
    pub detailed_metrics_enabled: Option<bool>,
}

impl From<&SdkRouteSettings> for ApiV2RouteSettings {
    fn from(rs: &SdkRouteSettings) -> Self {
        Self {
            throttling_burst_limit: rs.throttling_burst_limit(),
            throttling_rate_limit: rs.throttling_rate_limit(),
            logging_level: rs.logging_level().map(|l| l.as_str().to_string()),
            data_trace_enabled: rs.data_trace_enabled(),
            detailed_metrics_enabled: rs.detailed_metrics_enabled(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ApiV2 {
    pub api_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    /// HTTP | WEBSOCKET
    pub protocol_type: Option<String>,
    pub api_endpoint: Option<String>,
    pub route_selection_expression: Option<String>,
    pub cors_allow_origins: Vec<String>,
    pub cors_allow_methods: Vec<String>,
    pub disable_execute_api_endpoint: Option<bool>,
    pub created_date: Option<String>,
    pub tags: Vec<ApiV2Tag>,
}

impl From<aws_sdk_apigatewayv2::types::Api> for ApiV2 {
    fn from(a: aws_sdk_apigatewayv2::types::Api) -> Self {
        let cors_allow_origins = a
            .cors_configuration()
            .map(|c| c.allow_origins().iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let cors_allow_methods = a
            .cors_configuration()
            .map(|c| c.allow_methods().iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let tags = tags_from_map(a.tags());
        Self {
            api_id: a.api_id().map(|v| v.to_string()),
            name: a.name().map(|v| v.to_string()),
            description: a.description().map(|v| v.to_string()),
            protocol_type: a.protocol_type().map(|p| p.as_str().to_string()),
            api_endpoint: a.api_endpoint().map(|v| v.to_string()),
            route_selection_expression: a.route_selection_expression().map(|v| v.to_string()),
            cors_allow_origins,
            cors_allow_methods,
            disable_execute_api_endpoint: a.disable_execute_api_endpoint(),
            created_date: a.created_date().map(|t| t.to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ApiV2Stage {
    pub stage_name: Option<String>,
    pub description: Option<String>,
    pub auto_deploy: Option<bool>,
    pub deployment_id: Option<String>,
    pub last_updated: Option<String>,
    /// Key names only — values omitted to avoid exposing sensitive configuration.
    pub stage_variables: Vec<String>,
    pub default_route_settings: Option<ApiV2RouteSettings>,
    pub tags: Vec<ApiV2Tag>,
}

impl From<SdkStage> for ApiV2Stage {
    fn from(s: SdkStage) -> Self {
        let stage_variables = s
            .stage_variables()
            .map(|m| m.keys().map(|k| k.to_string()).collect())
            .unwrap_or_default();
        let tags = tags_from_map(s.tags());
        Self {
            stage_name: s.stage_name().map(|v| v.to_string()),
            description: s.description().map(|v| v.to_string()),
            auto_deploy: s.auto_deploy(),
            deployment_id: s.deployment_id().map(|v| v.to_string()),
            last_updated: s.last_updated_date().map(|t| t.to_string()),
            stage_variables,
            default_route_settings: s.default_route_settings().map(ApiV2RouteSettings::from),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ApiV2Route {
    pub route_id: Option<String>,
    /// e.g. "GET /users" or "$default"
    pub route_key: Option<String>,
    pub target: Option<String>,
    /// NONE | JWT | AWS_IAM | CUSTOM
    pub authorization_type: Option<String>,
    pub authorizer_id: Option<String>,
    pub api_key_required: Option<bool>,
}

impl From<SdkRoute> for ApiV2Route {
    fn from(r: SdkRoute) -> Self {
        Self {
            route_id: r.route_id().map(|v| v.to_string()),
            route_key: r.route_key().map(|v| v.to_string()),
            target: r.target().map(|v| v.to_string()),
            authorization_type: r.authorization_type().map(|a| a.as_str().to_string()),
            authorizer_id: r.authorizer_id().map(|v| v.to_string()),
            api_key_required: r.api_key_required(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ApiV2DomainNameConfig {
    pub api_gateway_domain_name: Option<String>,
    pub certificate_arn: Option<String>,
    /// REGIONAL | EDGE
    pub endpoint_type: Option<String>,
    pub hosted_zone_id: Option<String>,
    /// TLS_1_0 | TLS_1_2
    pub security_policy: Option<String>,
    pub domain_name_status: Option<String>,
}

impl From<&SdkDomainNameConfig> for ApiV2DomainNameConfig {
    fn from(c: &SdkDomainNameConfig) -> Self {
        Self {
            api_gateway_domain_name: c.api_gateway_domain_name().map(|v| v.to_string()),
            certificate_arn: c.certificate_arn().map(|v| v.to_string()),
            endpoint_type: c.endpoint_type().map(|e| e.as_str().to_string()),
            hosted_zone_id: c.hosted_zone_id().map(|v| v.to_string()),
            security_policy: c.security_policy().map(|p| p.as_str().to_string()),
            domain_name_status: c.domain_name_status().map(|s| s.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ApiV2DomainName {
    pub domain_name: String,
    pub api_mapping_selection_expression: Option<String>,
    pub domain_name_configurations: Vec<ApiV2DomainNameConfig>,
    pub tags: Vec<ApiV2Tag>,
}

impl From<SdkDomainName> for ApiV2DomainName {
    fn from(d: SdkDomainName) -> Self {
        let domain_name_configurations = d
            .domain_name_configurations()
            .iter()
            .map(ApiV2DomainNameConfig::from)
            .collect();
        let tags = tags_from_map(d.tags());
        Self {
            domain_name: d.domain_name().unwrap_or_default().to_string(),
            api_mapping_selection_expression: d
                .api_mapping_selection_expression()
                .map(|v| v.to_string()),
            domain_name_configurations,
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ApiV2VpcLink {
    pub vpc_link_id: Option<String>,
    pub name: Option<String>,
    /// AVAILABLE | PENDING | DELETING | FAILED | INACTIVE
    pub vpc_link_status: Option<String>,
    pub security_group_ids: Vec<String>,
    pub subnet_ids: Vec<String>,
    pub created_date: Option<String>,
    pub tags: Vec<ApiV2Tag>,
}

impl From<SdkVpcLink> for ApiV2VpcLink {
    fn from(v: SdkVpcLink) -> Self {
        let tags = tags_from_map(v.tags());
        Self {
            vpc_link_id: v.vpc_link_id().map(|id| id.to_string()),
            name: v.name().map(|n| n.to_string()),
            vpc_link_status: v.vpc_link_status().map(|s| s.as_str().to_string()),
            security_group_ids: v.security_group_ids().to_vec(),
            subnet_ids: v.subnet_ids().to_vec(),
            created_date: v.created_date().map(|t| t.to_string()),
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_v2_from_sdk_minimal() {
        let api = aws_sdk_apigatewayv2::types::Api::builder()
            .name("my-http-api")
            .protocol_type(aws_sdk_apigatewayv2::types::ProtocolType::Http)
            .route_selection_expression("$request.method $request.path")
            .build();
        let result = ApiV2::from(api);
        assert_eq!(result.name, Some("my-http-api".to_string()));
        assert_eq!(result.protocol_type, Some("HTTP".to_string()));
        assert!(result.cors_allow_origins.is_empty());
        assert!(result.cors_allow_methods.is_empty());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_api_v2_with_cors_and_tags() {
        let cors = aws_sdk_apigatewayv2::types::Cors::builder()
            .allow_origins("https://example.com")
            .allow_methods("GET")
            .allow_methods("POST")
            .build();
        let mut tags = std::collections::HashMap::new();
        tags.insert("env".to_string(), "prod".to_string());
        let api = aws_sdk_apigatewayv2::types::Api::builder()
            .api_id("api-123")
            .name("my-api")
            .protocol_type(aws_sdk_apigatewayv2::types::ProtocolType::Http)
            .route_selection_expression("$request.method $request.path")
            .api_endpoint("https://abc.execute-api.us-east-1.amazonaws.com")
            .cors_configuration(cors)
            .disable_execute_api_endpoint(false)
            .set_tags(Some(tags))
            .build();
        let result = ApiV2::from(api);
        assert_eq!(result.api_id, Some("api-123".to_string()));
        assert_eq!(result.api_endpoint, Some("https://abc.execute-api.us-east-1.amazonaws.com".to_string()));
        assert_eq!(result.cors_allow_origins, vec!["https://example.com"]);
        assert_eq!(result.cors_allow_methods, vec!["GET", "POST"]);
        assert_eq!(result.disable_execute_api_endpoint, Some(false));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_api_v2_websocket_protocol() {
        let api = aws_sdk_apigatewayv2::types::Api::builder()
            .name("my-ws-api")
            .protocol_type(aws_sdk_apigatewayv2::types::ProtocolType::Websocket)
            .route_selection_expression("$request.body.action")
            .build();
        let result = ApiV2::from(api);
        assert_eq!(result.protocol_type, Some("WEBSOCKET".to_string()));
    }

    #[test]
    fn test_api_v2_route_settings_from_sdk() {
        let rs = aws_sdk_apigatewayv2::types::RouteSettings::builder()
            .throttling_burst_limit(1000)
            .throttling_rate_limit(500.0)
            .data_trace_enabled(false)
            .detailed_metrics_enabled(true)
            .build();
        let result = ApiV2RouteSettings::from(&rs);
        assert_eq!(result.throttling_burst_limit, Some(1000));
        assert_eq!(result.throttling_rate_limit, Some(500.0));
        assert_eq!(result.data_trace_enabled, Some(false));
        assert_eq!(result.detailed_metrics_enabled, Some(true));
        assert!(result.logging_level.is_none());
    }

    #[test]
    fn test_api_v2_stage_from_sdk() {
        let route_settings = aws_sdk_apigatewayv2::types::RouteSettings::builder()
            .throttling_burst_limit(200)
            .throttling_rate_limit(100.0)
            .build();
        let stage = aws_sdk_apigatewayv2::types::Stage::builder()
            .stage_name("prod")
            .description("Production stage")
            .auto_deploy(true)
            .deployment_id("dep-abc")
            .default_route_settings(route_settings)
            .build();
        let result = ApiV2Stage::from(stage);
        assert_eq!(result.stage_name, Some("prod".to_string()));
        assert_eq!(result.description, Some("Production stage".to_string()));
        assert_eq!(result.auto_deploy, Some(true));
        assert_eq!(result.deployment_id, Some("dep-abc".to_string()));
        assert!(result.stage_variables.is_empty());
        let settings = result.default_route_settings.unwrap();
        assert_eq!(settings.throttling_burst_limit, Some(200));
        assert_eq!(settings.throttling_rate_limit, Some(100.0));
    }

    #[test]
    fn test_api_v2_stage_variables_keys_only() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("DB_HOST".to_string(), "secret-host.example.com".to_string());
        vars.insert("REGION".to_string(), "us-east-1".to_string());
        let stage = aws_sdk_apigatewayv2::types::Stage::builder()
            .stage_name("dev")
            .set_stage_variables(Some(vars))
            .build();
        let result = ApiV2Stage::from(stage);
        let mut keys = result.stage_variables.clone();
        keys.sort();
        assert_eq!(keys, vec!["DB_HOST", "REGION"]);
    }

    #[test]
    fn test_api_v2_route_from_sdk() {
        let route = aws_sdk_apigatewayv2::types::Route::builder()
            .route_id("route-xyz")
            .route_key("GET /users/{id}")
            .target("integrations/int-123")
            .authorization_type(aws_sdk_apigatewayv2::types::AuthorizationType::Jwt)
            .authorizer_id("auth-abc")
            .api_key_required(false)
            .build();
        let result = ApiV2Route::from(route);
        assert_eq!(result.route_id, Some("route-xyz".to_string()));
        assert_eq!(result.route_key, Some("GET /users/{id}".to_string()));
        assert_eq!(result.target, Some("integrations/int-123".to_string()));
        assert_eq!(result.authorization_type, Some("JWT".to_string()));
        assert_eq!(result.authorizer_id, Some("auth-abc".to_string()));
        assert_eq!(result.api_key_required, Some(false));
    }

    #[test]
    fn test_api_v2_route_default_key() {
        let route = aws_sdk_apigatewayv2::types::Route::builder()
            .route_key("$default")
            .build();
        let result = ApiV2Route::from(route);
        assert_eq!(result.route_key, Some("$default".to_string()));
        assert!(result.route_id.is_none());
        assert!(result.authorization_type.is_none());
    }

    #[test]
    fn test_api_v2_domain_name_from_sdk() {
        let config = aws_sdk_apigatewayv2::types::DomainNameConfiguration::builder()
            .api_gateway_domain_name("d-abc123.execute-api.us-east-1.amazonaws.com")
            .certificate_arn("arn:aws:acm:us-east-1:123:certificate/abc")
            .endpoint_type(aws_sdk_apigatewayv2::types::EndpointType::Regional)
            .hosted_zone_id("Z1UJRXOUMOOFQ8")
            .security_policy(aws_sdk_apigatewayv2::types::SecurityPolicy::Tls12)
            .domain_name_status(aws_sdk_apigatewayv2::types::DomainNameStatus::Available)
            .build();
        let dn = aws_sdk_apigatewayv2::types::DomainName::builder()
            .domain_name("api.example.com")
            .api_mapping_selection_expression("$request.basepath")
            .domain_name_configurations(config)
            .build();
        let result = ApiV2DomainName::from(dn);
        assert_eq!(result.domain_name, "api.example.com");
        assert_eq!(
            result.api_mapping_selection_expression,
            Some("$request.basepath".to_string())
        );
        assert_eq!(result.domain_name_configurations.len(), 1);
        let cfg = &result.domain_name_configurations[0];
        assert_eq!(
            cfg.api_gateway_domain_name,
            Some("d-abc123.execute-api.us-east-1.amazonaws.com".to_string())
        );
        assert_eq!(cfg.endpoint_type, Some("REGIONAL".to_string()));
        assert_eq!(cfg.security_policy, Some("TLS_1_2".to_string()));
        assert_eq!(cfg.domain_name_status, Some("AVAILABLE".to_string()));
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_api_v2_vpc_link_from_sdk() {
        let mut tags = std::collections::HashMap::new();
        tags.insert("team".to_string(), "platform".to_string());
        let vl = aws_sdk_apigatewayv2::types::VpcLink::builder()
            .vpc_link_id("vl-abc123")
            .name("my-vpc-link")
            .vpc_link_status(aws_sdk_apigatewayv2::types::VpcLinkStatus::Available)
            .security_group_ids("sg-111")
            .security_group_ids("sg-222")
            .subnet_ids("subnet-aaa")
            .subnet_ids("subnet-bbb")
            .set_tags(Some(tags))
            .build();
        let result = ApiV2VpcLink::from(vl);
        assert_eq!(result.vpc_link_id, Some("vl-abc123".to_string()));
        assert_eq!(result.name, Some("my-vpc-link".to_string()));
        assert_eq!(result.vpc_link_status, Some("AVAILABLE".to_string()));
        assert_eq!(result.security_group_ids, vec!["sg-111", "sg-222"]);
        assert_eq!(result.subnet_ids, vec!["subnet-aaa", "subnet-bbb"]);
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "team");
        assert_eq!(result.tags[0].value, "platform");
    }

    #[test]
    fn test_api_v2_vpc_link_minimal() {
        let vl = aws_sdk_apigatewayv2::types::VpcLink::builder()
            .vpc_link_id("vl-min")
            .name("minimal-link")
            .build();
        let result = ApiV2VpcLink::from(vl);
        assert_eq!(result.vpc_link_id, Some("vl-min".to_string()));
        assert!(result.security_group_ids.is_empty());
        assert!(result.subnet_ids.is_empty());
        assert!(result.vpc_link_status.is_none());
        assert!(result.tags.is_empty());
    }
}
