use async_graphql::{Context, Object, Result};

use crate::aws::appconfig::AppConfigClient;
use crate::schema::appconfig::types::{
    AppConfigApplication, AppConfigEnvironment, AppConfigProfile,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AppConfigQuery;

#[Object]
impl AppConfigQuery {
    /// Lists AppConfig applications. `limit` caps the number of results (default
    /// unlimited); `next_token` resumes from a prior page.
    async fn appconfig_applications(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppConfigApplication>> {
        let client = ctx.data::<AppConfigClient>()?;
        let (apps, next_token) = client.list_applications(limit, next_token).await?;
        Ok(Page {
            items: apps.into_iter().map(AppConfigApplication::from).collect(),
            next_token,
        })
    }

    /// Lists AppConfig environments for an application. `limit` caps the number of
    /// results (default unlimited); `next_token` resumes from a prior page.
    async fn appconfig_environments(
        &self,
        ctx: &Context<'_>,
        application_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppConfigEnvironment>> {
        let client = ctx.data::<AppConfigClient>()?;
        let (envs, next_token) = client
            .list_environments(&application_id, limit, next_token)
            .await?;
        Ok(Page {
            items: envs.into_iter().map(AppConfigEnvironment::from).collect(),
            next_token,
        })
    }

    /// Lists AppConfig configuration profiles for an application. `limit` caps the
    /// number of results (default unlimited); `next_token` resumes from a prior page.
    async fn appconfig_profiles(
        &self,
        ctx: &Context<'_>,
        application_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AppConfigProfile>> {
        let client = ctx.data::<AppConfigClient>()?;
        let (profiles, next_token) = client
            .list_configuration_profiles(&application_id, limit, next_token)
            .await?;
        Ok(Page {
            items: profiles.into_iter().map(AppConfigProfile::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `AppConfigClient` method each (see `src/aws/appconfig.rs`'s own test
// module for the pagination/limit/error-mapping behavior) — only light
// smoke tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::appconfig::AppConfigClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::AppConfigQuery;

    const BASE: &str = "https://appconfig.us-east-1.amazonaws.com/applications";

    #[tokio::test]
    async fn appconfig_applications_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}?max_results=1"), ""),
            json_response(
                200,
                r#"{"Items":[{"Id":"app1","Name":"one","Description":"first"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(AppConfigQuery)
            .data(AppConfigClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ appconfigApplications(limit: 1) { items { id name description } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appconfigApplications"]["items"];
        assert_eq!(items[0]["id"], "app1");
        assert_eq!(items[0]["name"], "one");
        assert_eq!(items[0]["description"], "first");
        assert_eq!(json["appconfigApplications"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn appconfig_environments_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/app1/environments"), ""),
            json_response(
                200,
                r#"{"Items":[{"Id":"env1","ApplicationId":"app1","Name":"prod","State":"READY_FOR_DEPLOYMENT"}]}"#,
            ),
        )]);
        let schema = build_query_schema(AppConfigQuery)
            .data(AppConfigClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ appconfigEnvironments(applicationId: "app1") { items { id applicationId name state } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appconfigEnvironments"]["items"];
        assert_eq!(items[0]["id"], "env1");
        assert_eq!(items[0]["applicationId"], "app1");
        assert_eq!(items[0]["name"], "prod");
        assert_eq!(items[0]["state"], "READY_FOR_DEPLOYMENT");
        assert!(json["appconfigEnvironments"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn appconfig_profiles_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/app1/configurationprofiles"), ""),
            json_response(
                200,
                r#"{"Items":[{"Id":"cp1","ApplicationId":"app1","Name":"profile1","LocationUri":"hosted","Type":"AWS.AppConfig.FeatureFlags"}]}"#,
            ),
        )]);
        let schema = build_query_schema(AppConfigQuery)
            .data(AppConfigClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ appconfigProfiles(applicationId: "app1") { items { id applicationId name locationUri profileType } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["appconfigProfiles"]["items"];
        assert_eq!(items[0]["id"], "cp1");
        assert_eq!(items[0]["applicationId"], "app1");
        assert_eq!(items[0]["name"], "profile1");
        assert_eq!(items[0]["locationUri"], "hosted");
        assert_eq!(items[0]["profileType"], "AWS.AppConfig.FeatureFlags");
        assert!(json["appconfigProfiles"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
