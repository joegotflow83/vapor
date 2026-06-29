use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
#[graphql(name = "RamTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct RamResourceShare {
    pub resource_share_arn: Option<String>,
    pub name: Option<String>,
    pub owner_id: Option<String>,
    pub status: Option<String>,
    pub allow_external_principals: Option<bool>,
    pub creation_time: Option<String>,
    pub last_updated_time: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<aws_sdk_ram::types::ResourceShare> for RamResourceShare {
    fn from(s: aws_sdk_ram::types::ResourceShare) -> Self {
        Self {
            resource_share_arn: s.resource_share_arn().map(|v| v.to_string()),
            name: s.name().map(|v| v.to_string()),
            owner_id: s.owning_account_id().map(|v| v.to_string()),
            status: s.status().map(|v| v.as_str().to_string()),
            allow_external_principals: s.allow_external_principals(),
            creation_time: s.creation_time().map(|d| d.to_string()),
            last_updated_time: s.last_updated_time().map(|d| d.to_string()),
            tags: s
                .tags()
                .iter()
                .map(|t| Tag {
                    key: t.key().unwrap_or_default().to_string(),
                    value: t.value().unwrap_or_default().to_string(),
                })
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RamResource {
    pub arn: Option<String>,
    pub type_: Option<String>,
    pub resource_share_arn: Option<String>,
    pub status: Option<String>,
    pub creation_time: Option<String>,
    pub last_updated_time: Option<String>,
}

impl From<aws_sdk_ram::types::Resource> for RamResource {
    fn from(r: aws_sdk_ram::types::Resource) -> Self {
        Self {
            arn: r.arn().map(|v| v.to_string()),
            type_: r.r#type().map(|v| v.to_string()),
            resource_share_arn: r.resource_share_arn().map(|v| v.to_string()),
            status: r.status().map(|v| v.as_str().to_string()),
            creation_time: r.creation_time().map(|d| d.to_string()),
            last_updated_time: r.last_updated_time().map(|d| d.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RamPrincipal {
    pub id: Option<String>,
    pub resource_share_arn: Option<String>,
    pub creation_time: Option<String>,
    pub last_updated_time: Option<String>,
    pub external: Option<bool>,
}

impl From<aws_sdk_ram::types::Principal> for RamPrincipal {
    fn from(p: aws_sdk_ram::types::Principal) -> Self {
        Self {
            id: p.id().map(|v| v.to_string()),
            resource_share_arn: p.resource_share_arn().map(|v| v.to_string()),
            creation_time: p.creation_time().map(|d| d.to_string()),
            last_updated_time: p.last_updated_time().map(|d| d.to_string()),
            external: p.external(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_resource_share_from_minimal() {
        let share = aws_sdk_ram::types::ResourceShare::builder().build();
        let result = RamResourceShare::from(share);
        assert!(result.resource_share_arn.is_none());
        assert!(result.name.is_none());
        assert!(result.owner_id.is_none());
        assert!(result.status.is_none());
        assert!(result.allow_external_principals.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_ram_resource_share_from_full() {
        let share = aws_sdk_ram::types::ResourceShare::builder()
            .resource_share_arn("arn:aws:ram:us-east-1:123456789012:resource-share/abc123")
            .name("MyShare")
            .owning_account_id("123456789012")
            .status(aws_sdk_ram::types::ResourceShareStatus::Active)
            .allow_external_principals(true)
            .build();
        let result = RamResourceShare::from(share);
        assert_eq!(
            result.resource_share_arn,
            Some("arn:aws:ram:us-east-1:123456789012:resource-share/abc123".to_string())
        );
        assert_eq!(result.name, Some("MyShare".to_string()));
        assert_eq!(result.owner_id, Some("123456789012".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.allow_external_principals, Some(true));
    }

    #[test]
    fn test_ram_resource_share_tags() {
        let tag = aws_sdk_ram::types::Tag::builder()
            .key("Env")
            .value("prod")
            .build();
        let share = aws_sdk_ram::types::ResourceShare::builder()
            .tags(tag)
            .build();
        let result = RamResourceShare::from(share);
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_ram_resource_from_minimal() {
        let resource = aws_sdk_ram::types::Resource::builder().build();
        let result = RamResource::from(resource);
        assert!(result.arn.is_none());
        assert!(result.type_.is_none());
        assert!(result.resource_share_arn.is_none());
        assert!(result.status.is_none());
    }

    #[test]
    fn test_ram_resource_from_full() {
        let resource = aws_sdk_ram::types::Resource::builder()
            .arn("arn:aws:ec2:us-east-1:123456789012:subnet/subnet-abc123")
            .r#type("ec2:Subnet")
            .resource_share_arn("arn:aws:ram:us-east-1:123456789012:resource-share/abc123")
            .status(aws_sdk_ram::types::ResourceStatus::Available)
            .build();
        let result = RamResource::from(resource);
        assert_eq!(
            result.arn,
            Some("arn:aws:ec2:us-east-1:123456789012:subnet/subnet-abc123".to_string())
        );
        assert_eq!(result.type_, Some("ec2:Subnet".to_string()));
        assert_eq!(result.status, Some("AVAILABLE".to_string()));
    }

    #[test]
    fn test_ram_principal_from_minimal() {
        let principal = aws_sdk_ram::types::Principal::builder().build();
        let result = RamPrincipal::from(principal);
        assert!(result.id.is_none());
        assert!(result.resource_share_arn.is_none());
        assert!(result.external.is_none());
    }

    #[test]
    fn test_ram_principal_from_full() {
        let principal = aws_sdk_ram::types::Principal::builder()
            .id("123456789012")
            .resource_share_arn("arn:aws:ram:us-east-1:123456789012:resource-share/abc123")
            .external(false)
            .build();
        let result = RamPrincipal::from(principal);
        assert_eq!(result.id, Some("123456789012".to_string()));
        assert_eq!(
            result.resource_share_arn,
            Some("arn:aws:ram:us-east-1:123456789012:resource-share/abc123".to_string())
        );
        assert_eq!(result.external, Some(false));
    }
}
