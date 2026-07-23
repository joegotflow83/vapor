use async_graphql::{Context, Object, Result};

use crate::aws::license_manager::LicenseManagerClient;
use crate::schema::license_manager::types::{License, LicenseConfiguration, LicenseGrant};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct LicenseManagerQuery;

#[Object]
impl LicenseManagerQuery {
    /// Lists license configurations, optionally capped at `limit` results
    /// (default unlimited) and resumed from `nextToken`.
    async fn license_configurations(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LicenseConfiguration>> {
        let client = ctx.data::<LicenseManagerClient>()?;
        let (items, next_token) = client.list_license_configurations(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(LicenseConfiguration::from).collect(),
            next_token,
        })
    }

    /// Lists licenses, optionally capped at `limit` results (default
    /// unlimited) and resumed from `nextToken`.
    async fn licenses(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<License>> {
        let client = ctx.data::<LicenseManagerClient>()?;
        let (items, next_token) = client.list_licenses(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(License::from).collect(),
            next_token,
        })
    }

    /// Lists received license grants, optionally capped at `limit` results
    /// (default unlimited) and resumed from `nextToken`.
    async fn license_grants(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LicenseGrant>> {
        let client = ctx.data::<LicenseManagerClient>()?;
        let (items, next_token) = client.list_received_grants(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(LicenseGrant::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::license_manager::LicenseManagerClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::LicenseManagerQuery;

    const ENDPOINT: &str = "https://license-manager.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn license_configurations_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"LicenseConfigurations":[{"LicenseConfigurationId":"lic-conf-1","Name":"my-config","LicenseCountingType":"vCPU","LicenseCount":10,"Status":"AVAILABLE","ProductInformationList":[{"ResourceType":"SSM_MANAGED"}]}],"NextToken":"cursor-a"}"#,
            ),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LicenseManagerQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ licenseConfigurations(limit: 1) { items { licenseConfigurationId name licenseCountingType licenseCount status productInformationList } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["licenseConfigurations"]["items"][0]["licenseConfigurationId"], "lic-conf-1");
        assert_eq!(data["licenseConfigurations"]["items"][0]["name"], "my-config");
        assert_eq!(data["licenseConfigurations"]["items"][0]["licenseCountingType"], "vCPU");
        assert_eq!(data["licenseConfigurations"]["items"][0]["licenseCount"], 10);
        assert_eq!(data["licenseConfigurations"]["items"][0]["status"], "AVAILABLE");
        assert_eq!(data["licenseConfigurations"]["items"][0]["productInformationList"][0], "SSM_MANAGED");
        assert_eq!(data["licenseConfigurations"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn licenses_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Licenses":[{"LicenseArn":"arn:aws:license-manager::123456789012:license:l-1","LicenseName":"my-license","ProductName":"my-product","ProductSKU":"sku-1","Issuer":{"Name":"issuer-1"},"Status":"AVAILABLE","Validity":{"Begin":"2026-01-01T00:00:00Z","End":"2027-01-01T00:00:00Z"}}],"NextToken":"cursor-b"}"#,
            ),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LicenseManagerQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ licenses(limit: 1) { items { licenseArn licenseName productName productSku issuer status validityPeriodStart validityPeriodEnd } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["licenses"]["items"][0]["licenseArn"], "arn:aws:license-manager::123456789012:license:l-1");
        assert_eq!(data["licenses"]["items"][0]["licenseName"], "my-license");
        assert_eq!(data["licenses"]["items"][0]["productName"], "my-product");
        assert_eq!(data["licenses"]["items"][0]["productSku"], "sku-1");
        assert_eq!(data["licenses"]["items"][0]["issuer"], "issuer-1");
        assert_eq!(data["licenses"]["items"][0]["status"], "AVAILABLE");
        assert_eq!(data["licenses"]["items"][0]["validityPeriodStart"], "2026-01-01T00:00:00Z");
        assert_eq!(data["licenses"]["items"][0]["validityPeriodEnd"], "2027-01-01T00:00:00Z");
        assert_eq!(data["licenses"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn license_grants_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Grants":[{"GrantArn":"arn:aws:license-manager::123456789012:grant:g-1","GrantName":"my-grant","ParentArn":"arn:aws:license-manager::123456789012:license:l-1","LicenseArn":"arn:aws:license-manager::123456789012:license:l-1","GranteePrincipalArn":"arn:aws:iam::123456789012:root","HomeRegion":"us-east-1","GrantStatus":"ACTIVE","Version":"1"}],"NextToken":"cursor-c"}"#,
            ),
        )]);
        let client = LicenseManagerClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LicenseManagerQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ licenseGrants(limit: 1) { items { grantArn grantName parentArn licenseArn granteePrincipalArn homeRegion grantStatus version } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["licenseGrants"]["items"][0]["grantArn"], "arn:aws:license-manager::123456789012:grant:g-1");
        assert_eq!(data["licenseGrants"]["items"][0]["grantName"], "my-grant");
        assert_eq!(data["licenseGrants"]["items"][0]["parentArn"], "arn:aws:license-manager::123456789012:license:l-1");
        assert_eq!(data["licenseGrants"]["items"][0]["licenseArn"], "arn:aws:license-manager::123456789012:license:l-1");
        assert_eq!(data["licenseGrants"]["items"][0]["granteePrincipalArn"], "arn:aws:iam::123456789012:root");
        assert_eq!(data["licenseGrants"]["items"][0]["homeRegion"], "us-east-1");
        assert_eq!(data["licenseGrants"]["items"][0]["grantStatus"], "ACTIVE");
        assert_eq!(data["licenseGrants"]["items"][0]["version"], "1");
        assert_eq!(data["licenseGrants"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }
}
