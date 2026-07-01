use async_graphql::SimpleObject;

use crate::schema::common::types::Tag;

/// A CloudFront distribution.
#[derive(SimpleObject, Clone)]
pub struct CfDistribution {
    pub id: Option<String>,
    pub arn: Option<String>,
    pub domain_name: Option<String>,
    pub status: Option<String>,
    pub enabled: Option<bool>,
    pub http_version: Option<String>,
    pub is_ipv6_enabled: Option<bool>,
    pub price_class: Option<String>,
    pub aliases: Vec<String>,
    pub origins: Vec<CfOrigin>,
    pub default_cache_behavior: Option<CfCacheBehavior>,
    pub viewer_certificate: Option<CfViewerCertificate>,
    pub web_acl_id: Option<String>,
    pub comment: Option<String>,
    pub last_modified_time: Option<String>,
    pub tags: Vec<Tag>,
}

/// A CloudFront origin.
#[derive(SimpleObject, Clone)]
pub struct CfOrigin {
    pub id: Option<String>,
    pub domain_name: Option<String>,
    pub origin_path: Option<String>,
}

/// Cache behavior settings for a CloudFront distribution.
#[derive(SimpleObject, Clone)]
pub struct CfCacheBehavior {
    pub target_origin_id: Option<String>,
    pub viewer_protocol_policy: Option<String>,
    pub compress: Option<bool>,
    pub allowed_methods: Vec<String>,
    pub cached_methods: Vec<String>,
}

/// TLS/SSL certificate configuration for a CloudFront distribution.
#[derive(SimpleObject, Clone)]
pub struct CfViewerCertificate {
    pub acm_certificate_arn: Option<String>,
    pub cloudfront_default_certificate: Option<bool>,
    pub minimum_protocol_version: Option<String>,
    pub ssl_support_method: Option<String>,
}

pub fn map_origins(origins: &aws_sdk_cloudfront::types::Origins) -> Vec<CfOrigin> {
    origins
        .items()
        .iter()
        .map(|o| CfOrigin {
            id: Some(o.id().to_string()),
            domain_name: Some(o.domain_name().to_string()),
            origin_path: o.origin_path().map(|s| s.to_string()),
        })
        .collect()
}

pub fn map_default_cache_behavior(
    dcb: &aws_sdk_cloudfront::types::DefaultCacheBehavior,
) -> CfCacheBehavior {
    let allowed_methods = dcb
        .allowed_methods()
        .map(|am| am.items().iter().map(|m| m.as_str().to_string()).collect())
        .unwrap_or_default();

    let cached_methods = dcb
        .allowed_methods()
        .and_then(|am| am.cached_methods())
        .map(|cm| cm.items().iter().map(|m| m.as_str().to_string()).collect())
        .unwrap_or_default();

    CfCacheBehavior {
        target_origin_id: Some(dcb.target_origin_id().to_string()),
        viewer_protocol_policy: Some(dcb.viewer_protocol_policy().as_str().to_string()),
        compress: dcb.compress(),
        allowed_methods,
        cached_methods,
    }
}

pub fn map_viewer_certificate(
    vc: &aws_sdk_cloudfront::types::ViewerCertificate,
) -> CfViewerCertificate {
    CfViewerCertificate {
        acm_certificate_arn: vc.acm_certificate_arn().map(|s| s.to_string()),
        cloudfront_default_certificate: vc.cloud_front_default_certificate(),
        minimum_protocol_version: vc
            .minimum_protocol_version()
            .map(|v| v.as_str().to_string()),
        ssl_support_method: vc.ssl_support_method().map(|m| m.as_str().to_string()),
    }
}

pub fn map_tags(sdk_tags: Vec<aws_sdk_cloudfront::types::Tag>) -> Vec<Tag> {
    sdk_tags
        .into_iter()
        .map(|t| Tag {
            key: t.key().to_string(),
            value: t.value().unwrap_or("").to_string(),
        })
        .collect()
}

