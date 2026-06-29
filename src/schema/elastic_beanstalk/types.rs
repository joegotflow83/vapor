use async_graphql::SimpleObject;

use crate::aws::elastic_beanstalk::{
    ElasticBeanstalkApplication, ElasticBeanstalkApplicationVersion, ElasticBeanstalkEnvironment,
    ElasticBeanstalkS3Location,
};

#[derive(SimpleObject, Clone)]
pub struct BeanstalkApplication {
    pub application_name: String,
    pub description: Option<String>,
    pub date_created: Option<String>,
    pub date_updated: Option<String>,
    pub versions: Vec<String>,
    pub configuration_templates: Vec<String>,
}

impl From<ElasticBeanstalkApplication> for BeanstalkApplication {
    fn from(a: ElasticBeanstalkApplication) -> Self {
        Self {
            application_name: a.application_name,
            description: a.description,
            date_created: a.date_created,
            date_updated: a.date_updated,
            versions: a.versions,
            configuration_templates: a.configuration_templates,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BeanstalkEnvironment {
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

impl From<ElasticBeanstalkEnvironment> for BeanstalkEnvironment {
    fn from(e: ElasticBeanstalkEnvironment) -> Self {
        Self {
            environment_id: e.environment_id,
            environment_name: e.environment_name,
            application_name: e.application_name,
            solution_stack_name: e.solution_stack_name,
            platform_arn: e.platform_arn,
            status: e.status,
            health: e.health,
            health_status: e.health_status,
            cname: e.cname,
            endpoint_url: e.endpoint_url,
            date_created: e.date_created,
            date_updated: e.date_updated,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BeanstalkS3Location {
    pub s3_bucket: Option<String>,
    pub s3_key: Option<String>,
}

impl From<ElasticBeanstalkS3Location> for BeanstalkS3Location {
    fn from(s: ElasticBeanstalkS3Location) -> Self {
        Self {
            s3_bucket: s.s3_bucket,
            s3_key: s.s3_key,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BeanstalkApplicationVersion {
    pub application_name: Option<String>,
    pub version_label: Option<String>,
    pub description: Option<String>,
    pub source_bundle: Option<BeanstalkS3Location>,
    pub status: Option<String>,
    pub date_created: Option<String>,
    pub date_updated: Option<String>,
}

impl From<ElasticBeanstalkApplicationVersion> for BeanstalkApplicationVersion {
    fn from(v: ElasticBeanstalkApplicationVersion) -> Self {
        Self {
            application_name: v.application_name,
            version_label: v.version_label,
            description: v.description,
            source_bundle: v.source_bundle.map(BeanstalkS3Location::from),
            status: v.status,
            date_created: v.date_created,
            date_updated: v.date_updated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::elastic_beanstalk::{
        ElasticBeanstalkApplication, ElasticBeanstalkApplicationVersion,
        ElasticBeanstalkEnvironment, ElasticBeanstalkS3Location,
    };

    #[test]
    fn test_beanstalk_application_from_minimal() {
        let app = ElasticBeanstalkApplication {
            application_name: "my-app".to_string(),
            description: None,
            date_created: None,
            date_updated: None,
            versions: vec![],
            configuration_templates: vec![],
        };
        let result = BeanstalkApplication::from(app);
        assert_eq!(result.application_name, "my-app");
        assert!(result.description.is_none());
        assert!(result.versions.is_empty());
        assert!(result.configuration_templates.is_empty());
    }

    #[test]
    fn test_beanstalk_application_from_full() {
        let app = ElasticBeanstalkApplication {
            application_name: "web-app".to_string(),
            description: Some("My web application".to_string()),
            date_created: Some("2024-01-01T00:00:00Z".to_string()),
            date_updated: Some("2024-06-01T00:00:00Z".to_string()),
            versions: vec!["v1".to_string(), "v2".to_string()],
            configuration_templates: vec!["prod-template".to_string()],
        };
        let result = BeanstalkApplication::from(app);
        assert_eq!(result.application_name, "web-app");
        assert_eq!(result.description, Some("My web application".to_string()));
        assert_eq!(result.versions, vec!["v1", "v2"]);
        assert_eq!(result.configuration_templates, vec!["prod-template"]);
    }

    #[test]
    fn test_beanstalk_environment_from_minimal() {
        let env = ElasticBeanstalkEnvironment {
            environment_id: None,
            environment_name: None,
            application_name: None,
            solution_stack_name: None,
            platform_arn: None,
            status: None,
            health: None,
            health_status: None,
            cname: None,
            endpoint_url: None,
            date_created: None,
            date_updated: None,
        };
        let result = BeanstalkEnvironment::from(env);
        assert!(result.environment_id.is_none());
        assert!(result.status.is_none());
        assert!(result.health.is_none());
    }

    #[test]
    fn test_beanstalk_environment_from_full() {
        let env = ElasticBeanstalkEnvironment {
            environment_id: Some("e-abc123".to_string()),
            environment_name: Some("my-env".to_string()),
            application_name: Some("my-app".to_string()),
            solution_stack_name: Some("64bit Amazon Linux 2023".to_string()),
            platform_arn: Some("arn:aws:elasticbeanstalk:us-east-1::platform/...".to_string()),
            status: Some("Ready".to_string()),
            health: Some("Green".to_string()),
            health_status: Some("Ok".to_string()),
            cname: Some("my-env.elasticbeanstalk.com".to_string()),
            endpoint_url: Some("awseb-myelb-123.us-east-1.elb.amazonaws.com".to_string()),
            date_created: Some("2024-01-01T00:00:00Z".to_string()),
            date_updated: Some("2024-06-01T00:00:00Z".to_string()),
        };
        let result = BeanstalkEnvironment::from(env);
        assert_eq!(result.environment_id, Some("e-abc123".to_string()));
        assert_eq!(result.status, Some("Ready".to_string()));
        assert_eq!(result.health, Some("Green".to_string()));
        assert_eq!(result.cname, Some("my-env.elasticbeanstalk.com".to_string()));
    }

    #[test]
    fn test_beanstalk_s3_location_from() {
        let loc = ElasticBeanstalkS3Location {
            s3_bucket: Some("my-bucket".to_string()),
            s3_key: Some("my-app/v1.zip".to_string()),
        };
        let result = BeanstalkS3Location::from(loc);
        assert_eq!(result.s3_bucket, Some("my-bucket".to_string()));
        assert_eq!(result.s3_key, Some("my-app/v1.zip".to_string()));
    }

    #[test]
    fn test_beanstalk_application_version_from_minimal() {
        let ver = ElasticBeanstalkApplicationVersion {
            application_name: None,
            version_label: None,
            description: None,
            source_bundle: None,
            status: None,
            date_created: None,
            date_updated: None,
        };
        let result = BeanstalkApplicationVersion::from(ver);
        assert!(result.application_name.is_none());
        assert!(result.source_bundle.is_none());
    }

    #[test]
    fn test_beanstalk_application_version_from_full() {
        let ver = ElasticBeanstalkApplicationVersion {
            application_name: Some("my-app".to_string()),
            version_label: Some("v1.2.3".to_string()),
            description: Some("Release 1.2.3".to_string()),
            source_bundle: Some(ElasticBeanstalkS3Location {
                s3_bucket: Some("my-bucket".to_string()),
                s3_key: Some("my-app/v1.2.3.zip".to_string()),
            }),
            status: Some("Processed".to_string()),
            date_created: Some("2024-03-01T00:00:00Z".to_string()),
            date_updated: Some("2024-03-01T00:05:00Z".to_string()),
        };
        let result = BeanstalkApplicationVersion::from(ver);
        assert_eq!(result.application_name, Some("my-app".to_string()));
        assert_eq!(result.version_label, Some("v1.2.3".to_string()));
        assert_eq!(result.status, Some("Processed".to_string()));
        let bundle = result.source_bundle.unwrap();
        assert_eq!(bundle.s3_bucket, Some("my-bucket".to_string()));
        assert_eq!(bundle.s3_key, Some("my-app/v1.2.3.zip".to_string()));
    }
}
