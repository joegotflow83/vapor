use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct AppSyncApi {
    pub api_id: String,
    pub name: Option<String>,
    pub arn: Option<String>,
    pub authentication_type: Option<String>,
    pub uris: Vec<AppSyncUri>,
    pub xray_enabled: bool,
}

#[derive(SimpleObject, Clone)]
pub struct AppSyncUri {
    pub endpoint_type: String,
    pub uri: String,
}

#[derive(SimpleObject, Clone)]
pub struct AppSyncDataSource {
    pub name: String,
    pub data_source_type: Option<String>,
    pub description: Option<String>,
    pub service_role_arn: Option<String>,
}

impl From<aws_sdk_appsync::types::GraphqlApi> for AppSyncApi {
    fn from(api: aws_sdk_appsync::types::GraphqlApi) -> Self {
        let uris = api
            .uris()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| AppSyncUri {
                        endpoint_type: k.clone(),
                        uri: v.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            api_id: api.api_id().unwrap_or_default().to_string(),
            name: api.name().map(|s| s.to_string()),
            arn: api.arn().map(|s| s.to_string()),
            authentication_type: api.authentication_type().map(|t| t.as_str().to_string()),
            uris,
            xray_enabled: api.xray_enabled(),
        }
    }
}

impl From<aws_sdk_appsync::types::DataSource> for AppSyncDataSource {
    fn from(ds: aws_sdk_appsync::types::DataSource) -> Self {
        Self {
            name: ds.name().unwrap_or_default().to_string(),
            data_source_type: ds.r#type().map(|t| t.as_str().to_string()),
            description: ds.description().map(|s| s.to_string()),
            service_role_arn: ds.service_role_arn().map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appsync_api_from_sdk_minimal() {
        let api = aws_sdk_appsync::types::GraphqlApi::builder().build();
        let result = AppSyncApi::from(api);
        assert_eq!(result.api_id, "");
        assert!(result.name.is_none());
        assert!(result.arn.is_none());
        assert!(result.authentication_type.is_none());
        assert!(result.uris.is_empty());
        assert!(!result.xray_enabled);
    }

    #[test]
    fn test_appsync_api_from_sdk_full() {
        let api = aws_sdk_appsync::types::GraphqlApi::builder()
            .api_id("abc123")
            .name("MyApi")
            .arn("arn:aws:appsync:us-east-1:123456789012:apis/abc123")
            .authentication_type(aws_sdk_appsync::types::AuthenticationType::ApiKey)
            .uris("GRAPHQL", "https://abc123.appsync-api.us-east-1.amazonaws.com/graphql")
            .xray_enabled(true)
            .build();
        let result = AppSyncApi::from(api);
        assert_eq!(result.api_id, "abc123");
        assert_eq!(result.name, Some("MyApi".to_string()));
        assert_eq!(
            result.arn,
            Some("arn:aws:appsync:us-east-1:123456789012:apis/abc123".to_string())
        );
        assert_eq!(result.authentication_type, Some("API_KEY".to_string()));
        assert_eq!(result.uris.len(), 1);
        assert_eq!(result.uris[0].endpoint_type, "GRAPHQL");
        assert!(result.xray_enabled);
    }

    #[test]
    fn test_appsync_data_source_from_sdk_minimal() {
        let ds = aws_sdk_appsync::types::DataSource::builder().build();
        let result = AppSyncDataSource::from(ds);
        assert_eq!(result.name, "");
        assert!(result.data_source_type.is_none());
        assert!(result.description.is_none());
        assert!(result.service_role_arn.is_none());
    }

    #[test]
    fn test_appsync_data_source_from_sdk_full() {
        let ds = aws_sdk_appsync::types::DataSource::builder()
            .name("MyTable")
            .r#type(aws_sdk_appsync::types::DataSourceType::AmazonDynamodb)
            .description("DynamoDB data source")
            .service_role_arn("arn:aws:iam::123456789012:role/appsync-role")
            .build();
        let result = AppSyncDataSource::from(ds);
        assert_eq!(result.name, "MyTable");
        assert_eq!(result.data_source_type, Some("AMAZON_DYNAMODB".to_string()));
        assert_eq!(result.description, Some("DynamoDB data source".to_string()));
        assert_eq!(
            result.service_role_arn,
            Some("arn:aws:iam::123456789012:role/appsync-role".to_string())
        );
    }
}
