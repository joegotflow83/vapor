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

    /// Describes managed instance information, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `DescribeInstanceInformation` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-ssm` 1.113.0's
    /// `operation/describe_instance_information/
    /// _describe_instance_information_output.rs`, which also documents that
    /// `next_token` comes back as an empty string, not absent, on the last
    /// page). Filters are built once, cloned into the request each loop
    /// iteration (fsx/transcribe/ses precedent).
    pub async fn describe_instance_information(
        &self,
        instance_ids: Option<Vec<String>>,
        ping_status: Option<String>,
        platform_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ssm::types::InstanceInformation>, Option<String>), VaporError> {
        let mut filters: Vec<aws_sdk_ssm::types::InstanceInformationStringFilter> = Vec::new();

        if let Some(ids) = instance_ids {
            if !ids.is_empty() {
                filters.push(
                    aws_sdk_ssm::types::InstanceInformationStringFilter::builder()
                        .key("InstanceIds")
                        .set_values(Some(ids))
                        .build()
                        .map_err(|e| VaporError::AwsSdk { code: None, message: e.to_string() })?,
                );
            }
        }

        if let Some(status) = ping_status {
            filters.push(
                aws_sdk_ssm::types::InstanceInformationStringFilter::builder()
                    .key("PingStatus")
                    .values(status)
                    .build()
                    .map_err(|e| VaporError::AwsSdk { code: None, message: e.to_string() })?,
            );
        }

        if let Some(platform) = platform_type {
            filters.push(
                aws_sdk_ssm::types::InstanceInformationStringFilter::builder()
                    .key("PlatformType")
                    .values(platform)
                    .build()
                    .map_err(|e| VaporError::AwsSdk { code: None, message: e.to_string() })?,
            );
        }

        let mut all_items: Vec<aws_sdk_ssm::types::InstanceInformation> = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self.inner.describe_instance_information();
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_items.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_items.extend(output.instance_information_list.unwrap_or_default());
            token = output.next_token.filter(|t| !t.is_empty());

            if token.is_none() || limit.is_some_and(|l| all_items.len() as i32 >= l) {
                break;
            }
        }

        Ok((all_items, token))
    }

    /// Gets named parameters (caller-supplied IDs, AWS-capped at 10 names
    /// per call). `GetParametersInput`/`Output` have no `max_results`/
    /// `next_token` fields at all (verified against pinned `aws-sdk-ssm`
    /// 1.113.0's `operation/get_parameters/*.rs`) — a genuinely
    /// non-paginated, single-call op (sts/opensearch-class carve-out),
    /// hence no `limit`/`next_token` params here.
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
            .map_err(crate::error::sdk_err)?;

        Ok(output.parameters().to_vec())
    }

    /// Lists parameters under `path`, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `GetParametersByPath` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-ssm` 1.113.0), standard
    /// kinesis/translate server-side-capping shape.
    pub async fn get_parameters_by_path(
        &self,
        path: String,
        recursive: bool,
        with_decryption: bool,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ssm::types::Parameter>, Option<String>), VaporError> {
        let mut all_params: Vec<aws_sdk_ssm::types::Parameter> = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self
                .inner
                .get_parameters_by_path()
                .path(&path)
                .recursive(recursive)
                .with_decryption(with_decryption);
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_params.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_params.extend(output.parameters.unwrap_or_default());
            token = output.next_token.filter(|t| !t.is_empty());

            if token.is_none() || limit.is_some_and(|l| all_params.len() as i32 >= l) {
                break;
            }
        }

        Ok((all_params, token))
    }

    /// Describes parameter metadata, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `DescribeParameters`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-ssm` 1.113.0), standard kinesis/translate
    /// server-side-capping shape.
    pub async fn describe_parameters(
        &self,
        filters: Option<Vec<aws_sdk_ssm::types::ParameterStringFilter>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ssm::types::ParameterMetadata>, Option<String>), VaporError> {
        let mut all_params: Vec<aws_sdk_ssm::types::ParameterMetadata> = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self.inner.describe_parameters().set_parameter_filters(filters.clone());
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_params.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_params.extend(output.parameters.unwrap_or_default());
            token = output.next_token.filter(|t| !t.is_empty());

            if token.is_none() || limit.is_some_and(|l| all_params.len() as i32 >= l) {
                break;
            }
        }

        Ok((all_params, token))
    }

    /// Fetches the tier for each of `names` by paging through
    /// `describe_parameters` (a `Name Equals` filter) to exhaustion — an
    /// internal aggregation helper, not a caller-facing resumable query, so
    /// it always fetches every matching page rather than taking its own
    /// `limit`/`next_token`.
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

        let mut all_metadata = Vec::new();
        let mut token = None;
        loop {
            let (page, next) = self
                .describe_parameters(Some(vec![filter.clone()]), None, token)
                .await?;
            all_metadata.extend(page);
            match next {
                Some(t) => token = Some(t),
                None => break,
            }
        }

        Ok(all_metadata
            .into_iter()
            .filter_map(|m| {
                let name = m.name()?.to_string();
                let tier = m.tier()?.clone();
                Some((name, tier))
            })
            .collect())
    }

    /// Lists SSM documents, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListDocuments` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-ssm` 1.113.0's `operation/list_documents/
    /// _list_documents_output.rs`, which also documents that `next_token`
    /// comes back as an empty string, not absent, on the last page).
    /// Filters are built once, cloned into the request each loop iteration
    /// (fsx/transcribe/ses precedent).
    pub async fn list_documents(
        &self,
        owner: Option<String>,
        document_type: Option<String>,
        name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ssm::types::DocumentIdentifier>, Option<String>), VaporError> {
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
        let mut token = next_token;

        loop {
            let mut request = self.inner.list_documents();
            if !filters.is_empty() {
                request = request.set_filters(Some(filters.clone()));
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_results(l - all_docs.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_docs.extend(output.document_identifiers.unwrap_or_default());
            token = output.next_token.filter(|t| !t.is_empty());

            if token.is_none() || limit.is_some_and(|l| all_docs.len() as i32 >= l) {
                break;
            }
        }

        Ok((all_docs, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    // awsJson1.1: POST JSON to a fixed `/` path, differentiated only by the
    // `x-amz-target` header (which `test_util::request` doesn't compare),
    // same shape as `ram.rs`/`service_quotas.rs`. Crate name (`aws-sdk-ssm`)
    // matches the endpoint hostname (`ssm.*`, verified against pinned
    // `aws-sdk-ssm` 1.113.0's `config/endpoint.rs`). Response bodies use
    // PascalCase keys throughout, with the "ARN" acronym staying all-caps
    // inside `Parameter`/`ParameterMetadata` (untested here since no method
    // surfaces `.arn()`). No `serde_util.rs` fn in this crate touches any of
    // `InstanceInformation`/`Parameter`/`ParameterMetadata`/
    // `DocumentIdentifier` (grepped, no hits for those types) — every
    // `Option<T>` field genuinely stays `None` on a missing key. `Parameter`
    // and `InstanceInformation` don't derive `Debug` (only `Clone`/
    // `PartialEq`), unlike `ParameterMetadata`/`DocumentIdentifier` which do
    // (gotcha 6) — error tests for `describe_instance_information`/
    // `get_parameters`/`get_parameters_by_path` match the `Result` directly
    // instead of `.unwrap_err()`.
    const BASE: &str = "https://ssm.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_instance_information_lists_all_when_no_limit_or_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"InstanceInformationList":[{"InstanceId":"i-1","PingStatus":"Online"}]}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_instance_information(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].instance_id(), Some("i-1"));
        assert_eq!(items[0].ping_status().map(|s| s.as_str()), Some("Online"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instance_information_builds_filters_from_instance_ids_ping_status_and_platform_type() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Filters":[{"Key":"InstanceIds","Values":["i-1","i-2"]},{"Key":"PingStatus","Values":["Online"]},{"Key":"PlatformType","Values":["Linux"]}]}"#,
            ),
            json_response(200, r#"{"InstanceInformationList":[]}"#),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .describe_instance_information(
                Some(vec!["i-1".to_string(), "i-2".to_string()]),
                Some("Online".to_string()),
                Some("Linux".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instance_information_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"InstanceInformationList":[{"InstanceId":"i-1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_instance_information(None, None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instance_information_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"InstanceInformationList":[{"InstanceId":"i-1"},{"InstanceId":"i-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"InstanceInformationList":[{"InstanceId":"i-3"}]}"#),
            ),
        ]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_instance_information(None, None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instance_information_treats_empty_next_token_as_end_of_pages() {
        // Doc comment claims the last page's `NextToken` comes back as an
        // empty string, not absent -- confirm `token.filter(|t|
        // !t.is_empty())` actually converts it to `None`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"InstanceInformationList":[{"InstanceId":"i-1"}],"NextToken":""}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_instance_information(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_instance_information_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidFilterKey", "unknown filter key"),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let result = client
            .describe_instance_information(None, None, None, None, None)
            .await;

        match result {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidFilterKey".to_string()));
                assert_eq!(message, "unknown filter key");
            }
            other => panic!("expected Err(VaporError::AwsSdk), got {}", other.is_ok()),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameters_fetches_named_parameters_with_decryption() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Names":["/app/db-password"],"WithDecryption":true}"#),
            json_response(
                200,
                r#"{"Parameters":[{"Name":"/app/db-password","Type":"SecureString","Value":"hunter2"}]}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let params = client
            .get_parameters(vec!["/app/db-password".to_string()], true)
            .await
            .unwrap();

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name(), Some("/app/db-password"));
        assert_eq!(
            params[0].r#type().map(|t| t.as_str()),
            Some("SecureString")
        );
        assert_eq!(params[0].value(), Some("hunter2"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameters_without_decryption_returns_empty_when_none_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Names":["/missing"],"WithDecryption":false}"#),
            json_response(200, r#"{"Parameters":[]}"#),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let params = client
            .get_parameters(vec!["/missing".to_string()], false)
            .await
            .unwrap();

        assert_eq!(params.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameters_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Names":["/app/x"],"WithDecryption":false}"#),
            json_error_response("InvalidKeyId", "bad kms key"),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let result = client.get_parameters(vec!["/app/x".to_string()], false).await;

        match result {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidKeyId".to_string()));
                assert_eq!(message, "bad kms key");
            }
            other => panic!("expected Err(VaporError::AwsSdk), got {}", other.is_ok()),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameters_by_path_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Path":"/app/","Recursive":false,"WithDecryption":false}"#,
            ),
            json_response(200, r#"{"Parameters":[{"Name":"/app/x","Value":"1"}]}"#),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (params, token) = client
            .get_parameters_by_path("/app/".to_string(), false, false, None, None)
            .await
            .unwrap();

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name(), Some("/app/x"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameters_by_path_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Path":"/app/","Recursive":true,"WithDecryption":true,"MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Parameters":[{"Name":"/app/x"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (params, token) = client
            .get_parameters_by_path("/app/".to_string(), true, true, Some(1), None)
            .await
            .unwrap();

        assert_eq!(params.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameters_by_path_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"Path":"/app/","Recursive":false,"WithDecryption":false,"MaxResults":10}"#,
                ),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/x"},{"Name":"/app/y"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"Path":"/app/","Recursive":false,"WithDecryption":false,"NextToken":"p2","MaxResults":8}"#,
                ),
                json_response(200, r#"{"Parameters":[{"Name":"/app/z"}]}"#),
            ),
        ]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (params, token) = client
            .get_parameters_by_path("/app/".to_string(), false, false, Some(10), None)
            .await
            .unwrap();

        assert_eq!(params.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameters_by_path_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Path":"/bogus","Recursive":false,"WithDecryption":false}"#,
            ),
            json_error_response("InvalidFilterKey", "unknown filter key"),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let result = client
            .get_parameters_by_path("/bogus".to_string(), false, false, None, None)
            .await;

        match result {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidFilterKey".to_string()));
                assert_eq!(message, "unknown filter key");
            }
            other => panic!("expected Err(VaporError::AwsSdk), got {}", other.is_ok()),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_parameters_lists_all_when_no_filters_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Parameters":[{"Name":"/app/x","Tier":"Standard"}]}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (params, token) = client.describe_parameters(None, None, None).await.unwrap();

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name(), Some("/app/x"));
        assert_eq!(params[0].tier().map(|t| t.as_str()), Some("Standard"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_parameters_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Parameters":[{"Name":"/app/x"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (params, token) = client
            .describe_parameters(None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(params.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_parameters_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/x"},{"Name":"/app/y"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Parameters":[{"Name":"/app/z"}]}"#),
            ),
        ]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (params, token) = client
            .describe_parameters(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(params.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_parameters_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidFilterKey", "unknown filter key"),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_parameters(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidFilterKey".to_string()));
                assert_eq!(message, "unknown filter key");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameter_tiers_returns_empty_map_for_empty_names() {
        let http_client = StaticReplayClient::new(vec![]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let tiers = client.get_parameter_tiers(&[]).await.unwrap();

        assert!(tiers.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameter_tiers_builds_name_equals_filter_and_extracts_tiers() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"ParameterFilters":[{"Key":"Name","Option":"Equals","Values":["/app/x","/app/y"]}]}"#,
            ),
            json_response(
                200,
                r#"{"Parameters":[{"Name":"/app/x","Tier":"Standard"},{"Name":"/app/y","Tier":"Advanced"}]}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let tiers = client
            .get_parameter_tiers(&["/app/x".to_string(), "/app/y".to_string()])
            .await
            .unwrap();

        assert_eq!(
            tiers.get("/app/x").map(|t| t.as_str()),
            Some("Standard")
        );
        assert_eq!(
            tiers.get("/app/y").map(|t| t.as_str()),
            Some("Advanced")
        );
        assert_eq!(tiers.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameter_tiers_pages_through_multiple_describe_parameters_calls() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"ParameterFilters":[{"Key":"Name","Option":"Equals","Values":["/app/x"]}]}"#,
                ),
                json_response(
                    200,
                    r#"{"Parameters":[{"Name":"/app/x","Tier":"Standard"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"ParameterFilters":[{"Key":"Name","Option":"Equals","Values":["/app/x"]}],"NextToken":"p2"}"#,
                ),
                json_response(200, r#"{"Parameters":[]}"#),
            ),
        ]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let tiers = client
            .get_parameter_tiers(&["/app/x".to_string()])
            .await
            .unwrap();

        assert_eq!(tiers.get("/app/x").map(|t| t.as_str()), Some("Standard"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_parameter_tiers_skips_entries_missing_name_or_tier() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"ParameterFilters":[{"Key":"Name","Option":"Equals","Values":["/app/x"]}]}"#,
            ),
            json_response(
                200,
                r#"{"Parameters":[{"Tier":"Standard"},{"Name":"/app/no-tier"},{"Name":"/app/x","Tier":"IntelligentTiering"}]}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let tiers = client
            .get_parameter_tiers(&["/app/x".to_string()])
            .await
            .unwrap();

        assert_eq!(tiers.len(), 1);
        assert_eq!(
            tiers.get("/app/x").map(|t| t.as_str()),
            Some("IntelligentTiering")
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_documents_lists_all_when_no_filters_or_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"DocumentIdentifiers":[{"Name":"my-doc","Owner":"self"}]}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (docs, token) = client
            .list_documents(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].name(), Some("my-doc"));
        assert_eq!(docs[0].owner(), Some("self"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_documents_builds_filters_from_owner_document_type_and_name() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Filters":[{"Key":"Owner","Values":["Self"]},{"Key":"DocumentType","Values":["Command"]},{"Key":"Name","Values":["my-doc"]}]}"#,
            ),
            json_response(200, r#"{"DocumentIdentifiers":[]}"#),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (docs, _token) = client
            .list_documents(
                Some("Self".to_string()),
                Some("Command".to_string()),
                Some("my-doc".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(docs.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_documents_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"DocumentIdentifiers":[{"Name":"my-doc"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (docs, token) = client
            .list_documents(None, None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(docs.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_documents_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"DocumentIdentifiers":[{"Name":"doc-1"},{"Name":"doc-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"DocumentIdentifiers":[{"Name":"doc-3"}]}"#),
            ),
        ]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let (docs, token) = client
            .list_documents(None, None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(docs.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_documents_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidFilterKey", "unknown filter key"),
        )]);
        let client = SsmClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_documents(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidFilterKey".to_string()));
                assert_eq!(message, "unknown filter key");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
