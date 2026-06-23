use async_graphql::SimpleObject;

/// An AWS Route 53 hosted zone.
#[derive(SimpleObject, Clone)]
pub struct R53HostedZone {
    /// The hosted zone ID, stripped of the /hostedzone/ prefix.
    pub id: String,
    pub name: String,
    pub private_zone: bool,
    pub record_count: Option<i64>,
    pub comment: Option<String>,
}

impl From<aws_sdk_route53::types::HostedZone> for R53HostedZone {
    fn from(hz: aws_sdk_route53::types::HostedZone) -> Self {
        let id = hz.id().trim_start_matches("/hostedzone/").to_string();
        let (private_zone, comment) = hz
            .config()
            .map(|c| (c.private_zone(), c.comment().map(|s| s.to_string())))
            .unwrap_or((false, None));
        Self {
            id,
            name: hz.name().to_string(),
            private_zone,
            record_count: hz.resource_record_set_count(),
            comment,
        }
    }
}

/// The alias target for an aliased DNS record.
#[derive(SimpleObject, Clone)]
pub struct R53AliasTarget {
    pub hosted_zone_id: String,
    pub dns_name: String,
    pub evaluate_target_health: bool,
}

impl From<&aws_sdk_route53::types::AliasTarget> for R53AliasTarget {
    fn from(a: &aws_sdk_route53::types::AliasTarget) -> Self {
        Self {
            hosted_zone_id: a.hosted_zone_id().to_string(),
            dns_name: a.dns_name().to_string(),
            evaluate_target_health: a.evaluate_target_health(),
        }
    }
}

/// A DNS resource record set in a Route 53 hosted zone.
#[derive(SimpleObject, Clone)]
pub struct R53ResourceRecordSet {
    pub name: String,
    /// Record type as string (A, AAAA, CNAME, MX, NS, PTR, SOA, SPF, SRV, TXT, CAA, ...).
    pub record_type: String,
    pub ttl: Option<i64>,
    /// Individual record values extracted from resource_records.
    pub records: Vec<String>,
    pub alias_target: Option<R53AliasTarget>,
    pub set_identifier: Option<String>,
    pub weight: Option<i64>,
    pub region: Option<String>,
    pub failover: Option<String>,
    pub health_check_id: Option<String>,
}

impl From<aws_sdk_route53::types::ResourceRecordSet> for R53ResourceRecordSet {
    fn from(rrs: aws_sdk_route53::types::ResourceRecordSet) -> Self {
        let records = rrs
            .resource_records()
            .iter()
            .map(|r| r.value().to_string())
            .collect();
        let alias_target = rrs.alias_target().map(R53AliasTarget::from);
        let record_type = rrs.r#type().as_str().to_string();
        let region = rrs.region().map(|r| r.as_str().to_string());
        let failover = rrs.failover().map(|f| f.as_str().to_string());
        Self {
            name: rrs.name().to_string(),
            record_type,
            ttl: rrs.ttl(),
            records,
            alias_target,
            set_identifier: rrs.set_identifier().map(|s| s.to_string()),
            weight: rrs.weight(),
            region,
            failover,
            health_check_id: rrs.health_check_id().map(|s| s.to_string()),
        }
    }
}

/// Configuration for a Route 53 health check.
#[derive(SimpleObject, Clone)]
pub struct R53HealthCheckConfig {
    pub ip_address: Option<String>,
    pub port: Option<i32>,
    /// Health check type: HTTP | HTTPS | HTTP_STR_MATCH | HTTPS_STR_MATCH | TCP | CALCULATED | CLOUDWATCH_METRIC
    pub health_check_type: Option<String>,
    pub resource_path: Option<String>,
    pub fqdn: Option<String>,
    pub search_string: Option<String>,
    pub request_interval: Option<i32>,
    pub failure_threshold: Option<i32>,
    pub inverted: Option<bool>,
    pub disabled: Option<bool>,
    pub regions: Vec<String>,
}

