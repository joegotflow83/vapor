use async_graphql::{Context, Object, Result};

use crate::aws::elastic_beanstalk::ElasticBeanstalkClient;
use crate::schema::elastic_beanstalk::types::{
    BeanstalkApplication, BeanstalkApplicationVersion, BeanstalkEnvironment,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct ElasticBeanstalkQuery;

#[Object]
impl ElasticBeanstalkQuery {
    async fn beanstalk_applications(
        &self,
        ctx: &Context<'_>,
        application_names: Option<Vec<String>>,
    ) -> Result<Vec<BeanstalkApplication>> {
        let client = ctx.data::<ElasticBeanstalkClient>()?;
        let apps = client.describe_applications(application_names).await?;
        Ok(apps.into_iter().map(BeanstalkApplication::from).collect())
    }

    /// `limit` caps the number of environments returned in this page (default
    /// unlimited); pass the returned `nextToken` back in to resume.
    async fn beanstalk_environments(
        &self,
        ctx: &Context<'_>,
        application_name: Option<String>,
        environment_names: Option<Vec<String>>,
        included_deleted_back_to: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BeanstalkEnvironment>> {
        let client = ctx.data::<ElasticBeanstalkClient>()?;
        let (envs, next_token) = client
            .describe_environments(
                application_name,
                environment_names,
                included_deleted_back_to,
                limit,
                next_token,
            )
            .await?;
        Ok(Page {
            items: envs.into_iter().map(BeanstalkEnvironment::from).collect(),
            next_token,
        })
    }

    /// `limit` caps the number of application versions returned in this page
    /// (default unlimited); pass the returned `nextToken` back in to resume.
    async fn beanstalk_application_versions(
        &self,
        ctx: &Context<'_>,
        application_name: Option<String>,
        version_labels: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BeanstalkApplicationVersion>> {
        let client = ctx.data::<ElasticBeanstalkClient>()?;
        let (versions, next_token) = client
            .describe_application_versions(application_name, version_labels, limit, next_token)
            .await?;
        Ok(Page {
            items: versions
                .into_iter()
                .map(BeanstalkApplicationVersion::from)
                .collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::elastic_beanstalk::ElasticBeanstalkClient;
    use crate::aws::test_util::{
        request, sdk_config, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::ElasticBeanstalkQuery;

    const ENDPOINT: &str = "https://elasticbeanstalk.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn beanstalk_applications_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeApplications&Version=2010-12-01"),
            xml_response(
                200,
                "<DescribeApplicationsResponse><DescribeApplicationsResult><Applications>\
                 <member><ApplicationName>my-app</ApplicationName><Description>desc</Description>\
                 <DateCreated>2024-01-15T10:00:00Z</DateCreated><DateUpdated>2024-01-16T10:00:00Z</DateUpdated>\
                 <Versions><member>v1</member><member>v2</member></Versions>\
                 <ConfigurationTemplates><member>tmpl1</member></ConfigurationTemplates></member>\
                 </Applications></DescribeApplicationsResult></DescribeApplicationsResponse>",
            ),
        )]);
        let schema = build_query_schema(ElasticBeanstalkQuery)
            .data(ElasticBeanstalkClient::new(&sdk_config(
                http_client.clone(),
            )))
            .finish();

        let res = schema
            .execute(
                r#"{ beanstalkApplications { applicationName description versions configurationTemplates } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["beanstalkApplications"];
        assert_eq!(items[0]["applicationName"], "my-app");
        assert_eq!(items[0]["description"], "desc");
        assert_eq!(items[0]["versions"], serde_json::json!(["v1", "v2"]));
        assert_eq!(
            items[0]["configurationTemplates"],
            serde_json::json!(["tmpl1"])
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn beanstalk_environments_forwards_limit_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeEnvironments&Version=2010-12-01&MaxRecords=1",
            ),
            xml_response(
                200,
                "<DescribeEnvironmentsResponse><DescribeEnvironmentsResult><Environments>\
                 <member><EnvironmentId>e-123</EnvironmentId><EnvironmentName>my-env</EnvironmentName>\
                 <ApplicationName>my-app</ApplicationName><Status>Ready</Status><Health>Green</Health>\
                 </member></Environments><NextToken>page2</NextToken></DescribeEnvironmentsResult></DescribeEnvironmentsResponse>",
            ),
        )]);
        let schema = build_query_schema(ElasticBeanstalkQuery)
            .data(ElasticBeanstalkClient::new(&sdk_config(
                http_client.clone(),
            )))
            .finish();

        let res = schema
            .execute(
                r#"{ beanstalkEnvironments(limit: 1) { items { environmentId environmentName applicationName status health } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["beanstalkEnvironments"]["items"];
        assert_eq!(items[0]["environmentId"], "e-123");
        assert_eq!(items[0]["environmentName"], "my-env");
        assert_eq!(items[0]["applicationName"], "my-app");
        assert_eq!(items[0]["status"], "Ready");
        assert_eq!(items[0]["health"], "Green");
        assert_eq!(json["beanstalkEnvironments"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn beanstalk_application_versions_forwards_limit_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeApplicationVersions&Version=2010-12-01&MaxRecords=1",
            ),
            xml_response(
                200,
                "<DescribeApplicationVersionsResponse><DescribeApplicationVersionsResult><ApplicationVersions>\
                 <member><ApplicationName>my-app</ApplicationName><VersionLabel>v1</VersionLabel>\
                 <Description>first release</Description><Status>PROCESSED</Status>\
                 <SourceBundle><S3Bucket>my-bucket</S3Bucket><S3Key>my-key.zip</S3Key></SourceBundle>\
                 </member></ApplicationVersions><NextToken>page2</NextToken></DescribeApplicationVersionsResult></DescribeApplicationVersionsResponse>",
            ),
        )]);
        let schema = build_query_schema(ElasticBeanstalkQuery)
            .data(ElasticBeanstalkClient::new(&sdk_config(
                http_client.clone(),
            )))
            .finish();

        let res = schema
            .execute(
                r#"{ beanstalkApplicationVersions(limit: 1) { items { applicationName versionLabel description status sourceBundle { s3Bucket s3Key } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["beanstalkApplicationVersions"]["items"];
        assert_eq!(items[0]["applicationName"], "my-app");
        assert_eq!(items[0]["versionLabel"], "v1");
        assert_eq!(items[0]["description"], "first release");
        assert_eq!(items[0]["status"], "PROCESSED");
        assert_eq!(items[0]["sourceBundle"]["s3Bucket"], "my-bucket");
        assert_eq!(items[0]["sourceBundle"]["s3Key"], "my-key.zip");
        assert_eq!(json["beanstalkApplicationVersions"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
