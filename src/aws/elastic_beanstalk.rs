use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct ElasticBeanstalkApplication {
    pub application_name: String,
    pub description: Option<String>,
    pub date_created: Option<String>,
    pub date_updated: Option<String>,
    pub versions: Vec<String>,
    pub configuration_templates: Vec<String>,
}

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
    pub date_created: Option<String>,
    pub date_updated: Option<String>,
}

pub struct ElasticBeanstalkS3Location {
    pub s3_bucket: Option<String>,
    pub s3_key: Option<String>,
}

pub struct ElasticBeanstalkApplicationVersion {
    pub application_name: Option<String>,
    pub version_label: Option<String>,
    pub description: Option<String>,
    pub source_bundle: Option<ElasticBeanstalkS3Location>,
    pub status: Option<String>,
    pub date_created: Option<String>,
    pub date_updated: Option<String>,
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
        let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        let apps = output
            .applications()
            .iter()
            .map(|a| ElasticBeanstalkApplication {
                application_name: a.application_name().unwrap_or_default().to_string(),
                description: a.description().map(|s| s.to_string()),
                date_created: a.date_created().map(|dt| dt.to_string()),
                date_updated: a.date_updated().map(|dt| dt.to_string()),
                versions: a.versions().to_vec(),
                configuration_templates: a.configuration_templates().to_vec(),
            })
            .collect();
        Ok(apps)
    }

    pub async fn describe_environments(
        &self,
        application_name: Option<String>,
        environment_names: Option<Vec<String>>,
        included_deleted_back_to: Option<String>,
    ) -> Result<Vec<ElasticBeanstalkEnvironment>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;
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
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
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
                    date_created: env.date_created().map(|dt| dt.to_string()),
                    date_updated: env.date_updated().map(|dt| dt.to_string()),
                });
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_application_versions(
        &self,
        application_name: Option<String>,
        version_labels: Option<Vec<String>>,
    ) -> Result<Vec<ElasticBeanstalkApplicationVersion>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_application_versions();
            if let Some(ref name) = application_name {
                req = req.application_name(name);
            }
            if let Some(ref labels) = version_labels {
                req = req.set_version_labels(Some(labels.clone()));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
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
                    date_created: ver.date_created().map(|dt| dt.to_string()),
                    date_updated: ver.date_updated().map(|dt| dt.to_string()),
                });
            }
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}

fn parse_datetime(s: &str) -> Option<aws_sdk_elasticbeanstalk::primitives::DateTime> {
    let dt = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    Some(aws_sdk_elasticbeanstalk::primitives::DateTime::from_secs_and_nanos(
        dt.timestamp(),
        dt.timestamp_subsec_nanos(),
    ))
}
