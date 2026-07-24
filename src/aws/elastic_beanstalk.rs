use aws_config::SdkConfig;

use crate::error::VaporError;

#[derive(Debug)]
pub struct ElasticBeanstalkApplication {
    pub application_name: String,
    pub description: Option<String>,
    pub date_created: Option<aws_smithy_types::DateTime>,
    pub date_updated: Option<aws_smithy_types::DateTime>,
    pub versions: Vec<String>,
    pub configuration_templates: Vec<String>,
}

#[derive(Debug)]
pub struct ElasticBeanstalkEnvironment {
    pub environment_id: Option<String>,
    pub environment_name: Option<String>,
    pub application_name: Option<String>,
    pub solution_stack_name: Option<String>,
    pub platform_arn: Option<String>,
    pub status: Option<String>,
    pub health: Option<String>,
    pub health_status: Option<String>,
    pub cname: Option<String>,
    pub endpoint_url: Option<String>,
    pub date_created: Option<aws_smithy_types::DateTime>,
    pub date_updated: Option<aws_smithy_types::DateTime>,
}

#[derive(Debug)]
pub struct ElasticBeanstalkS3Location {
    pub s3_bucket: Option<String>,
    pub s3_key: Option<String>,
}

#[derive(Debug)]
pub struct ElasticBeanstalkApplicationVersion {
    pub application_name: Option<String>,
    pub version_label: Option<String>,
    pub description: Option<String>,
    pub source_bundle: Option<ElasticBeanstalkS3Location>,
    pub status: Option<String>,
    pub date_created: Option<aws_smithy_types::DateTime>,
    pub date_updated: Option<aws_smithy_types::DateTime>,
}

pub struct ElasticBeanstalkClient {
    inner: aws_sdk_elasticbeanstalk::Client,
}

