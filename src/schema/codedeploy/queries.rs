use async_graphql::{Context, Object, Result};

use crate::aws::codedeploy::CodeDeployClient;
use crate::schema::codedeploy::types::{DeployApplication, Deployment, DeploymentGroup};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CodeDeployQuery;

#[Object]
impl CodeDeployQuery {
    /// Lists CodeDeploy applications, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn deploy_applications(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DeployApplication>> {
        let client = ctx.data::<CodeDeployClient>()?;
        let (names, next_token) = client.list_applications(limit, next_token).await?;
        if names.is_empty() {
            return Ok(Page {
                items: Vec::new(),
                next_token,
            });
        }
        let apps = client.batch_get_applications(names).await?;
        Ok(Page {
            items: apps.into_iter().map(DeployApplication::from).collect(),
            next_token,
        })
    }

    /// Lists deployment groups for an application, optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    async fn deployment_groups(
        &self,
        ctx: &Context<'_>,
        application_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DeploymentGroup>> {
        let client = ctx.data::<CodeDeployClient>()?;
        let (names, next_token) = client
            .list_deployment_groups(&application_name, limit, next_token)
            .await?;
        if names.is_empty() {
            return Ok(Page {
                items: Vec::new(),
                next_token,
            });
        }
        let groups = client
            .batch_get_deployment_groups(&application_name, names)
            .await?;
        Ok(Page {
            items: groups.into_iter().map(DeploymentGroup::from).collect(),
            next_token,
        })
    }

    /// Lists deployments, optionally filtered by application/deployment group,
    /// capped at `limit` results (default unlimited), and resumed from
    /// `next_token`.
    async fn deployments(
        &self,
        ctx: &Context<'_>,
        application_name: Option<String>,
        deployment_group_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Deployment>> {
        let client = ctx.data::<CodeDeployClient>()?;
        let (ids, next_token) = client
            .list_deployments(
                application_name.as_deref(),
                deployment_group_name.as_deref(),
                limit,
                next_token,
            )
            .await?;
        if ids.is_empty() {
            return Ok(Page {
                items: Vec::new(),
                next_token,
            });
        }
        let deployments = client.batch_get_deployments(ids).await?;
        Ok(Page {
            items: deployments.into_iter().map(Deployment::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are list-then-batch_get fan-outs to already-tested
// `CodeDeployClient` methods (see `src/aws/codedeploy.rs`'s own test module
// for the pagination/chunking/error-mapping behavior of `list_applications`/
// `batch_get_applications` etc.) — only light smoke tests are needed here
// per the resolver-layer sweep's stated scope, covering the resolver-local
// logic: the empty-list short-circuit (skips `batch_get_*` entirely) and
// filter/argument forwarding.
#[cfg(test)]
mod tests {
    use crate::aws::codedeploy::CodeDeployClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CodeDeployQuery;

    const ENDPOINT: &str = "https://codedeploy.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn deploy_applications_maps_items_and_next_token() {
        // `ListApplicationsInput` has no `maxResults`-equivalent field, so
        // `limit: 1` is required to trip client-side `apply_limit` after one
        // page, otherwise `list_applications` pages again instead of moving
        // on to `batch_get_applications` and consumes this test's second
        // queued event out of order (gotcha class from codecommit/codebuild).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"applications":["app-1"],"nextToken":"cursor-b"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"applicationNames":["app-1"]}"#),
                json_response(
                    200,
                    r#"{"applicationsInfo":[{"applicationName":"app-1","applicationId":"id-1","computePlatform":"Server"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CodeDeployQuery)
            .data(CodeDeployClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ deployApplications(limit: 1) { items { id name computePlatform } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["deployApplications"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["id"], "id-1");
        assert_eq!(items[0]["name"], "app-1");
        assert_eq!(items[0]["computePlatform"], "Server");
        assert_eq!(json["deployApplications"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn deploy_applications_returns_empty_page_without_batch_get_when_no_applications() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"applications":[]}"#),
        )]);
        let schema = build_query_schema(CodeDeployQuery)
            .data(CodeDeployClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ deployApplications { items { name } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["deployApplications"]["items"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert!(json["deployApplications"]["nextToken"].is_null());
        // Only one queued event (list_applications) — a second call would
        // fail `relaxed_requests_match` below if batch_get_applications were
        // wrongly invoked with an empty name list.
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn deployment_groups_forwards_application_name_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"applicationName":"my-app"}"#),
                json_response(200, r#"{"deploymentGroups":["group-1"]}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"applicationName":"my-app","deploymentGroupNames":["group-1"]}"#,
                ),
                json_response(
                    200,
                    r#"{"deploymentGroupsInfo":[{"applicationName":"my-app","deploymentGroupName":"group-1","deploymentGroupId":"dg-1","serviceRoleArn":"arn:aws:iam::123456789012:role/deploy-role"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CodeDeployQuery)
            .data(CodeDeployClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ deploymentGroups(applicationName: "my-app") { items { id name applicationName serviceRoleArn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["deploymentGroups"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["id"], "dg-1");
        assert_eq!(items[0]["name"], "group-1");
        assert_eq!(items[0]["applicationName"], "my-app");
        assert!(json["deploymentGroups"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn deployment_groups_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"applicationName":"missing-app"}"#),
            json_error_response("ApplicationDoesNotExistException", "no such application"),
        )]);
        let schema = build_query_schema(CodeDeployQuery)
            .data(CodeDeployClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ deploymentGroups(applicationName: "missing-app") { items { name } } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("no such application"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn deployments_forwards_application_and_group_filters_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"applicationName":"my-app","deploymentGroupName":"my-group"}"#,
                ),
                json_response(200, r#"{"deployments":["d-1"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"deploymentIds":["d-1"]}"#),
                json_response(
                    200,
                    r#"{"deploymentsInfo":[{"deploymentId":"d-1","applicationName":"my-app","deploymentGroupName":"my-group","status":"Failed","errorInformation":{"code":"HEALTH_CONSTRAINTS","message":"deployment failed"}}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(CodeDeployQuery)
            .data(CodeDeployClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ deployments(applicationName: "my-app", deploymentGroupName: "my-group") { items { deploymentId applicationName deploymentGroupName status errorCode errorMessage } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["deployments"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["deploymentId"], "d-1");
        assert_eq!(items[0]["status"], "Failed");
        assert_eq!(items[0]["errorCode"], "HEALTH_CONSTRAINTS");
        assert_eq!(items[0]["errorMessage"], "deployment failed");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn deployments_propagates_batch_get_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"deployments":["d-1"]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"deploymentIds":["d-1"]}"#),
                json_error_response("BatchLimitExceededException", "too many ids"),
            ),
        ]);
        let schema = build_query_schema(CodeDeployQuery)
            .data(CodeDeployClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ deployments { items { deploymentId } } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("too many ids"));
        http_client.relaxed_requests_match();
    }
}
