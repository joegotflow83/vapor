use async_graphql::{Enum, SimpleObject};

// === Enums ===

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum LoadBalancerScheme {
    InternetFacing,
    Internal,
}

impl LoadBalancerScheme {
    pub fn from_sdk(
        s: &aws_sdk_elasticloadbalancingv2::types::LoadBalancerSchemeEnum,
    ) -> Self {
        match s {
            aws_sdk_elasticloadbalancingv2::types::LoadBalancerSchemeEnum::InternetFacing => {
                Self::InternetFacing
            }
            aws_sdk_elasticloadbalancingv2::types::LoadBalancerSchemeEnum::Internal => {
                Self::Internal
            }
            _ => Self::Internal,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum LoadBalancerType {
    Application,
    Network,
    Gateway,
}

impl LoadBalancerType {
    pub fn from_sdk(
        s: &aws_sdk_elasticloadbalancingv2::types::LoadBalancerTypeEnum,
    ) -> Self {
        match s {
            aws_sdk_elasticloadbalancingv2::types::LoadBalancerTypeEnum::Application => {
                Self::Application
            }
            aws_sdk_elasticloadbalancingv2::types::LoadBalancerTypeEnum::Network => Self::Network,
            aws_sdk_elasticloadbalancingv2::types::LoadBalancerTypeEnum::Gateway => Self::Gateway,
            _ => Self::Application,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum TargetType {
    Instance,
    Ip,
    Lambda,
    Alb,
}

impl TargetType {
    pub fn from_sdk(s: &aws_sdk_elasticloadbalancingv2::types::TargetTypeEnum) -> Self {
        match s {
            aws_sdk_elasticloadbalancingv2::types::TargetTypeEnum::Instance => Self::Instance,
            aws_sdk_elasticloadbalancingv2::types::TargetTypeEnum::Ip => Self::Ip,
            aws_sdk_elasticloadbalancingv2::types::TargetTypeEnum::Lambda => Self::Lambda,
            aws_sdk_elasticloadbalancingv2::types::TargetTypeEnum::Alb => Self::Alb,
            _ => Self::Instance,
        }
    }
}

// === Output Types ===

#[derive(SimpleObject, Clone)]
pub struct AvailabilityZone {
    pub zone_name: Option<String>,
    pub subnet_id: Option<String>,
}

impl From<&aws_sdk_elasticloadbalancingv2::types::AvailabilityZone> for AvailabilityZone {
    fn from(az: &aws_sdk_elasticloadbalancingv2::types::AvailabilityZone) -> Self {
        Self {
            zone_name: az.zone_name().map(|s| s.to_string()),
            subnet_id: az.subnet_id().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LoadBalancer {
    pub arn: String,
    pub name: String,
    pub dns_name: Option<String>,
    pub scheme: Option<LoadBalancerScheme>,
    pub lb_type: Option<LoadBalancerType>,
    pub state: Option<String>,
    pub vpc_id: Option<String>,
    pub availability_zones: Vec<AvailabilityZone>,
    pub security_groups: Vec<String>,
    pub created_time: Option<String>,
}

impl From<aws_sdk_elasticloadbalancingv2::types::LoadBalancer> for LoadBalancer {
    fn from(lb: aws_sdk_elasticloadbalancingv2::types::LoadBalancer) -> Self {
        Self {
            arn: lb.load_balancer_arn().unwrap_or_default().to_string(),
            name: lb.load_balancer_name().unwrap_or_default().to_string(),
            dns_name: lb.dns_name().map(|s| s.to_string()),
            scheme: lb.scheme().map(LoadBalancerScheme::from_sdk),
            lb_type: lb.r#type().map(LoadBalancerType::from_sdk),
            state: lb
                .state()
                .and_then(|s| s.code())
                .map(|c| c.as_str().to_string()),
            vpc_id: lb.vpc_id().map(|s| s.to_string()),
            availability_zones: lb
                .availability_zones()
                .iter()
                .map(AvailabilityZone::from)
                .collect(),
            security_groups: lb
                .security_groups()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            created_time: lb.created_time().map(|d| d.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TargetGroup {
    pub arn: String,
    pub name: String,
    pub protocol: Option<String>,
    pub port: Option<i32>,
    pub vpc_id: Option<String>,
    pub target_type: Option<TargetType>,
    pub health_check_path: Option<String>,
    pub health_check_protocol: Option<String>,
    pub health_check_port: Option<String>,
    pub healthy_threshold: Option<i32>,
    pub unhealthy_threshold: Option<i32>,
    pub load_balancer_arns: Vec<String>,
}

impl From<aws_sdk_elasticloadbalancingv2::types::TargetGroup> for TargetGroup {
    fn from(tg: aws_sdk_elasticloadbalancingv2::types::TargetGroup) -> Self {
        Self {
            arn: tg.target_group_arn().unwrap_or_default().to_string(),
            name: tg.target_group_name().unwrap_or_default().to_string(),
            protocol: tg.protocol().map(|p| p.as_str().to_string()),
            port: tg.port(),
            vpc_id: tg.vpc_id().map(|s| s.to_string()),
            target_type: tg.target_type().map(TargetType::from_sdk),
            health_check_path: tg.health_check_path().map(|s| s.to_string()),
            health_check_protocol: tg.health_check_protocol().map(|p| p.as_str().to_string()),
            health_check_port: tg.health_check_port().map(|s| s.to_string()),
            healthy_threshold: tg.healthy_threshold_count(),
            unhealthy_threshold: tg.unhealthy_threshold_count(),
            load_balancer_arns: tg
                .load_balancer_arns()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TargetHealthInfo {
    pub target_id: String,
    pub port: Option<i32>,
    pub health_state: String,
    pub health_reason: Option<String>,
    pub health_description: Option<String>,
}

impl From<aws_sdk_elasticloadbalancingv2::types::TargetHealthDescription> for TargetHealthInfo {
    fn from(d: aws_sdk_elasticloadbalancingv2::types::TargetHealthDescription) -> Self {
        Self {
            target_id: d
                .target()
                .and_then(|t| t.id())
                .unwrap_or_default()
                .to_string(),
            port: d.target().and_then(|t| t.port()),
            health_state: d
                .target_health()
                .and_then(|h| h.state())
                .map(|s| s.as_str().to_string())
                .unwrap_or_default(),
            health_reason: d
                .target_health()
                .and_then(|h| h.reason())
                .map(|r| r.as_str().to_string()),
            health_description: d
                .target_health()
                .and_then(|h| h.description())
                .map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Listener {
    pub arn: String,
    pub load_balancer_arn: Option<String>,
    pub protocol: Option<String>,
    pub port: Option<i32>,
    pub default_actions: Vec<String>,
}

impl From<aws_sdk_elasticloadbalancingv2::types::Listener> for Listener {
    fn from(l: aws_sdk_elasticloadbalancingv2::types::Listener) -> Self {
        Self {
            arn: l.listener_arn().unwrap_or_default().to_string(),
            load_balancer_arn: l.load_balancer_arn().map(|s| s.to_string()),
            protocol: l.protocol().map(|p| p.as_str().to_string()),
            port: l.port(),
            default_actions: l
                .default_actions()
                .iter()
                .filter_map(|a| a.r#type().map(|t| t.as_str().to_string()))
                .collect(),
        }
    }
}

// === Listener Rules ===

#[derive(SimpleObject, Clone)]
pub struct RuleCondition {
    pub field: Option<String>,
    pub values: Vec<String>,
}

impl From<&aws_sdk_elasticloadbalancingv2::types::RuleCondition> for RuleCondition {
    fn from(c: &aws_sdk_elasticloadbalancingv2::types::RuleCondition) -> Self {
        // Prefer the legacy values() field (still populated by AWS for most condition types).
        // Fall back to type-specific config fields for conditions that don't use values().
        let values = if !c.values().is_empty() {
            c.values().iter().map(|s| s.to_string()).collect()
        } else if let Some(hh) = c.host_header_config() {
            hh.values().iter().map(|s| s.to_string()).collect()
        } else if let Some(pp) = c.path_pattern_config() {
            pp.values().iter().map(|s| s.to_string()).collect()
        } else if let Some(si) = c.source_ip_config() {
            si.values().iter().map(|s| s.to_string()).collect()
        } else if let Some(hrm) = c.http_request_method_config() {
            hrm.values().iter().map(|s| s.to_string()).collect()
        } else {
            vec![]
        };
        Self {
            field: c.field().map(|s| s.to_string()),
            values,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct RuleAction {
    pub action_type: String,
    pub target_group_arn: Option<String>,
    pub redirect_protocol: Option<String>,
    pub redirect_host: Option<String>,
    pub redirect_path: Option<String>,
    pub redirect_port: Option<String>,
    pub redirect_status_code: Option<String>,
    pub fixed_response_status_code: Option<String>,
    pub fixed_response_content_type: Option<String>,
    pub fixed_response_message_body: Option<String>,
}

impl From<&aws_sdk_elasticloadbalancingv2::types::Action> for RuleAction {
    fn from(a: &aws_sdk_elasticloadbalancingv2::types::Action) -> Self {
        Self {
            action_type: a.r#type().map(|t| t.as_str().to_string()).unwrap_or_default(),
            target_group_arn: a.target_group_arn().map(|s| s.to_string()),
            redirect_protocol: a
                .redirect_config()
                .and_then(|r| r.protocol())
                .map(|s| s.to_string()),
            redirect_host: a
                .redirect_config()
                .and_then(|r| r.host())
                .map(|s| s.to_string()),
            redirect_path: a
                .redirect_config()
                .and_then(|r| r.path())
                .map(|s| s.to_string()),
            redirect_port: a
                .redirect_config()
                .and_then(|r| r.port())
                .map(|s| s.to_string()),
            redirect_status_code: a
                .redirect_config()
                .and_then(|r| r.status_code())
                .map(|c| c.as_str().to_string()),
            fixed_response_status_code: a
                .fixed_response_config()
                .and_then(|f| f.status_code())
                .map(|s| s.to_string()),
            fixed_response_content_type: a
                .fixed_response_config()
                .and_then(|f| f.content_type())
                .map(|s| s.to_string()),
            fixed_response_message_body: a
                .fixed_response_config()
                .and_then(|f| f.message_body())
                .map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ListenerRule {
    pub rule_arn: String,
    pub priority: Option<String>,
    pub is_default: bool,
    pub conditions: Vec<RuleCondition>,
    pub actions: Vec<RuleAction>,
}

impl From<aws_sdk_elasticloadbalancingv2::types::Rule> for ListenerRule {
    fn from(r: aws_sdk_elasticloadbalancingv2::types::Rule) -> Self {
        Self {
            rule_arn: r.rule_arn().unwrap_or_default().to_string(),
            priority: r.priority().map(|s| s.to_string()),
            is_default: r.is_default().unwrap_or(false),
            conditions: r.conditions().iter().map(RuleCondition::from).collect(),
            actions: r.actions().iter().map(RuleAction::from).collect(),
        }
    }
}

// === Unit Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_elasticloadbalancingv2::types::{
        LoadBalancerSchemeEnum, LoadBalancerTypeEnum, TargetTypeEnum,
    };

    // --- LoadBalancerScheme ---

    #[test]
    fn test_load_balancer_scheme_internet_facing() {
        assert_eq!(
            LoadBalancerScheme::from_sdk(&LoadBalancerSchemeEnum::InternetFacing),
            LoadBalancerScheme::InternetFacing
        );
    }

    #[test]
    fn test_load_balancer_scheme_internal() {
        assert_eq!(
            LoadBalancerScheme::from_sdk(&LoadBalancerSchemeEnum::Internal),
            LoadBalancerScheme::Internal
        );
    }

    #[test]
    fn test_load_balancer_scheme_fallback() {
        assert_eq!(
            LoadBalancerScheme::from_sdk(&LoadBalancerSchemeEnum::from("other")),
            LoadBalancerScheme::Internal
        );
    }

    // --- LoadBalancerType ---

    #[test]
    fn test_load_balancer_type_application() {
        assert_eq!(
            LoadBalancerType::from_sdk(&LoadBalancerTypeEnum::Application),
            LoadBalancerType::Application
        );
    }

    #[test]
    fn test_load_balancer_type_network() {
        assert_eq!(
            LoadBalancerType::from_sdk(&LoadBalancerTypeEnum::Network),
            LoadBalancerType::Network
        );
    }

    #[test]
    fn test_load_balancer_type_gateway() {
        assert_eq!(
            LoadBalancerType::from_sdk(&LoadBalancerTypeEnum::Gateway),
            LoadBalancerType::Gateway
        );
    }

    #[test]
    fn test_load_balancer_type_fallback() {
        assert_eq!(
            LoadBalancerType::from_sdk(&LoadBalancerTypeEnum::from("other")),
            LoadBalancerType::Application
        );
    }

    // --- TargetType ---

    #[test]
    fn test_target_type_instance() {
        assert_eq!(
            TargetType::from_sdk(&TargetTypeEnum::Instance),
            TargetType::Instance
        );
    }

    #[test]
    fn test_target_type_ip() {
        assert_eq!(
            TargetType::from_sdk(&TargetTypeEnum::Ip),
            TargetType::Ip
        );
    }

    #[test]
    fn test_target_type_lambda() {
        assert_eq!(
            TargetType::from_sdk(&TargetTypeEnum::Lambda),
            TargetType::Lambda
        );
    }

    #[test]
    fn test_target_type_alb() {
        assert_eq!(
            TargetType::from_sdk(&TargetTypeEnum::Alb),
            TargetType::Alb
        );
    }

    #[test]
    fn test_target_type_fallback() {
        assert_eq!(
            TargetType::from_sdk(&TargetTypeEnum::from("other")),
            TargetType::Instance
        );
    }

    // --- AvailabilityZone ---

    #[test]
    fn test_availability_zone_from_sdk() {
        let az = aws_sdk_elasticloadbalancingv2::types::AvailabilityZone::builder()
            .zone_name("us-east-1a")
            .subnet_id("subnet-12345")
            .build();
        let result = AvailabilityZone::from(&az);
        assert_eq!(result.zone_name, Some("us-east-1a".to_string()));
        assert_eq!(result.subnet_id, Some("subnet-12345".to_string()));
    }

    #[test]
    fn test_availability_zone_empty() {
        let az = aws_sdk_elasticloadbalancingv2::types::AvailabilityZone::builder().build();
        let result = AvailabilityZone::from(&az);
        assert_eq!(result.zone_name, None);
        assert_eq!(result.subnet_id, None);
    }

    // --- LoadBalancer ---

    #[test]
    fn test_load_balancer_from_sdk_full() {
        let state = aws_sdk_elasticloadbalancingv2::types::LoadBalancerState::builder()
            .code(aws_sdk_elasticloadbalancingv2::types::LoadBalancerStateEnum::Active)
            .build();
        let az = aws_sdk_elasticloadbalancingv2::types::AvailabilityZone::builder()
            .zone_name("us-east-1a")
            .subnet_id("subnet-abc")
            .build();
        let lb = aws_sdk_elasticloadbalancingv2::types::LoadBalancer::builder()
            .load_balancer_arn("arn:aws:elasticloadbalancing:us-east-1:123:loadbalancer/app/my-lb/abc")
            .load_balancer_name("my-lb")
            .dns_name("my-lb.us-east-1.elb.amazonaws.com")
            .scheme(LoadBalancerSchemeEnum::InternetFacing)
            .r#type(LoadBalancerTypeEnum::Application)
            .state(state)
            .vpc_id("vpc-12345")
            .availability_zones(az)
            .security_groups("sg-12345")
            .build();
        let result = LoadBalancer::from(lb);
        assert_eq!(result.arn, "arn:aws:elasticloadbalancing:us-east-1:123:loadbalancer/app/my-lb/abc");
        assert_eq!(result.name, "my-lb");
        assert_eq!(result.dns_name, Some("my-lb.us-east-1.elb.amazonaws.com".to_string()));
        assert_eq!(result.scheme, Some(LoadBalancerScheme::InternetFacing));
        assert_eq!(result.lb_type, Some(LoadBalancerType::Application));
        assert_eq!(result.state, Some("active".to_string()));
        assert_eq!(result.vpc_id, Some("vpc-12345".to_string()));
        assert_eq!(result.availability_zones.len(), 1);
        assert_eq!(result.security_groups, vec!["sg-12345".to_string()]);
    }

    #[test]
    fn test_load_balancer_from_sdk_minimal() {
        let lb = aws_sdk_elasticloadbalancingv2::types::LoadBalancer::builder().build();
        let result = LoadBalancer::from(lb);
        assert_eq!(result.arn, "");
        assert_eq!(result.name, "");
        assert_eq!(result.dns_name, None);
        assert_eq!(result.scheme, None);
        assert_eq!(result.lb_type, None);
        assert_eq!(result.state, None);
        assert_eq!(result.vpc_id, None);
        assert!(result.availability_zones.is_empty());
        assert!(result.security_groups.is_empty());
        assert_eq!(result.created_time, None);
    }

    // --- TargetGroup ---

    #[test]
    fn test_target_group_from_sdk_full() {
        let tg = aws_sdk_elasticloadbalancingv2::types::TargetGroup::builder()
            .target_group_arn("arn:aws:elasticloadbalancing:us-east-1:123:targetgroup/my-tg/abc")
            .target_group_name("my-tg")
            .protocol(aws_sdk_elasticloadbalancingv2::types::ProtocolEnum::Https)
            .port(443)
            .vpc_id("vpc-12345")
            .target_type(TargetTypeEnum::Instance)
            .health_check_path("/health")
            .health_check_protocol(aws_sdk_elasticloadbalancingv2::types::ProtocolEnum::Http)
            .health_check_port("traffic-port")
            .healthy_threshold_count(3)
            .unhealthy_threshold_count(2)
            .load_balancer_arns("arn:aws:elasticloadbalancing:us-east-1:123:loadbalancer/app/my-lb/abc")
            .build();
        let result = TargetGroup::from(tg);
        assert_eq!(result.arn, "arn:aws:elasticloadbalancing:us-east-1:123:targetgroup/my-tg/abc");
        assert_eq!(result.name, "my-tg");
        assert_eq!(result.protocol, Some("HTTPS".to_string()));
        assert_eq!(result.port, Some(443));
        assert_eq!(result.vpc_id, Some("vpc-12345".to_string()));
        assert_eq!(result.target_type, Some(TargetType::Instance));
        assert_eq!(result.health_check_path, Some("/health".to_string()));
        assert_eq!(result.health_check_protocol, Some("HTTP".to_string()));
        assert_eq!(result.health_check_port, Some("traffic-port".to_string()));
        assert_eq!(result.healthy_threshold, Some(3));
        assert_eq!(result.unhealthy_threshold, Some(2));
        assert_eq!(result.load_balancer_arns.len(), 1);
    }

    #[test]
    fn test_target_group_from_sdk_minimal() {
        let tg = aws_sdk_elasticloadbalancingv2::types::TargetGroup::builder().build();
        let result = TargetGroup::from(tg);
        assert_eq!(result.arn, "");
        assert_eq!(result.name, "");
        assert_eq!(result.protocol, None);
        assert_eq!(result.port, None);
        assert_eq!(result.vpc_id, None);
        assert_eq!(result.target_type, None);
        assert_eq!(result.health_check_path, None);
        assert!(result.load_balancer_arns.is_empty());
    }

    // --- TargetHealthInfo ---

    #[test]
    fn test_target_health_info_from_sdk() {
        let target = aws_sdk_elasticloadbalancingv2::types::TargetDescription::builder()
            .id("i-12345")
            .port(80)
            .build();
        let health = aws_sdk_elasticloadbalancingv2::types::TargetHealth::builder()
            .state(aws_sdk_elasticloadbalancingv2::types::TargetHealthStateEnum::Healthy)
            .description("Target is healthy")
            .build();
        let desc = aws_sdk_elasticloadbalancingv2::types::TargetHealthDescription::builder()
            .target(target)
            .target_health(health)
            .build();
        let result = TargetHealthInfo::from(desc);
        assert_eq!(result.target_id, "i-12345");
        assert_eq!(result.port, Some(80));
        assert_eq!(result.health_state, "healthy");
        assert_eq!(result.health_description, Some("Target is healthy".to_string()));
    }

    #[test]
    fn test_target_health_info_empty() {
        let desc =
            aws_sdk_elasticloadbalancingv2::types::TargetHealthDescription::builder().build();
        let result = TargetHealthInfo::from(desc);
        assert_eq!(result.target_id, "");
        assert_eq!(result.port, None);
        assert_eq!(result.health_state, "");
        assert_eq!(result.health_reason, None);
        assert_eq!(result.health_description, None);
    }

    // --- Listener ---

    #[test]
    fn test_listener_from_sdk_full() {
        let action = aws_sdk_elasticloadbalancingv2::types::Action::builder()
            .r#type(aws_sdk_elasticloadbalancingv2::types::ActionTypeEnum::Forward)
            .build();
        let listener = aws_sdk_elasticloadbalancingv2::types::Listener::builder()
            .listener_arn("arn:aws:elasticloadbalancing:us-east-1:123:listener/app/my-lb/abc/def")
            .load_balancer_arn("arn:aws:elasticloadbalancing:us-east-1:123:loadbalancer/app/my-lb/abc")
            .protocol(aws_sdk_elasticloadbalancingv2::types::ProtocolEnum::Https)
            .port(443)
            .default_actions(action)
            .build();
        let result = Listener::from(listener);
        assert_eq!(result.arn, "arn:aws:elasticloadbalancing:us-east-1:123:listener/app/my-lb/abc/def");
        assert_eq!(result.load_balancer_arn, Some("arn:aws:elasticloadbalancing:us-east-1:123:loadbalancer/app/my-lb/abc".to_string()));
        assert_eq!(result.protocol, Some("HTTPS".to_string()));
        assert_eq!(result.port, Some(443));
        assert_eq!(result.default_actions, vec!["forward".to_string()]);
    }

    #[test]
    fn test_listener_from_sdk_minimal() {
        let listener = aws_sdk_elasticloadbalancingv2::types::Listener::builder().build();
        let result = Listener::from(listener);
        assert_eq!(result.arn, "");
        assert_eq!(result.load_balancer_arn, None);
        assert_eq!(result.protocol, None);
        assert_eq!(result.port, None);
        assert!(result.default_actions.is_empty());
    }

    // --- RuleCondition ---

    #[test]
    fn test_rule_condition_uses_values_field() {
        let cond = aws_sdk_elasticloadbalancingv2::types::RuleCondition::builder()
            .field("host-header")
            .values("example.com")
            .build();
        let result = RuleCondition::from(&cond);
        assert_eq!(result.field, Some("host-header".to_string()));
        assert_eq!(result.values, vec!["example.com".to_string()]);
    }

    #[test]
    fn test_rule_condition_falls_back_to_path_pattern_config() {
        let config =
            aws_sdk_elasticloadbalancingv2::types::PathPatternConditionConfig::builder()
                .values("/api/*")
                .build();
        let cond = aws_sdk_elasticloadbalancingv2::types::RuleCondition::builder()
            .field("path-pattern")
            .path_pattern_config(config)
            .build();
        let result = RuleCondition::from(&cond);
        assert_eq!(result.field, Some("path-pattern".to_string()));
        assert_eq!(result.values, vec!["/api/*".to_string()]);
    }

    // --- RuleAction ---

    #[test]
    fn test_rule_action_forward() {
        let action = aws_sdk_elasticloadbalancingv2::types::Action::builder()
            .r#type(aws_sdk_elasticloadbalancingv2::types::ActionTypeEnum::Forward)
            .target_group_arn("arn:aws:elasticloadbalancing:us-east-1:123:targetgroup/my-tg/abc")
            .build();
        let result = RuleAction::from(&action);
        assert_eq!(result.action_type, "forward");
        assert_eq!(
            result.target_group_arn,
            Some("arn:aws:elasticloadbalancing:us-east-1:123:targetgroup/my-tg/abc".to_string())
        );
        assert_eq!(result.redirect_status_code, None);
        assert_eq!(result.fixed_response_status_code, None);
    }

    #[test]
    fn test_rule_action_redirect() {
        let redirect = aws_sdk_elasticloadbalancingv2::types::RedirectActionConfig::builder()
            .protocol("HTTPS")
            .port("443")
            .status_code(
                aws_sdk_elasticloadbalancingv2::types::RedirectActionStatusCodeEnum::Http301,
            )
            .build();
        let action = aws_sdk_elasticloadbalancingv2::types::Action::builder()
            .r#type(aws_sdk_elasticloadbalancingv2::types::ActionTypeEnum::Redirect)
            .redirect_config(redirect)
            .build();
        let result = RuleAction::from(&action);
        assert_eq!(result.action_type, "redirect");
        assert_eq!(result.redirect_protocol, Some("HTTPS".to_string()));
        assert_eq!(result.redirect_port, Some("443".to_string()));
        assert_eq!(result.redirect_status_code, Some("HTTP_301".to_string()));
        assert_eq!(result.target_group_arn, None);
    }

    // --- ListenerRule ---

    #[test]
    fn test_listener_rule_from_sdk() {
        let cond = aws_sdk_elasticloadbalancingv2::types::RuleCondition::builder()
            .field("path-pattern")
            .values("/api/*")
            .build();
        let action = aws_sdk_elasticloadbalancingv2::types::Action::builder()
            .r#type(aws_sdk_elasticloadbalancingv2::types::ActionTypeEnum::Forward)
            .target_group_arn("arn:aws:elasticloadbalancing:us-east-1:123:targetgroup/my-tg/abc")
            .build();
        let rule = aws_sdk_elasticloadbalancingv2::types::Rule::builder()
            .rule_arn("arn:aws:elasticloadbalancing:us-east-1:123:listener-rule/app/my-lb/abc/def/rule1")
            .priority("10")
            .is_default(false)
            .conditions(cond)
            .actions(action)
            .build();
        let result = ListenerRule::from(rule);
        assert_eq!(
            result.rule_arn,
            "arn:aws:elasticloadbalancing:us-east-1:123:listener-rule/app/my-lb/abc/def/rule1"
        );
        assert_eq!(result.priority, Some("10".to_string()));
        assert!(!result.is_default);
        assert_eq!(result.conditions.len(), 1);
        assert_eq!(result.conditions[0].field, Some("path-pattern".to_string()));
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0].action_type, "forward");
    }

    #[test]
    fn test_listener_rule_default() {
        let action = aws_sdk_elasticloadbalancingv2::types::Action::builder()
            .r#type(aws_sdk_elasticloadbalancingv2::types::ActionTypeEnum::Forward)
            .build();
        let rule = aws_sdk_elasticloadbalancingv2::types::Rule::builder()
            .priority("default")
            .is_default(true)
            .actions(action)
            .build();
        let result = ListenerRule::from(rule);
        assert_eq!(result.rule_arn, "");
        assert_eq!(result.priority, Some("default".to_string()));
        assert!(result.is_default);
        assert!(result.conditions.is_empty());
    }
}