impl ElasticBeanstalkClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_elasticbeanstalk::Client::new(config),
        }
    }

    pub async fn describe_applications(
        &self,
        application_names: Option<Vec<String>>,
    ) -> Result<Vec<ElasticBeanstalkApplication>, VaporError> {
        let mut req = self.inner.describe_applications();
        if let Some(names) = application_names {
            req = req.set_application_names(Some(names));
        }
        let output = req.send().await.map_err(crate::error::sdk_err)?;
        let apps = output
            .applications()
            .iter()
            .map(|a| ElasticBeanstalkApplication {
                application_name: a.application_name().unwrap_or_default().to_string(),
                description: a.description().map(|s| s.to_string()),
                date_created: a.date_created().cloned(),
                date_updated: a.date_updated().cloned(),
                versions: a.versions().to_vec(),
                configuration_templates: a.configuration_templates().to_vec(),
            })
            .collect();
        Ok(apps)
    }

    /// Lists environments, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeEnvironments` has
    /// both `max_records` and `next_token` (verified against pinned
    /// `aws-sdk-elasticbeanstalk` 1.104.0's `operation/describe_environments/
    /// _describe_environments_input.rs`), so `limit` is capped to the
    /// remaining budget on the request itself, matching `mq.rs`'s
    /// `list_configurations` pattern.
    pub async fn describe_environments(
        &self,
        application_name: Option<String>,
        environment_names: Option<Vec<String>>,
        included_deleted_back_to: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ElasticBeanstalkEnvironment>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;
        let included_deleted_dt = included_deleted_back_to.as_deref().and_then(parse_datetime);

        loop {
            let mut req = self.inner.describe_environments();
            if let Some(ref name) = application_name {
                req = req.application_name(name);
            }
            if let Some(ref names) = environment_names {
                req = req.set_environment_names(Some(names.clone()));
            }
            if let Some(dt) = included_deleted_dt {
                req = req.include_deleted(true).included_deleted_back_to(dt);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_records(l - items.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for env in output.environments() {
                items.push(ElasticBeanstalkEnvironment {
                    environment_id: env.environment_id().map(|s| s.to_string()),
                    environment_name: env.environment_name().map(|s| s.to_string()),
                    application_name: env.application_name().map(|s| s.to_string()),
                    solution_stack_name: env.solution_stack_name().map(|s| s.to_string()),
                    platform_arn: env.platform_arn().map(|s| s.to_string()),
                    status: env.status().map(|s| s.as_str().to_string()),
                    health: env.health().map(|h| h.as_str().to_string()),
                    health_status: env.health_status().map(|h| h.as_str().to_string()),
                    cname: env.cname().map(|s| s.to_string()),
                    endpoint_url: env.endpoint_url().map(|s| s.to_string()),
                    date_created: env.date_created().cloned(),
                    date_updated: env.date_updated().cloned(),
                });
            }
            token = output
                .next_token()
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists application versions, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `DescribeApplicationVersions` has both `max_records` and `next_token`
    /// (verified against pinned `aws-sdk-elasticbeanstalk` 1.104.0's
    /// `operation/describe_application_versions/
    /// _describe_application_versions_input.rs`), so `limit` is capped to the
    /// remaining budget on the request itself, matching `mq.rs`'s
    /// `list_configurations` pattern.
    pub async fn describe_application_versions(
        &self,
        application_name: Option<String>,
        version_labels: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ElasticBeanstalkApplicationVersion>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_application_versions();
            if let Some(ref name) = application_name {
                req = req.application_name(name);
            }
            if let Some(ref labels) = version_labels {
                req = req.set_version_labels(Some(labels.clone()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_records(l - items.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for ver in output.application_versions() {
                let source_bundle = ver.source_bundle().map(|s| ElasticBeanstalkS3Location {
                    s3_bucket: s.s3_bucket().map(|b| b.to_string()),
                    s3_key: s.s3_key().map(|k| k.to_string()),
                });
                items.push(ElasticBeanstalkApplicationVersion {
                    application_name: ver.application_name().map(|s| s.to_string()),
                    version_label: ver.version_label().map(|s| s.to_string()),
                    description: ver.description().map(|s| s.to_string()),
                    source_bundle,
                    status: ver.status().map(|s| s.as_str().to_string()),
                    date_created: ver.date_created().cloned(),
                    date_updated: ver.date_updated().cloned(),
                });
            }
            token = output
                .next_token()
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

fn parse_datetime(s: &str) -> Option<aws_sdk_elasticbeanstalk::primitives::DateTime> {
    let dt = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    Some(
        aws_sdk_elasticbeanstalk::primitives::DateTime::from_secs_and_nanos(
            dt.timestamp(),
            dt.timestamp_subsec_nanos(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::error::VaporError;

    const ENDPOINT: &str = "https://elasticbeanstalk.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_applications_happy_path() {
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
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let apps = client.describe_applications(None).await.unwrap();

        assert_eq!(apps.len(), 1);
        let app = &apps[0];
        assert_eq!(app.application_name, "my-app");
        assert_eq!(app.description, Some("desc".to_string()));
        assert_eq!(
            app.date_created.as_ref().map(|d| d.secs()),
            Some(1705312800)
        );
        assert_eq!(
            app.date_updated.as_ref().map(|d| d.secs()),
            Some(1705399200)
        );
        assert_eq!(app.versions, vec!["v1".to_string(), "v2".to_string()]);
        assert_eq!(app.configuration_templates, vec!["tmpl1".to_string()]);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_applications_filters_by_names() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeApplications&Version=2010-12-01&ApplicationNames.member.1=app-a&ApplicationNames.member.2=app-b",
            ),
            xml_response(
                200,
                "<DescribeApplicationsResponse><DescribeApplicationsResult><Applications>\
                 </Applications></DescribeApplicationsResult></DescribeApplicationsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let apps = client
            .describe_applications(Some(vec!["app-a".to_string(), "app-b".to_string()]))
            .await
            .unwrap();

        assert_eq!(apps.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_applications_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeApplications&Version=2010-12-01"),
            xml_error_response("InsufficientPrivilegesException", "not authorized"),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_applications(None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InsufficientPrivilegesException"));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_environments_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeEnvironments&Version=2010-12-01"),
            xml_response(
                200,
                "<DescribeEnvironmentsResponse><DescribeEnvironmentsResult><Environments>\
                 <member><EnvironmentId>e-123</EnvironmentId><EnvironmentName>my-env</EnvironmentName>\
                 <ApplicationName>my-app</ApplicationName><SolutionStackName>64bit Amazon Linux</SolutionStackName>\
                 <PlatformArn>arn:aws:elasticbeanstalk:us-east-1::platform/foo</PlatformArn>\
                 <Status>Ready</Status><Health>Green</Health><HealthStatus>Ok</HealthStatus>\
                 <CNAME>my-env.us-east-1.elasticbeanstalk.com</CNAME>\
                 <EndpointURL>10.0.0.1</EndpointURL></member>\
                 </Environments></DescribeEnvironmentsResult></DescribeEnvironmentsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client
            .describe_environments(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(envs.len(), 1);
        let env = &envs[0];
        assert_eq!(env.environment_id.as_deref(), Some("e-123"));
        assert_eq!(env.environment_name.as_deref(), Some("my-env"));
        assert_eq!(env.application_name.as_deref(), Some("my-app"));
        assert_eq!(
            env.solution_stack_name.as_deref(),
            Some("64bit Amazon Linux")
        );
        assert_eq!(env.status.as_deref(), Some("Ready"));
        assert_eq!(env.health.as_deref(), Some("Green"));
        assert_eq!(env.health_status.as_deref(), Some("Ok"));
        assert_eq!(
            env.cname.as_deref(),
            Some("my-env.us-east-1.elasticbeanstalk.com")
        );
        assert_eq!(env.endpoint_url.as_deref(), Some("10.0.0.1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_environments_resumes_from_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeEnvironments&Version=2010-12-01&NextToken=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeEnvironmentsResponse><DescribeEnvironmentsResult><Environments>\
                 </Environments></DescribeEnvironmentsResult></DescribeEnvironmentsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client
            .describe_environments(None, None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(envs.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_environments_stops_at_limit_with_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeEnvironments&Version=2010-12-01&MaxRecords=2",
            ),
            xml_response(
                200,
                "<DescribeEnvironmentsResponse><DescribeEnvironmentsResult><Environments>\
                 <member><EnvironmentId>e-1</EnvironmentId></member>\
                 <member><EnvironmentId>e-2</EnvironmentId></member>\
                 </Environments><NextToken>page2</NextToken></DescribeEnvironmentsResult></DescribeEnvironmentsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client
            .describe_environments(None, None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(envs.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_environments_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeEnvironments&Version=2010-12-01&ApplicationName=my-app",
                ),
                xml_response(
                    200,
                    "<DescribeEnvironmentsResponse><DescribeEnvironmentsResult><Environments>\
                     <member><EnvironmentId>e-1</EnvironmentId></member>\
                     </Environments><NextToken>page2</NextToken></DescribeEnvironmentsResult></DescribeEnvironmentsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeEnvironments&Version=2010-12-01&ApplicationName=my-app&NextToken=page2",
                ),
                xml_response(
                    200,
                    "<DescribeEnvironmentsResponse><DescribeEnvironmentsResult><Environments>\
                     <member><EnvironmentId>e-2</EnvironmentId></member>\
                     </Environments></DescribeEnvironmentsResult></DescribeEnvironmentsResponse>",
                ),
            ),
        ]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client
            .describe_environments(Some("my-app".to_string()), None, None, None, None)
            .await
            .unwrap();

        assert_eq!(envs.len(), 2);
        assert_eq!(envs[0].environment_id.as_deref(), Some("e-1"));
        assert_eq!(envs[1].environment_id.as_deref(), Some("e-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_environments_passes_included_deleted_back_to() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeEnvironments&Version=2010-12-01&IncludeDeleted=true&IncludedDeletedBackTo=2024-01-15T10%3A00%3A00Z",
            ),
            xml_response(
                200,
                "<DescribeEnvironmentsResponse><DescribeEnvironmentsResult><Environments>\
                 </Environments></DescribeEnvironmentsResult></DescribeEnvironmentsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client
            .describe_environments(
                None,
                None,
                Some("2024-01-15T10:00:00Z".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(envs.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_environments_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeEnvironments&Version=2010-12-01"),
            xml_error_response("InvalidParameterValueException", "bad param"),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_environments(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidParameterValueException"));
                assert_eq!(message, "bad param");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_application_versions_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeApplicationVersions&Version=2010-12-01"),
            xml_response(
                200,
                "<DescribeApplicationVersionsResponse><DescribeApplicationVersionsResult><ApplicationVersions>\
                 <member><ApplicationName>my-app</ApplicationName><VersionLabel>v1</VersionLabel>\
                 <Description>first release</Description><Status>PROCESSED</Status>\
                 <SourceBundle><S3Bucket>my-bucket</S3Bucket><S3Key>my-key.zip</S3Key></SourceBundle>\
                 </member></ApplicationVersions></DescribeApplicationVersionsResult></DescribeApplicationVersionsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (versions, token) = client
            .describe_application_versions(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(versions.len(), 1);
        let ver = &versions[0];
        assert_eq!(ver.application_name.as_deref(), Some("my-app"));
        assert_eq!(ver.version_label.as_deref(), Some("v1"));
        assert_eq!(ver.description.as_deref(), Some("first release"));
        assert_eq!(ver.status.as_deref(), Some("PROCESSED"));
        let bundle = ver.source_bundle.as_ref().unwrap();
        assert_eq!(bundle.s3_bucket.as_deref(), Some("my-bucket"));
        assert_eq!(bundle.s3_key.as_deref(), Some("my-key.zip"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_application_versions_filters_by_version_labels() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeApplicationVersions&Version=2010-12-01&VersionLabels.member.1=v1&VersionLabels.member.2=v2",
            ),
            xml_response(
                200,
                "<DescribeApplicationVersionsResponse><DescribeApplicationVersionsResult><ApplicationVersions>\
                 </ApplicationVersions></DescribeApplicationVersionsResult></DescribeApplicationVersionsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (versions, _token) = client
            .describe_application_versions(
                None,
                Some(vec!["v1".to_string(), "v2".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(versions.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_application_versions_resumes_from_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeApplicationVersions&Version=2010-12-01&NextToken=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeApplicationVersionsResponse><DescribeApplicationVersionsResult><ApplicationVersions>\
                 </ApplicationVersions></DescribeApplicationVersionsResult></DescribeApplicationVersionsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (versions, token) = client
            .describe_application_versions(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(versions.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_application_versions_stops_at_limit_with_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeApplicationVersions&Version=2010-12-01&MaxRecords=1",
            ),
            xml_response(
                200,
                "<DescribeApplicationVersionsResponse><DescribeApplicationVersionsResult><ApplicationVersions>\
                 <member><VersionLabel>v1</VersionLabel></member>\
                 </ApplicationVersions><NextToken>page2</NextToken></DescribeApplicationVersionsResult></DescribeApplicationVersionsResponse>",
            ),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (versions, token) = client
            .describe_application_versions(None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(versions.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_application_versions_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeApplicationVersions&Version=2010-12-01"),
                xml_response(
                    200,
                    "<DescribeApplicationVersionsResponse><DescribeApplicationVersionsResult><ApplicationVersions>\
                     <member><VersionLabel>v1</VersionLabel></member>\
                     </ApplicationVersions><NextToken>page2</NextToken></DescribeApplicationVersionsResult></DescribeApplicationVersionsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeApplicationVersions&Version=2010-12-01&NextToken=page2",
                ),
                xml_response(
                    200,
                    "<DescribeApplicationVersionsResponse><DescribeApplicationVersionsResult><ApplicationVersions>\
                     <member><VersionLabel>v2</VersionLabel></member>\
                     </ApplicationVersions></DescribeApplicationVersionsResult></DescribeApplicationVersionsResponse>",
                ),
            ),
        ]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let (versions, token) = client
            .describe_application_versions(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version_label.as_deref(), Some("v1"));
        assert_eq!(versions[1].version_label.as_deref(), Some("v2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_application_versions_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeApplicationVersions&Version=2010-12-01",
            ),
            xml_error_response("TooManyApplicationVersionsException", "limit exceeded"),
        )]);
        let client = ElasticBeanstalkClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_application_versions(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("TooManyApplicationVersionsException"));
                assert_eq!(message, "limit exceeded");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
