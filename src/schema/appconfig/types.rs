use async_graphql::SimpleObject;
use aws_sdk_appconfig::types::{Application, ConfigurationProfileSummary, Environment};

#[derive(SimpleObject, Clone)]
pub struct AppConfigApplication {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl From<Application> for AppConfigApplication {
    fn from(a: Application) -> Self {
        Self {
            id: a.id().unwrap_or_default().to_string(),
            name: a.name().map(|n| n.to_string()),
            description: a.description().map(|d| d.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AppConfigEnvironment {
    pub id: String,
    pub application_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub state: Option<String>,
}

impl From<Environment> for AppConfigEnvironment {
    fn from(e: Environment) -> Self {
        Self {
            id: e.id().unwrap_or_default().to_string(),
            application_id: e.application_id().unwrap_or_default().to_string(),
            name: e.name().map(|n| n.to_string()),
            description: e.description().map(|d| d.to_string()),
            state: e.state().map(|s| s.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AppConfigProfile {
    pub id: String,
    pub application_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub location_uri: Option<String>,
    pub profile_type: Option<String>,
}

impl From<ConfigurationProfileSummary> for AppConfigProfile {
    fn from(p: ConfigurationProfileSummary) -> Self {
        Self {
            id: p.id().unwrap_or_default().to_string(),
            application_id: p.application_id().unwrap_or_default().to_string(),
            name: p.name().map(|n| n.to_string()),
            // `ConfigurationProfileSummary` carries no description; left unset.
            description: None,
            location_uri: p.location_uri().map(|l| l.to_string()),
            profile_type: p.r#type().map(|t| t.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appconfig_application_full() {
        let a = AppConfigApplication {
            id: "abc123".to_string(),
            name: Some("MyApp".to_string()),
            description: Some("My application".to_string()),
        };

        assert_eq!(a.id, "abc123");
        assert_eq!(a.name, Some("MyApp".to_string()));
        assert_eq!(a.description, Some("My application".to_string()));
    }

    #[test]
    fn test_appconfig_application_minimal() {
        let a = AppConfigApplication {
            id: "xyz".to_string(),
            name: None,
            description: None,
        };

        assert_eq!(a.id, "xyz");
        assert!(a.name.is_none());
        assert!(a.description.is_none());
    }

    #[test]
    fn test_appconfig_environment_full() {
        let e = AppConfigEnvironment {
            id: "env1".to_string(),
            application_id: "app1".to_string(),
            name: Some("Production".to_string()),
            description: Some("Prod env".to_string()),
            state: Some("READY_FOR_DEPLOYMENT".to_string()),
        };

        assert_eq!(e.id, "env1");
        assert_eq!(e.application_id, "app1");
        assert_eq!(e.name, Some("Production".to_string()));
        assert_eq!(e.description, Some("Prod env".to_string()));
        assert_eq!(e.state, Some("READY_FOR_DEPLOYMENT".to_string()));
    }

    #[test]
    fn test_appconfig_environment_minimal() {
        let e = AppConfigEnvironment {
            id: "env2".to_string(),
            application_id: "app2".to_string(),
            name: None,
            description: None,
            state: None,
        };

        assert_eq!(e.id, "env2");
        assert_eq!(e.application_id, "app2");
        assert!(e.name.is_none());
        assert!(e.description.is_none());
        assert!(e.state.is_none());
    }

    #[test]
    fn test_appconfig_profile_full() {
        let p = AppConfigProfile {
            id: "prof1".to_string(),
            application_id: "app1".to_string(),
            name: Some("FeatureFlags".to_string()),
            description: Some("Feature flag config".to_string()),
            location_uri: Some("hosted".to_string()),
            profile_type: Some("AWS.AppConfig.FeatureFlags".to_string()),
        };

        assert_eq!(p.id, "prof1");
        assert_eq!(p.application_id, "app1");
        assert_eq!(p.name, Some("FeatureFlags".to_string()));
        assert_eq!(p.description, Some("Feature flag config".to_string()));
        assert_eq!(p.location_uri, Some("hosted".to_string()));
        assert_eq!(p.profile_type, Some("AWS.AppConfig.FeatureFlags".to_string()));
    }

    #[test]
    fn test_appconfig_profile_minimal() {
        let p = AppConfigProfile {
            id: "prof2".to_string(),
            application_id: "app2".to_string(),
            name: None,
            description: None,
            location_uri: None,
            profile_type: None,
        };

        assert_eq!(p.id, "prof2");
        assert_eq!(p.application_id, "app2");
        assert!(p.name.is_none());
        assert!(p.description.is_none());
        assert!(p.location_uri.is_none());
        assert!(p.profile_type.is_none());
    }
}