/// Build a CfDistribution from a DistributionSummary and pre-fetched tags.
pub fn distribution_from_summary(
    d: &aws_sdk_cloudfront::types::DistributionSummary,
    tags: Vec<Tag>,
) -> CfDistribution {
    let origins = d.origins().map(map_origins).unwrap_or_default();
    let default_cache_behavior = d.default_cache_behavior().map(map_default_cache_behavior);
    let viewer_certificate = d.viewer_certificate().map(map_viewer_certificate);
    let aliases = d
        .aliases()
        .map(|a| a.items().iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    CfDistribution {
        id: Some(d.id().to_string()),
        arn: Some(d.arn().to_string()),
        domain_name: Some(d.domain_name().to_string()),
        status: Some(d.status().to_string()),
        enabled: Some(d.enabled()),
        http_version: Some(d.http_version().as_str().to_string()),
        is_ipv6_enabled: Some(d.is_ipv6_enabled()),
        price_class: Some(d.price_class().as_str().to_string()),
        aliases,
        origins,
        default_cache_behavior,
        viewer_certificate,
        web_acl_id: {
            let w = d.web_acl_id();
            if w.is_empty() { None } else { Some(w.to_string()) }
        },
        comment: {
            let c = d.comment();
            if c.is_empty() { None } else { Some(c.to_string()) }
        },
        last_modified_time: Some(d.last_modified_time().to_string()),
        tags,
    }
}

/// Build a CfDistribution from a Distribution (returned by get_distribution) and pre-fetched tags.
pub fn distribution_from_get(
    d: &aws_sdk_cloudfront::types::Distribution,
    tags: Vec<Tag>,
) -> CfDistribution {
    let cfg = d.distribution_config();
    let origins = cfg
        .and_then(|c| c.origins())
        .map(map_origins)
        .unwrap_or_default();
    let default_cache_behavior = cfg
        .and_then(|c| c.default_cache_behavior())
        .map(map_default_cache_behavior);
    let viewer_certificate = cfg
        .and_then(|c| c.viewer_certificate())
        .map(map_viewer_certificate);
    let aliases = cfg
        .and_then(|c| c.aliases())
        .map(|a| a.items().iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    CfDistribution {
        id: Some(d.id().to_string()),
        arn: Some(d.arn().to_string()),
        domain_name: Some(d.domain_name().to_string()),
        status: Some(d.status().to_string()),
        enabled: cfg.map(|c| c.enabled()),
        http_version: cfg
            .and_then(|c| c.http_version())
            .map(|v| v.as_str().to_string()),
        is_ipv6_enabled: cfg.and_then(|c| c.is_ipv6_enabled()),
        price_class: cfg
            .and_then(|c| c.price_class())
            .map(|p| p.as_str().to_string()),
        aliases,
        origins,
        default_cache_behavior,
        viewer_certificate,
        web_acl_id: cfg.and_then(|c| c.web_acl_id()).and_then(|w| {
            if w.is_empty() { None } else { Some(w.to_string()) }
        }),
        comment: cfg.and_then(|c| {
            let cc = c.comment();
            if cc.is_empty() { None } else { Some(cc.to_string()) }
        }),
        last_modified_time: Some(d.last_modified_time().to_string()),
        tags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_cloudfront::types::{
        AllowedMethods, CachedMethods, DefaultCacheBehavior, Method, MinimumProtocolVersion,
        Origin, Origins, SslSupportMethod, ViewerCertificate, ViewerProtocolPolicy,
    };

    #[test]
    fn test_map_tags_with_values() {
        let sdk_tags = vec![
            aws_sdk_cloudfront::types::Tag::builder()
                .key("Env")
                .value("production")
                .build()
                .unwrap(),
            aws_sdk_cloudfront::types::Tag::builder()
                .key("App")
                .value("my-app")
                .build()
                .unwrap(),
        ];

        let tags = map_tags(sdk_tags);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].key, "Env");
        assert_eq!(tags[0].value, "production");
        assert_eq!(tags[1].key, "App");
        assert_eq!(tags[1].value, "my-app");
    }

    #[test]
    fn test_map_tags_missing_value_becomes_empty_string() {
        // A tag with no value set maps to empty string (not None) in CfDistribution
        let sdk_tags = vec![
            aws_sdk_cloudfront::types::Tag::builder()
                .key("NoValue")
                .build()
                .unwrap(),
        ];

        let tags = map_tags(sdk_tags);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key, "NoValue");
        assert_eq!(tags[0].value, "");
    }

    #[test]
    fn test_map_tags_empty_list() {
        let tags = map_tags(vec![]);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_map_viewer_certificate_acm() {
        let sdk = ViewerCertificate::builder()
            .acm_certificate_arn("arn:aws:acm:us-east-1:123456789012:certificate/abc-123")
            .cloud_front_default_certificate(false)
            .minimum_protocol_version(MinimumProtocolVersion::TlSv122021)
            .ssl_support_method(SslSupportMethod::SniOnly)
            .build();

        let cert = map_viewer_certificate(&sdk);
        assert_eq!(
            cert.acm_certificate_arn,
            Some("arn:aws:acm:us-east-1:123456789012:certificate/abc-123".to_string())
        );
        assert_eq!(cert.cloudfront_default_certificate, Some(false));
        assert!(cert.minimum_protocol_version.is_some());
        assert!(cert.ssl_support_method.is_some());
    }

    #[test]
    fn test_map_viewer_certificate_default() {
        let sdk = ViewerCertificate::builder()
            .cloud_front_default_certificate(true)
            .build();

        let cert = map_viewer_certificate(&sdk);
        assert!(cert.acm_certificate_arn.is_none());
        assert_eq!(cert.cloudfront_default_certificate, Some(true));
        assert!(cert.minimum_protocol_version.is_none());
        assert!(cert.ssl_support_method.is_none());
    }

    #[test]
    fn test_map_viewer_certificate_empty() {
        let sdk = ViewerCertificate::builder().build();
        let cert = map_viewer_certificate(&sdk);
        assert!(cert.acm_certificate_arn.is_none());
        assert!(cert.cloudfront_default_certificate.is_none());
    }

    #[test]
    fn test_map_origins_single_origin() {
        let origin = Origin::builder()
            .id("my-origin")
            .domain_name("example.com")
            .origin_path("/api")
            .build()
            .unwrap();

        let sdk_origins = Origins::builder()
            .quantity(1)
            .items(origin)
            .build()
            .unwrap();

        let origins = map_origins(&sdk_origins);
        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0].id, Some("my-origin".to_string()));
        assert_eq!(origins[0].domain_name, Some("example.com".to_string()));
        assert_eq!(origins[0].origin_path, Some("/api".to_string()));
    }

    #[test]
    fn test_map_origins_no_path() {
        let origin = Origin::builder()
            .id("s3-origin")
            .domain_name("mybucket.s3.amazonaws.com")
            .build()
            .unwrap();

        let sdk_origins = Origins::builder()
            .quantity(1)
            .items(origin)
            .build()
            .unwrap();

        let origins = map_origins(&sdk_origins);
        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0].id, Some("s3-origin".to_string()));
        assert!(origins[0].origin_path.is_none());
    }

    #[test]
    fn test_map_default_cache_behavior_basic() {
        let dcb = DefaultCacheBehavior::builder()
            .target_origin_id("my-origin")
            .viewer_protocol_policy(ViewerProtocolPolicy::RedirectToHttps)
            .compress(true)
            .build()
            .unwrap();

        let behavior = map_default_cache_behavior(&dcb);
        assert_eq!(behavior.target_origin_id, Some("my-origin".to_string()));
        assert_eq!(behavior.viewer_protocol_policy, Some("redirect-to-https".to_string()));
        assert_eq!(behavior.compress, Some(true));
        assert!(behavior.allowed_methods.is_empty());
        assert!(behavior.cached_methods.is_empty());
    }

    #[test]
    fn test_map_default_cache_behavior_with_methods() {
        let cached = CachedMethods::builder()
            .quantity(2)
            .items(Method::Get)
            .items(Method::Head)
            .build()
            .unwrap();

        let allowed = AllowedMethods::builder()
            .quantity(2)
            .items(Method::Get)
            .items(Method::Head)
            .cached_methods(cached)
            .build()
            .unwrap();

        let dcb = DefaultCacheBehavior::builder()
            .target_origin_id("my-origin")
            .viewer_protocol_policy(ViewerProtocolPolicy::AllowAll)
            .allowed_methods(allowed)
            .build()
            .unwrap();

        let behavior = map_default_cache_behavior(&dcb);
        assert_eq!(behavior.viewer_protocol_policy, Some("allow-all".to_string()));
        assert_eq!(behavior.allowed_methods.len(), 2);
        assert_eq!(behavior.cached_methods.len(), 2);
    }
}
