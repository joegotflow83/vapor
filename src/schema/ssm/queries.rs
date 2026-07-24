use async_graphql::{Context, Object, Result};

use crate::aws::ssm::SsmClient;
use crate::schema::pagination::Page;
use crate::schema::ssm::types::{
    ManagedInstance, Parameter, ParameterFilter, ParameterMeta, ParameterTier, ParameterType,
    PingStatus, PlatformType, SsmDocument,
};

#[derive(Default)]
pub struct SsmQuery;

#[Object]
impl SsmQuery {
    /// Describes managed instance information. `limit` caps the number of
    /// instances listed (default unlimited); resumable via `nextToken`.
    async fn managed_instances(
        &self,
        ctx: &Context<'_>,
        instance_ids: Option<Vec<String>>,
        ping_status: Option<PingStatus>,
        platform_type: Option<PlatformType>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ManagedInstance>> {
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

        let (results, token) = ssm
            .describe_instance_information(instance_ids, ping_str, platform_str, limit, next_token)
            .await?;

        Ok(Page {
            items: results.into_iter().map(ManagedInstance::from).collect(),
            next_token: token,
        })
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

    /// Lists parameters under `path`. `limit` caps the number of parameters
    /// listed (default unlimited); resumable via `nextToken`.
    async fn parameters_by_path(
        &self,
        ctx: &Context<'_>,
        path: String,
        recursive: Option<bool>,
        with_decryption: Option<bool>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Parameter>> {
        let ssm = ctx.data::<SsmClient>()?;
        let decrypt = with_decryption.unwrap_or(false);

        let (results, token) = ssm
            .get_parameters_by_path(path, recursive.unwrap_or(true), decrypt, limit, next_token)
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

        Ok(Page {
            items: params,
            next_token: token,
        })
    }

    /// Describes parameter metadata. `limit` caps the number of results
    /// returned (default unlimited); resumable via `nextToken`.
    async fn parameter_metadata(
        &self,
        ctx: &Context<'_>,
        filters: Option<Vec<ParameterFilter>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ParameterMeta>> {
        let ssm = ctx.data::<SsmClient>()?;

        let sdk_filters = filters.map(|fs| fs.iter().map(|f| f.to_sdk_filter()).collect());

        let (results, token) = ssm
            .describe_parameters(sdk_filters, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(ParameterMeta::from).collect(),
            next_token: token,
        })
    }

    /// Lists SSM documents. `limit` caps the number of documents listed
    /// (default unlimited); resumable via `nextToken`.
    async fn documents(
        &self,
        ctx: &Context<'_>,
        owner: Option<String>,
        document_type: Option<String>,
        name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SsmDocument>> {
        let ssm = ctx.data::<SsmClient>()?;

        let (results, token) = ssm
            .list_documents(owner, document_type, name, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(SsmDocument::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::ssm::SsmClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::SsmQuery;

    // awsJson1.1, fixed `/` path, endpoint hostname matches crate name (no
    // gotcha-3 quirk). All timestamps here are epoch-seconds (confirmed
    // against pinned `aws-sdk-ssm` 1.113.0's `protocol_serde/
    // shape_instance_information.rs` — `LastPingDateTime`/`RegistrationDate`
    // both use `Format::EpochSeconds`, no gotcha-31 RFC3339 surprise).
    const BASE: &str = "https://ssm.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn managed_instances_maps_full_metadata_and_forwards_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Filters":[{"Key":"InstanceIds","Values":["i-1","i-2"]},{"Key":"PingStatus","Values":["Online"]},{"Key":"PlatformType","Values":["Linux"]}],"MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"InstanceInformationList":[{"InstanceId":"i-1","PingStatus":"Online","LastPingDateTime":1700000000,"PlatformType":"Linux","PlatformName":"Amazon Linux 2","PlatformVersion":"2.0","AgentVersion":"3.1.1804.0","IPAddress":"10.0.1.42","ComputerName":"ip-10-0-1-42","Name":"my-managed-instance","ResourceType":"ManagedInstance","IamRole":"arn:aws:iam::123456789012:role/SSMRole","RegistrationDate":1600000000}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let schema = build_query_schema(SsmQuery)
            .data(SsmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ managedInstances(instanceIds: ["i-1", "i-2"], pingStatus: ONLINE, platformType: LINUX, limit: 1) {
                    items { instanceId pingStatus lastPingTime platformType platformName platformVersion agentVersion ipAddress computerName name resourceType iamRole registrationDate }
                    nextToken
                } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["managedInstances"]["items"][0];
        assert_eq!(item["instanceId"], "i-1");
        assert_eq!(item["pingStatus"], "ONLINE");
        assert!(item["lastPingTime"].is_string());
        assert_eq!(item["platformType"], "LINUX");
        assert_eq!(item["platformName"], "Amazon Linux 2");
        assert_eq!(item["platformVersion"], "2.0");
        assert_eq!(item["agentVersion"], "3.1.1804.0");
        assert_eq!(item["ipAddress"], "10.0.1.42");
        assert_eq!(item["computerName"], "ip-10-0-1-42");
        assert_eq!(item["name"], "my-managed-instance");
        assert_eq!(item["resourceType"], "ManagedInstance");
        assert_eq!(item["iamRole"], "arn:aws:iam::123456789012:role/SSMRole");
        assert!(item["registrationDate"].is_string());
        assert_eq!(json["managedInstances"]["nextToken"], "page2-token");
        http_client.relaxed_requests_match();
    }

    // --- parameters (real logic: masks SecureString values unless
    // withDecryption is true, then fans out to get_parameter_tiers) ---

    #[tokio::test]
    async fn parameters_masks_secure_string_unless_decrypted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"Names":["/app/db-password"],"WithDecryption":false}"#,
                ),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/db-password","Type":"SecureString","Value":"hunter2","Version":3}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"ParameterFilters":[{"Key":"Name","Option":"Equals","Values":["/app/db-password"]}]}"#,
                ),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/db-password","Tier":"Advanced"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(SsmQuery)
            .data(SsmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ parameters(names: ["/app/db-password"]) { name parameterType value version tier } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let param = &json["parameters"][0];
        assert_eq!(param["name"], "/app/db-password");
        assert_eq!(param["parameterType"], "SECURE_STRING");
        assert_eq!(param["value"], "***");
        assert_eq!(param["version"], 3);
        assert_eq!(param["tier"], "ADVANCED");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn parameters_with_decryption_reveals_value() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"Names":["/app/db-password"],"WithDecryption":true}"#,
                ),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/db-password","Type":"SecureString","Value":"hunter2"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"ParameterFilters":[{"Key":"Name","Option":"Equals","Values":["/app/db-password"]}]}"#,
                ),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/db-password","Tier":"Standard"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(SsmQuery)
            .data(SsmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ parameters(names: ["/app/db-password"], withDecryption: true) { value tier } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let param = &json["parameters"][0];
        assert_eq!(param["value"], "hunter2");
        assert_eq!(param["tier"], "STANDARD");
        http_client.relaxed_requests_match();
    }

    // --- parametersByPath (same masking + tier fan-out logic as
    // `parameters`, plus a resolver-local `recursive.unwrap_or(true)`
    // default) ---

    #[tokio::test]
    async fn parameters_by_path_masks_secure_string_and_defaults_recursive_true() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"Path":"/app/","Recursive":true,"WithDecryption":false}"#,
                ),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/x","Type":"SecureString","Value":"s3cr3t"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"ParameterFilters":[{"Key":"Name","Option":"Equals","Values":["/app/x"]}]}"#,
                ),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/x","Tier":"Intelligent-Tiering"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(SsmQuery)
            .data(SsmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ parametersByPath(path: "/app/") { items { name value tier } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["parametersByPath"]["items"][0];
        assert_eq!(item["name"], "/app/x");
        assert_eq!(item["value"], "***");
        assert_eq!(item["tier"], "INTELLIGENT_TIERING");
        assert_eq!(
            json["parametersByPath"]["nextToken"],
            serde_json::Value::Null
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn parameter_metadata_maps_fields_and_forwards_filters_and_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"ParameterFilters":[{"Key":"Name","Option":"Equals","Values":["/app/x"]}],"MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Parameters":[{"Name":"/app/x","Type":"String","Tier":"Advanced","Version":3,"Description":"Connection timeout in seconds","ARN":"arn:aws:ssm:us-east-1:123456789012:parameter/app/x","DataType":"text","KeyId":"alias/my-key"}],"NextToken":"p2"}"#,
            ),
        )]);
        let schema = build_query_schema(SsmQuery)
            .data(SsmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ parameterMetadata(filters: [{ key: "Name", option: "Equals", values: ["/app/x"] }], limit: 1) {
                    items { name parameterType tier version description arn dataType keyId policies }
                    nextToken
                } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["parameterMetadata"]["items"][0];
        assert_eq!(item["name"], "/app/x");
        assert_eq!(item["parameterType"], "STRING");
        assert_eq!(item["tier"], "ADVANCED");
        assert_eq!(item["version"], 3);
        assert_eq!(item["description"], "Connection timeout in seconds");
        assert_eq!(
            item["arn"],
            "arn:aws:ssm:us-east-1:123456789012:parameter/app/x"
        );
        assert_eq!(item["dataType"], "text");
        assert_eq!(item["keyId"], "alias/my-key");
        assert_eq!(json["parameterMetadata"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn documents_maps_fields_and_forwards_filters_and_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Filters":[{"Key":"Owner","Values":["Self"]},{"Key":"DocumentType","Values":["Command"]},{"Key":"Name","Values":["my-doc"]}],"MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"DocumentIdentifiers":[{"Name":"my-doc","DocumentType":"Command","DocumentFormat":"JSON","DocumentVersion":"1","Owner":"Self","CreatedDate":1600000000,"PlatformTypes":["Linux"],"SchemaVersion":"2.2","TargetType":"/AWS::EC2::Instance","Tags":[{"Key":"Environment","Value":"prod"}]}],"NextToken":"p2"}"#,
            ),
        )]);
        let schema = build_query_schema(SsmQuery)
            .data(SsmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ documents(owner: "Self", documentType: "Command", name: "my-doc", limit: 1) {
                    items { name documentType documentFormat documentVersion owner createdDate platformTypes schemaVersion targetType tags { key value } }
                    nextToken
                } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["documents"]["items"][0];
        assert_eq!(item["name"], "my-doc");
        assert_eq!(item["documentType"], "Command");
        assert_eq!(item["documentFormat"], "JSON");
        assert_eq!(item["documentVersion"], "1");
        assert_eq!(item["owner"], "Self");
        assert!(item["createdDate"].is_string());
        assert_eq!(item["platformTypes"][0], "Linux");
        assert_eq!(item["schemaVersion"], "2.2");
        assert_eq!(item["targetType"], "/AWS::EC2::Instance");
        assert_eq!(item["tags"][0]["key"], "Environment");
        assert_eq!(item["tags"][0]["value"], "prod");
        assert_eq!(json["documents"]["nextToken"], "p2");
        http_client.relaxed_requests_match();
    }
}