impl From<&aws_sdk_route53::types::HealthCheckConfig> for R53HealthCheckConfig {
    fn from(c: &aws_sdk_route53::types::HealthCheckConfig) -> Self {
        Self {
            ip_address: c.ip_address().map(|s| s.to_string()),
            port: c.port(),
            health_check_type: Some(c.r#type().as_str().to_string()),
            resource_path: c.resource_path().map(|s| s.to_string()),
            fqdn: c.fully_qualified_domain_name().map(|s| s.to_string()),
            search_string: c.search_string().map(|s| s.to_string()),
            request_interval: c.request_interval(),
            failure_threshold: c.failure_threshold(),
            inverted: c.inverted(),
            disabled: c.disabled(),
            regions: c
                .regions()
                .iter()
                .map(|r| r.as_str().to_string())
                .collect(),
        }
    }
}

/// An AWS Route 53 health check.
#[derive(SimpleObject, Clone)]
pub struct R53HealthCheck {
    pub id: String,
    pub arn: Option<String>,
    pub health_check_version: Option<i64>,
    pub config: Option<R53HealthCheckConfig>,
}

impl From<aws_sdk_route53::types::HealthCheck> for R53HealthCheck {
    fn from(hc: aws_sdk_route53::types::HealthCheck) -> Self {
        let config = hc.health_check_config().map(R53HealthCheckConfig::from);
        Self {
            id: hc.id().to_string(),
            // `HealthCheck` carries no ARN field in the SDK; left unset.
            arn: None,
            health_check_version: Some(hc.health_check_version()),
            config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hosted_zone_strips_prefix() {
        let hz = aws_sdk_route53::types::HostedZone::builder()
            .id("/hostedzone/Z1D633PJN98FT9")
            .name("example.com.")
            .caller_reference("ref")
            .build()
            .unwrap();
        let zone = R53HostedZone::from(hz);
        assert_eq!(zone.id, "Z1D633PJN98FT9");
        assert_eq!(zone.name, "example.com.");
        assert_eq!(zone.private_zone, false);
        assert_eq!(zone.comment, None);
        assert_eq!(zone.record_count, None);
    }

    #[test]
    fn test_hosted_zone_with_config() {
        let config = aws_sdk_route53::types::HostedZoneConfig::builder()
            .private_zone(true)
            .comment("My private zone")
            .build();
        let hz = aws_sdk_route53::types::HostedZone::builder()
            .id("/hostedzone/ZXYZ123")
            .name("internal.example.com.")
            .caller_reference("ref2")
            .config(config)
            .resource_record_set_count(42)
            .build()
            .unwrap();
        let zone = R53HostedZone::from(hz);
        assert_eq!(zone.id, "ZXYZ123");
        assert_eq!(zone.private_zone, true);
        assert_eq!(zone.comment, Some("My private zone".to_string()));
        assert_eq!(zone.record_count, Some(42));
    }

    #[test]
    fn test_record_set_from_basic() {
        let rrs = aws_sdk_route53::types::ResourceRecordSet::builder()
            .name("www.example.com.")
            .r#type(aws_sdk_route53::types::RrType::A)
            .ttl(300)
            .resource_records(
                aws_sdk_route53::types::ResourceRecord::builder()
                    .value("1.2.3.4")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let record = R53ResourceRecordSet::from(rrs);
        assert_eq!(record.name, "www.example.com.");
        assert_eq!(record.record_type, "A");
        assert_eq!(record.ttl, Some(300));
        assert_eq!(record.records, vec!["1.2.3.4".to_string()]);
        assert!(record.alias_target.is_none());
    }

    #[test]
    fn test_record_set_alias() {
        let alias = aws_sdk_route53::types::AliasTarget::builder()
            .hosted_zone_id("Z2FDTNDATAQYW2")
            .dns_name("d111111abcdef8.cloudfront.net.")
            .evaluate_target_health(false)
            .build()
            .unwrap();
        let rrs = aws_sdk_route53::types::ResourceRecordSet::builder()
            .name("cdn.example.com.")
            .r#type(aws_sdk_route53::types::RrType::A)
            .alias_target(alias)
            .build()
            .unwrap();
        let record = R53ResourceRecordSet::from(rrs);
        assert!(record.alias_target.is_some());
        let at = record.alias_target.unwrap();
        assert_eq!(at.hosted_zone_id, "Z2FDTNDATAQYW2");
        assert_eq!(at.dns_name, "d111111abcdef8.cloudfront.net.");
        assert_eq!(at.evaluate_target_health, false);
    }

    #[test]
    fn test_health_check_from() {
        let config = aws_sdk_route53::types::HealthCheckConfig::builder()
            .r#type(aws_sdk_route53::types::HealthCheckType::Https)
            .ip_address("1.2.3.4")
            .port(443)
            .resource_path("/health")
            .failure_threshold(3)
            .build()
            .unwrap();
        let hc = aws_sdk_route53::types::HealthCheck::builder()
            .id("abc-123")
            .health_check_version(1)
            .health_check_config(config)
            .caller_reference("ref")
            .build()
            .unwrap();
        let check = R53HealthCheck::from(hc);
        assert_eq!(check.id, "abc-123");
        assert_eq!(check.health_check_version, Some(1));
        let cfg = check.config.unwrap();
        assert_eq!(cfg.health_check_type, Some("HTTPS".to_string()));
        assert_eq!(cfg.ip_address, Some("1.2.3.4".to_string()));
        assert_eq!(cfg.port, Some(443));
        assert_eq!(cfg.resource_path, Some("/health".to_string()));
        assert_eq!(cfg.failure_threshold, Some(3));
    }

    #[test]
    fn test_health_check_minimal() {
        let config = aws_sdk_route53::types::HealthCheckConfig::builder()
            .r#type(aws_sdk_route53::types::HealthCheckType::Http)
            .build()
            .unwrap();
        let hc = aws_sdk_route53::types::HealthCheck::builder()
            .id("min-id")
            .health_check_version(2)
            .health_check_config(config)
            .caller_reference("ref3")
            .build()
            .unwrap();
        let check = R53HealthCheck::from(hc);
        assert_eq!(check.id, "min-id");
        assert_eq!(check.arn, None);
        let cfg = check.config.unwrap();
        assert_eq!(cfg.health_check_type, Some("HTTP".to_string()));
        assert!(cfg.regions.is_empty());
    }
}
