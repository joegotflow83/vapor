use async_graphql::{Context, Object, Result};

use crate::aws::service_quotas::ServiceQuotasClient;
use crate::schema::pagination::Page;
use crate::schema::service_quotas::types::ServiceQuota;

#[derive(Default)]
pub struct ServiceQuotasQuery;

#[Object]
impl ServiceQuotasQuery {
    /// `limit` caps the number of quotas returned per call (default
    /// unlimited); pass the returned `nextToken` back to resume.
    async fn service_quotas(
        &self,
        ctx: &Context<'_>,
        service_code: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ServiceQuota>> {
        let client = ctx.data::<ServiceQuotasClient>()?;
        let (quotas, next_token) = client
            .list_service_quotas(&service_code, limit, next_token)
            .await?;
        Ok(Page {
            items: quotas.iter().map(ServiceQuota::from).collect(),
            next_token,
        })
    }

    /// `limit` caps the number of services returned per call (default
    /// unlimited); pass the returned `nextToken` back to resume.
    async fn service_quota_services(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<String>> {
        let client = ctx.data::<ServiceQuotasClient>()?;
        let (services, next_token) = client.list_services(limit, next_token).await?;
        Ok(Page {
            items: services
                .iter()
                .map(|s| s.service_code().unwrap_or_default().to_string())
                .collect(),
            next_token,
        })
    }
}

// Both resolvers are 1:1 passthroughs to a single already-tested
// `ServiceQuotasClient` method each (see `src/aws/service_quotas.rs`'s own
// test module for the pagination/limit/error-mapping behavior) — only light
// smoke tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::service_quotas::ServiceQuotasClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::ServiceQuotasQuery;

    const BASE: &str = "https://servicequotas.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn service_quotas_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"ServiceCode":"ec2","MaxResults":1}"#),
            json_response(
                200,
                r#"{"Quotas":[{"ServiceCode":"ec2","ServiceName":"Amazon EC2","QuotaCode":"L-1216C47A","QuotaName":"Running On-Demand Standard instances","Value":32.0,"Unit":"None","Adjustable":true,"GlobalQuota":false}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(ServiceQuotasQuery)
            .data(ServiceQuotasClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ serviceQuotas(serviceCode: "ec2", limit: 1) { items { serviceCode quotaCode quotaName value unit adjustable globalQuota } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["serviceQuotas"]["items"];
        assert_eq!(items[0]["serviceCode"], "ec2");
        assert_eq!(items[0]["quotaCode"], "L-1216C47A");
        assert_eq!(
            items[0]["quotaName"],
            "Running On-Demand Standard instances"
        );
        assert_eq!(items[0]["value"], 32.0);
        assert_eq!(items[0]["unit"], "None");
        assert_eq!(items[0]["adjustable"], true);
        assert_eq!(items[0]["globalQuota"], false);
        assert_eq!(json["serviceQuotas"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn service_quota_services_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Services":[{"ServiceCode":"ec2","ServiceName":"Amazon EC2"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(ServiceQuotasQuery)
            .data(ServiceQuotasClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ serviceQuotaServices(limit: 1) { items nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["serviceQuotaServices"]["items"][0], "ec2");
        assert_eq!(json["serviceQuotaServices"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
