use async_graphql::{Context, Object, Result};

use crate::aws::app_runner::AppRunnerClient;
use crate::schema::app_runner::types::{
    AppRunnerConnection, AppRunnerObservabilityConfiguration, AppRunnerService,
    AppRunnerVpcConnector,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AppRunnerQuery;

#[Object]
impl AppRunnerQuery {
    /// Lists App Runner services. `limit` caps the number of results
    /// returned (default unlimited); `next_token` resumes from a prior page.
    async fn app_runner_services(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppRunnerService>> {
        let client = ctx.data::<AppRunnerClient>()?;
        let (services, next_token) = client.list_services(limit, next_token).await?;
        Ok(Page {
            items: services.into_iter().map(AppRunnerService::from).collect(),
            next_token,
        })
    }

    async fn app_runner_service(
        &self,
        ctx: &Context<'_>,
        service_arn: String,
    ) -> Result<Option<AppRunnerService>> {
        let client = ctx.data::<AppRunnerClient>()?;
        let svc = client.describe_service(&service_arn).await?;
        Ok(svc.map(AppRunnerService::from))
    }

    /// Lists App Runner VPC connectors. `limit` caps the number of results
    /// returned (default unlimited); `next_token` resumes from a prior page.
    async fn app_runner_vpc_connectors(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppRunnerVpcConnector>> {
        let client = ctx.data::<AppRunnerClient>()?;
        let (connectors, next_token) = client.list_vpc_connectors(limit, next_token).await?;
        Ok(Page {
            items: connectors
                .into_iter()
                .map(AppRunnerVpcConnector::from)
                .collect(),
            next_token,
        })
    }

    /// Lists App Runner source repository connections. `limit` caps the
    /// number of results returned (default unlimited); `next_token` resumes
    /// from a prior page.
    async fn app_runner_connections(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppRunnerConnection>> {
        let client = ctx.data::<AppRunnerClient>()?;
        let (connections, next_token) = client.list_connections(limit, next_token).await?;
        Ok(Page {
            items: connections
                .into_iter()
                .map(AppRunnerConnection::from)
                .collect(),
            next_token,
        })
    }

    /// Lists App Runner observability configurations. `limit` caps the
    /// number of results returned (default unlimited); `next_token` resumes
    /// from a prior page.
    async fn app_runner_observability_configurations(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppRunnerObservabilityConfiguration>> {
        let client = ctx.data::<AppRunnerClient>()?;
        let (configs, next_token) = client
            .list_observability_configurations(limit, next_token)
            .await?;
        Ok(Page {
            items: configs
                .into_iter()
                .map(AppRunnerObservabilityConfiguration::from)
                .collect(),
            next_token,
        })
    }
}

// All 4 resolvers are 1:1 passthroughs to a single already-tested
// `AppRunnerClient` method (see `src/aws/app_runner.rs`'s own test module
// for the fan-out/pagination/error-mapping behavior) — only light smoke
// tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::app_runner::AppRunnerClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::AppRunnerQuery;

    const ENDPOINT: &str = "https://apprunner.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn app_runner_services_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"ServiceSummaryList":[{"ServiceArn":"arn-1"}],"NextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ServiceArn":"arn-1"}"#),
                json_response(
                    200,
                    r#"{"Service":{"ServiceName":"svc1","ServiceId":"id1","ServiceArn":"arn-1","Status":"RUNNING","CreatedAt":1700000000,"UpdatedAt":1700000000}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(AppRunnerQuery)
            .data(AppRunnerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ appRunnerServices(limit: 1) { items { serviceName status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appRunnerServices"]["items"];
        assert_eq!(items[0]["serviceName"], "svc1");
        assert_eq!(items[0]["status"], "RUNNING");
        assert_eq!(json["appRunnerServices"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn app_runner_service_returns_none_when_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ServiceArn":"arn-missing"}"#),
            json_error_response("ResourceNotFoundException", "no such service"),
        )]);
        let schema = build_query_schema(AppRunnerQuery)
            .data(AppRunnerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ appRunnerService(serviceArn: "arn-missing") { serviceName } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["appRunnerService"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn app_runner_service_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ServiceArn":"arn-1"}"#),
            json_error_response("InternalServiceErrorException", "internal failure"),
        )]);
        let schema = build_query_schema(AppRunnerQuery)
            .data(AppRunnerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ appRunnerService(serviceArn: "arn-1") { serviceName } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn app_runner_vpc_connectors_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"VpcConnectors":[{"VpcConnectorName":"conn1","VpcConnectorArn":"arn-1","VpcConnectorRevision":1,"Subnets":["subnet-1"],"SecurityGroups":["sg-1"],"Status":"ACTIVE"}]}"#,
            ),
        )]);
        let schema = build_query_schema(AppRunnerQuery)
            .data(AppRunnerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ appRunnerVpcConnectors { items { vpcConnectorName status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appRunnerVpcConnectors"]["items"];
        assert_eq!(items[0]["vpcConnectorName"], "conn1");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert!(json["appRunnerVpcConnectors"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn app_runner_connections_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ConnectionSummaryList":[{"ConnectionName":"conn1","ProviderType":"GITHUB","Status":"AVAILABLE"}]}"#,
            ),
        )]);
        let schema = build_query_schema(AppRunnerQuery)
            .data(AppRunnerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ appRunnerConnections { items { connectionName providerType status } } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appRunnerConnections"]["items"];
        assert_eq!(items[0]["connectionName"], "conn1");
        assert_eq!(items[0]["providerType"], "GITHUB");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn app_runner_observability_configurations_maps_items() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{}"#),
                json_response(
                    200,
                    r#"{"ObservabilityConfigurationSummaryList":[{"ObservabilityConfigurationArn":"arn-1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ObservabilityConfigurationArn":"arn-1"}"#),
                json_response(
                    200,
                    r#"{"ObservabilityConfiguration":{"ObservabilityConfigurationArn":"arn-1","ObservabilityConfigurationName":"cfg1","ObservabilityConfigurationRevision":1,"TraceConfiguration":{"Vendor":"AWSXRAY"},"Latest":true,"Status":"ACTIVE"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(AppRunnerQuery)
            .data(AppRunnerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ appRunnerObservabilityConfigurations { items { observabilityConfigurationName tracingVendor latest status } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appRunnerObservabilityConfigurations"]["items"];
        assert_eq!(items[0]["observabilityConfigurationName"], "cfg1");
        assert_eq!(items[0]["tracingVendor"], "AWSXRAY");
        assert_eq!(items[0]["latest"], true);
        http_client.relaxed_requests_match();
    }
}
