use async_graphql::SimpleObject;
use aws_sdk_codebuild::types::{Build as SdkBuild, Project as SdkProject};
use chrono::{DateTime, Utc};

use crate::schema::time::to_utc;

#[derive(SimpleObject, Clone)]
#[graphql(name = "CodeBuildTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct BuildProject {
    pub name: String,
    pub arn: Option<String>,
    pub description: Option<String>,
    pub source_type: Option<String>,
    pub source_location: Option<String>,
    pub environment_type: Option<String>,
    pub compute_type: Option<String>,
    pub image: Option<String>,
    pub service_role: Option<String>,
    pub created: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub tags: Vec<Tag>,
}

impl From<SdkProject> for BuildProject {
    fn from(p: SdkProject) -> Self {
        let (source_type, source_location) = match p.source() {
            Some(src) => (
                Some(src.r#type().as_str().to_string()),
                src.location().map(|l| l.to_string()),
            ),
            None => (None, None),
        };
        let (environment_type, compute_type, image) = match p.environment() {
            Some(env) => (
                Some(env.r#type().as_str().to_string()),
                Some(env.compute_type().as_str().to_string()),
                Some(env.image().to_string()),
            ),
            None => (None, None, None),
        };
        Self {
            name: p.name().unwrap_or_default().to_string(),
            arn: p.arn().map(|v| v.to_string()),
            description: p.description().map(|v| v.to_string()),
            source_type,
            source_location,
            environment_type,
            compute_type,
            image,
            service_role: p.service_role().map(|v| v.to_string()),
            created: to_utc(p.created()),
            last_modified: to_utc(p.last_modified()),
            tags: p.tags().iter().map(|t| Tag {
                key: t.key().unwrap_or_default().to_string(),
                value: t.value().unwrap_or_default().to_string(),
            }).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Build {
    pub id: String,
    pub arn: Option<String>,
    pub project_name: Option<String>,
    pub build_status: Option<String>,
    pub initiator: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_in_seconds: Option<f64>,
    pub current_phase: Option<String>,
    pub source_version: Option<String>,
    pub logs_group: Option<String>,
    pub logs_stream: Option<String>,
}

impl From<SdkBuild> for Build {
    fn from(b: SdkBuild) -> Self {
        let duration_in_seconds = match (b.start_time(), b.end_time()) {
            (Some(start), Some(end)) => {
                let d = end.secs() - start.secs();
                Some(d as f64)
            }
            _ => None,
        };
        let (logs_group, logs_stream) = match b.logs() {
            Some(logs) => (
                logs.group_name().map(|v| v.to_string()),
                logs.stream_name().map(|v| v.to_string()),
            ),
            None => (None, None),
        };
        Self {
            id: b.id().unwrap_or_default().to_string(),
            arn: b.arn().map(|v| v.to_string()),
            project_name: b.project_name().map(|v| v.to_string()),
            build_status: b.build_status().map(|s| s.as_str().to_string()),
            initiator: b.initiator().map(|v| v.to_string()),
            start_time: to_utc(b.start_time()),
            end_time: to_utc(b.end_time()),
            duration_in_seconds,
            current_phase: b.current_phase().map(|v| v.to_string()),
            source_version: b.source_version().map(|v| v.to_string()),
            logs_group,
            logs_stream,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_project_from_sdk() {
        let project = SdkProject::builder()
            .name("my-project")
            .description("A test project")
            .created(aws_smithy_types::DateTime::from_secs(1_700_000_000))
            .last_modified(aws_smithy_types::DateTime::from_secs(1_710_000_000))
            .build();
        let bp = BuildProject::from(project);
        assert_eq!(bp.name, "my-project");
        assert_eq!(bp.description, Some("A test project".to_string()));
        assert!(bp.arn.is_none());
        assert!(bp.source_type.is_none());
        assert!(bp.tags.is_empty());
        assert_eq!(
            bp.created,
            Some(DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::seconds(1_700_000_000))
        );
        assert_eq!(
            bp.last_modified,
            Some(DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::seconds(1_710_000_000))
        );
    }

    #[test]
    fn test_build_project_minimal() {
        let project = SdkProject::builder().build();
        let bp = BuildProject::from(project);
        assert_eq!(bp.name, "");
        assert!(bp.description.is_none());
        assert!(bp.environment_type.is_none());
        assert!(bp.compute_type.is_none());
        assert!(bp.image.is_none());
    }

    #[test]
    fn test_build_from_sdk() {
        let build = SdkBuild::builder()
            .id("my-project:build-1")
            .project_name("my-project")
            .build_status(aws_sdk_codebuild::types::StatusType::Succeeded)
            .initiator("user/dev")
            .start_time(aws_smithy_types::DateTime::from_secs(1_700_000_000))
            .end_time(aws_smithy_types::DateTime::from_secs(1_700_000_600))
            .build();
        let b = Build::from(build);
        assert_eq!(b.id, "my-project:build-1");
        assert_eq!(b.project_name, Some("my-project".to_string()));
        assert_eq!(b.build_status, Some("SUCCEEDED".to_string()));
        assert_eq!(b.initiator, Some("user/dev".to_string()));
        assert_eq!(
            b.start_time,
            Some(DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::seconds(1_700_000_000))
        );
        assert_eq!(
            b.end_time,
            Some(DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::seconds(1_700_000_600))
        );
        assert_eq!(b.duration_in_seconds, Some(600.0));
    }

    #[test]
    fn test_build_minimal() {
        let build = SdkBuild::builder().build();
        let b = Build::from(build);
        assert_eq!(b.id, "");
        assert!(b.project_name.is_none());
        assert!(b.build_status.is_none());
        assert!(b.logs_group.is_none());
        assert!(b.logs_stream.is_none());
    }

    #[test]
    fn test_tag() {
        let tag = Tag { key: "env".to_string(), value: "prod".to_string() };
        assert_eq!(tag.key, "env");
        assert_eq!(tag.value, "prod");
    }
}
