use aws_config::SdkConfig;

use crate::error::VaporError;

#[derive(Debug)]
pub struct LicenseConfigurationInfo {
    pub license_configuration_id: Option<String>,
    pub license_configuration_arn: Option<String>,
    pub name: Option<String>,
    pub license_counting_type: Option<String>,
    pub license_count: Option<i64>,
    pub license_count_hard_limit: Option<bool>,
    pub consumed_licenses: Option<i64>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub product_information_list: Vec<String>,
}

#[derive(Debug)]
pub struct LicenseGrantInfo {
    pub grant_arn: Option<String>,
    pub grant_name: Option<String>,
    pub parent_arn: Option<String>,
    pub license_arn: Option<String>,
    pub grantee_principal_arn: Option<String>,
    pub home_region: Option<String>,
    pub grant_status: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug)]
pub struct LicenseInfo {
    pub license_arn: Option<String>,
    pub license_name: Option<String>,
    pub product_name: Option<String>,
    pub product_sku: Option<String>,
    pub issuer: Option<String>,
    pub status: Option<String>,
    pub validity_period_start: Option<String>,
    pub validity_period_end: Option<String>,
}

pub struct LicenseManagerClient {
    inner: aws_sdk_licensemanager::Client,
}

impl LicenseManagerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_licensemanager::Client::new(config),
        }
    }

    /// Lists license configurations, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListLicenseConfigurations`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-licensemanager` 1.106.0's
    /// `operation/list_license_configurations/_list_license_configurations_input.rs`
    /// — the earlier claim of "no SDK paginator" only meant no *generated*
    /// paginator, the request-level `max_results` field was there all along),
    /// so `limit` is capped to the remaining budget on the request itself,
    /// matching `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_license_configurations(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LicenseConfigurationInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_license_configurations();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for cfg in output.license_configurations() {
                // `resource_type` is a required field on every `ProductInformation`
                // entry (verified against `aws-sdk-licensemanager` 1.106.0's
                // `_product_information.rs`), so mapping on it (instead of an
                // arbitrary nested filter name) both preserves one entry per
                // product information item and surfaces the most meaningful
                // single string (e.g. "SSM_MANAGED", "RDS") per entry.
                let product_information_list: Vec<String> = cfg
                    .product_information_list()
                    .iter()
                    .map(|pi| pi.resource_type().to_string())
                    .collect();

                items.push(LicenseConfigurationInfo {
                    license_configuration_id: cfg
                        .license_configuration_id()
                        .map(|s| s.to_string()),
                    license_configuration_arn: cfg
                        .license_configuration_arn()
                        .map(|s| s.to_string()),
                    name: cfg.name().map(|s| s.to_string()),
                    license_counting_type: cfg
                        .license_counting_type()
                        .map(|t| t.as_str().to_string()),
                    license_count: cfg.license_count(),
                    license_count_hard_limit: cfg.license_count_hard_limit(),
                    consumed_licenses: cfg.consumed_licenses(),
                    status: cfg.status().map(|s| s.to_string()),
                    description: cfg.description().map(|s| s.to_string()),
                    product_information_list,
                });
            }
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists licenses, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListLicenses` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-licensemanager` 1.106.0's
    /// `operation/list_licenses/_list_licenses_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_licenses(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LicenseInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_licenses();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for license in output.licenses() {
                let (validity_start, validity_end) =
                    if let Some(validity) = license.validity() {
                        (
                            Some(validity.begin().to_string()),
                            validity.end().map(|s| s.to_string()),
                        )
                    } else {
                        (None, None)
                    };

                items.push(LicenseInfo {
                    license_arn: license.license_arn().map(|s| s.to_string()),
                    license_name: license.license_name().map(|s| s.to_string()),
                    product_name: license.product_name().map(|s| s.to_string()),
                    product_sku: license.product_sku().map(|s| s.to_string()),
                    issuer: license.issuer().and_then(|i| i.name()).map(|s| s.to_string()),
                    status: license.status().map(|s| s.as_str().to_string()),
                    validity_period_start: validity_start,
                    validity_period_end: validity_end,
                });
            }
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists received grants, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListReceivedGrants` has
    /// both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-licensemanager` 1.106.0's
    /// `operation/list_received_grants/_list_received_grants_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself,
    /// matching `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_received_grants(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LicenseGrantInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_received_grants();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for grant in output.grants() {
                items.push(LicenseGrantInfo {
                    grant_arn: Some(grant.grant_arn().to_string()),
                    grant_name: Some(grant.grant_name().to_string()),
                    parent_arn: Some(grant.parent_arn().to_string()),
                    license_arn: Some(grant.license_arn().to_string()),
                    grantee_principal_arn: Some(grant.grantee_principal_arn().to_string()),
                    home_region: Some(grant.home_region().to_string()),
                    grant_status: Some(grant.grant_status().as_str().to_string()),
                    version: Some(grant.version().to_string()),
                });
            }
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://license-manager.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_license_configurations_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"LicenseConfigurations":[{"LicenseConfigurationId":"lic-conf-1","LicenseConfigurationArn":"arn:aws:license-manager:us-east-1:123456789012:license-configuration:lic-conf-1","Name":"my-config","LicenseCountingType":"vCPU","LicenseCount":10,"LicenseCountHardLimit":true,"ConsumedLicenses":3,"Status":"AVAILABLE","Description":"a config","ProductInformationList":[{"ResourceType":"SSM_MANAGED"},{"ResourceType":"RDS"}]}]}"#,
            ),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_license_configurations(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let cfg = &items[0];
        assert_eq!(cfg.license_configuration_id, Some("lic-conf-1".to_string()));
        assert_eq!(
            cfg.license_configuration_arn,
            Some("arn:aws:license-manager:us-east-1:123456789012:license-configuration:lic-conf-1".to_string())
        );
        assert_eq!(cfg.name, Some("my-config".to_string()));
        assert_eq!(cfg.license_counting_type, Some("vCPU".to_string()));
        assert_eq!(cfg.license_count, Some(10));
        assert_eq!(cfg.license_count_hard_limit, Some(true));
        assert_eq!(cfg.consumed_licenses, Some(3));
        assert_eq!(cfg.status, Some("AVAILABLE".to_string()));
        assert_eq!(cfg.description, Some("a config".to_string()));
        assert_eq!(cfg.product_information_list, vec!["SSM_MANAGED".to_string(), "RDS".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_license_configurations_maps_minimal_config_with_no_optional_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"LicenseConfigurations":[{}]}"#),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.list_license_configurations(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let cfg = &items[0];
        assert_eq!(cfg.license_configuration_id, None);
        assert_eq!(cfg.license_configuration_arn, None);
        assert_eq!(cfg.name, None);
        assert_eq!(cfg.license_counting_type, None);
        assert_eq!(cfg.license_count, None);
        assert_eq!(cfg.license_count_hard_limit, None);
        assert_eq!(cfg.consumed_licenses, None);
        assert_eq!(cfg.status, None);
        assert_eq!(cfg.description, None);
        assert_eq!(cfg.product_information_list, Vec::<String>::new());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_license_configurations_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"LicenseConfigurations":[{"LicenseConfigurationId":"lic-conf-2"}]}"#),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_license_configurations(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].license_configuration_id, Some("lic-conf-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_license_configurations_stops_at_limit_and_returns_resume_token() {
        // ListLicenseConfigurations forwards `MaxResults` straight to AWS
        // with no client-side truncation, so the canned response must
        // return exactly the requested count, not more (durable gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"LicenseConfigurations":[{"LicenseConfigurationId":"lic-conf-1"},{"LicenseConfigurationId":"lic-conf-2"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_license_configurations(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_license_configurations_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"LicenseConfigurations":[{"LicenseConfigurationId":"lic-conf-1"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":9}"#),
                json_response(200, r#"{"LicenseConfigurations":[{"LicenseConfigurationId":"lic-conf-2"}]}"#),
            ),
        ]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_license_configurations(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_license_configurations_propagates_errors() {
        // `InvalidParameterValueException`, not a throttling-classified
        // code (durable gotcha 1: those get retried and exhaust the single
        // replay event).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidParameterValueException", "bad parameter"),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_license_configurations(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_licenses_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Licenses":[{"LicenseArn":"arn:aws:license-manager::123456789012:license:l-1","LicenseName":"my-license","ProductName":"my-product","ProductSKU":"sku-1","Issuer":{"Name":"issuer-1"},"Status":"AVAILABLE","Validity":{"Begin":"2026-01-01T00:00:00Z","End":"2027-01-01T00:00:00Z"}}]}"#,
            ),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_licenses(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let license = &items[0];
        assert_eq!(license.license_arn, Some("arn:aws:license-manager::123456789012:license:l-1".to_string()));
        assert_eq!(license.license_name, Some("my-license".to_string()));
        assert_eq!(license.product_name, Some("my-product".to_string()));
        assert_eq!(license.product_sku, Some("sku-1".to_string()));
        assert_eq!(license.issuer, Some("issuer-1".to_string()));
        assert_eq!(license.status, Some("AVAILABLE".to_string()));
        assert_eq!(license.validity_period_start, Some("2026-01-01T00:00:00Z".to_string()));
        assert_eq!(license.validity_period_end, Some("2027-01-01T00:00:00Z".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_licenses_maps_license_with_no_validity_or_issuer() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"Licenses":[{"LicenseArn":"arn:aws:license-manager::123456789012:license:l-2"}]}"#),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.list_licenses(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let license = &items[0];
        assert_eq!(license.issuer, None);
        assert_eq!(license.validity_period_start, None);
        assert_eq!(license.validity_period_end, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_licenses_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Licenses":[{"LicenseArn":"arn:aws:license-manager::123456789012:license:l-3"}]}"#),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_licenses(None, Some("cursor-a".to_string())).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_licenses_stops_at_limit_and_returns_resume_token() {
        // ListLicenses forwards `MaxResults` straight to AWS with no
        // client-side truncation (durable gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Licenses":[{"LicenseArn":"l-1"},{"LicenseArn":"l-2"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_licenses(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_licenses_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(200, r#"{"Licenses":[{"LicenseArn":"l-1"}],"NextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":9}"#),
                json_response(200, r#"{"Licenses":[{"LicenseArn":"l-2"}]}"#),
            ),
        ]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_licenses(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_licenses_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidParameterValueException", "bad parameter"),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_licenses(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_received_grants_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Grants":[{"GrantArn":"arn:aws:license-manager::123456789012:grant:g-1","GrantName":"my-grant","ParentArn":"arn:aws:license-manager::123456789012:license:l-1","LicenseArn":"arn:aws:license-manager::123456789012:license:l-1","GranteePrincipalArn":"arn:aws:iam::123456789012:root","HomeRegion":"us-east-1","GrantStatus":"ACTIVE","Version":"1"}]}"#,
            ),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_received_grants(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let grant = &items[0];
        assert_eq!(grant.grant_arn, Some("arn:aws:license-manager::123456789012:grant:g-1".to_string()));
        assert_eq!(grant.grant_name, Some("my-grant".to_string()));
        assert_eq!(grant.parent_arn, Some("arn:aws:license-manager::123456789012:license:l-1".to_string()));
        assert_eq!(grant.license_arn, Some("arn:aws:license-manager::123456789012:license:l-1".to_string()));
        assert_eq!(grant.grantee_principal_arn, Some("arn:aws:iam::123456789012:root".to_string()));
        assert_eq!(grant.home_region, Some("us-east-1".to_string()));
        assert_eq!(grant.grant_status, Some("ACTIVE".to_string()));
        assert_eq!(grant.version, Some("1".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_received_grants_maps_minimal_grant_with_correct_errors_defaults() {
        // `Grant`'s fields are all non-`Option` accessors (`&str`/`&GrantStatus`
        // rather than `Option<&str>`) because `grant_correct_errors`
        // unconditionally default-fills every field before `.build()`
        // (durable gotcha 16). An absent `GrantStatus` defaults to the
        // sentinel string "no value was set" parsed into `GrantStatus::Unknown`,
        // whose `.as_str()` echoes that same sentinel back out.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"Grants":[{}]}"#),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.list_received_grants(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let grant = &items[0];
        assert_eq!(grant.grant_arn, Some(String::new()));
        assert_eq!(grant.grant_name, Some(String::new()));
        assert_eq!(grant.grant_status, Some("no value was set".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_received_grants_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Grants":[{"GrantArn":"g-2"}]}"#),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_received_grants(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_received_grants_stops_at_limit_and_returns_resume_token() {
        // ListReceivedGrants forwards `MaxResults` straight to AWS with no
        // client-side truncation (durable gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(200, r#"{"Grants":[{"GrantArn":"g-1"},{"GrantArn":"g-2"}],"NextToken":"page2"}"#),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_received_grants(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_received_grants_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(200, r#"{"Grants":[{"GrantArn":"g-1"}],"NextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":9}"#),
                json_response(200, r#"{"Grants":[{"GrantArn":"g-2"}]}"#),
            ),
        ]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_received_grants(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_received_grants_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidParameterValueException", "bad parameter"),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_received_grants(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

