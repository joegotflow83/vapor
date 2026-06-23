use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct ShieldSubscription {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub auto_renew: Option<String>,
    pub proactive_engagement_status: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct ShieldProtection {
    pub id: Option<String>,
    pub name: Option<String>,
    pub resource_arn: Option<String>,
    pub health_check_ids: Vec<String>,
    pub protection_arn: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct ProtectionGroup {
    pub protection_group_id: String,
    pub aggregation: Option<String>,
    pub pattern: Option<String>,
    pub resource_type: Option<String>,
    pub members: Vec<String>,
    pub protection_group_arn: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct AttackSummary {
    pub attack_id: Option<String>,
    pub resource_arn: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub attack_vectors: Vec<String>,
}

impl From<aws_sdk_shield::types::Subscription> for ShieldSubscription {
    fn from(sub: aws_sdk_shield::types::Subscription) -> Self {
        Self {
            start_time: sub.start_time().map(|dt| dt.to_string()),
            end_time: sub.end_time().map(|dt| dt.to_string()),
            auto_renew: sub.auto_renew().map(|ar| ar.as_str().to_string()),
            proactive_engagement_status: sub
                .proactive_engagement_status()
                .map(|s| s.as_str().to_string()),
        }
    }
}

impl From<aws_sdk_shield::types::Protection> for ShieldProtection {
    fn from(p: aws_sdk_shield::types::Protection) -> Self {
        Self {
            id: p.id().map(|s| s.to_string()),
            name: p.name().map(|s| s.to_string()),
            resource_arn: p.resource_arn().map(|s| s.to_string()),
            health_check_ids: p.health_check_ids().to_vec(),
            protection_arn: p.protection_arn().map(|s| s.to_string()),
        }
    }
}

impl From<aws_sdk_shield::types::ProtectionGroup> for ProtectionGroup {
    fn from(pg: aws_sdk_shield::types::ProtectionGroup) -> Self {
        Self {
            protection_group_id: pg.protection_group_id().to_string(),
            aggregation: Some(pg.aggregation().as_str().to_string()),
            pattern: Some(pg.pattern().as_str().to_string()),
            resource_type: pg.resource_type().map(|rt| rt.as_str().to_string()),
            members: pg.members().to_vec(),
            protection_group_arn: pg.protection_group_arn().map(|s| s.to_string()),
        }
    }
}

impl From<aws_sdk_shield::types::AttackSummary> for AttackSummary {
    fn from(a: aws_sdk_shield::types::AttackSummary) -> Self {
        Self {
            attack_id: a.attack_id().map(|s| s.to_string()),
            resource_arn: a.resource_arn().map(|s| s.to_string()),
            start_time: a.start_time().map(|dt| dt.to_string()),
            end_time: a.end_time().map(|dt| dt.to_string()),
            attack_vectors: a
                .attack_vectors()
                .iter()
                .map(|v| v.vector_type().to_string())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shield_subscription_from_minimal() {
        let sub = aws_sdk_shield::types::Subscription::builder().build();
        let result = ShieldSubscription::from(sub);
        assert!(result.start_time.is_none());
        assert!(result.end_time.is_none());
        assert!(result.auto_renew.is_none());
        assert!(result.proactive_engagement_status.is_none());
    }

    #[test]
    fn test_shield_subscription_from_full() {
        let sub = aws_sdk_shield::types::Subscription::builder()
            .auto_renew(aws_sdk_shield::types::AutoRenew::Enabled)
            .proactive_engagement_status(
                aws_sdk_shield::types::ProactiveEngagementStatus::Enabled,
            )
            .build();
        let result = ShieldSubscription::from(sub);
        assert_eq!(result.auto_renew, Some("ENABLED".to_string()));
        assert_eq!(
            result.proactive_engagement_status,
            Some("ENABLED".to_string())
        );
    }

    #[test]
    fn test_shield_protection_from_minimal() {
        let p = aws_sdk_shield::types::Protection::builder().build();
        let result = ShieldProtection::from(p);
        assert!(result.id.is_none());
        assert!(result.name.is_none());
        assert!(result.resource_arn.is_none());
        assert!(result.health_check_ids.is_empty());
        assert!(result.protection_arn.is_none());
    }

    #[test]
    fn test_shield_protection_from_full() {
        let p = aws_sdk_shield::types::Protection::builder()
            .id("prot-123")
            .name("my-protection")
            .resource_arn("arn:aws:ec2::123:eip/eipalloc-abc")
            .health_check_ids("hc-1")
            .health_check_ids("hc-2")
            .protection_arn("arn:aws:shield::123:protection/prot-123")
            .build();
        let result = ShieldProtection::from(p);
        assert_eq!(result.id, Some("prot-123".to_string()));
        assert_eq!(result.name, Some("my-protection".to_string()));
        assert_eq!(
            result.resource_arn,
            Some("arn:aws:ec2::123:eip/eipalloc-abc".to_string())
        );
        assert_eq!(result.health_check_ids, vec!["hc-1", "hc-2"]);
        assert_eq!(
            result.protection_arn,
            Some("arn:aws:shield::123:protection/prot-123".to_string())
        );
    }

    #[test]
    fn test_protection_group_from_minimal() {
        let pg = aws_sdk_shield::types::ProtectionGroup::builder()
            .protection_group_id("pg-123")
            .aggregation(aws_sdk_shield::types::ProtectionGroupAggregation::Sum)
            .pattern(aws_sdk_shield::types::ProtectionGroupPattern::All)
            .set_members(Some(vec![]))
            .build()
            .expect("pg build");
        let result = ProtectionGroup::from(pg);
        assert_eq!(result.protection_group_id, "pg-123");
        assert_eq!(result.aggregation, Some("SUM".to_string()));
        assert_eq!(result.pattern, Some("ALL".to_string()));
        assert!(result.resource_type.is_none());
        assert!(result.members.is_empty());
        assert!(result.protection_group_arn.is_none());
    }

    #[test]
    fn test_protection_group_from_full() {
        let pg = aws_sdk_shield::types::ProtectionGroup::builder()
            .protection_group_id("pg-456")
            .aggregation(aws_sdk_shield::types::ProtectionGroupAggregation::Max)
            .pattern(aws_sdk_shield::types::ProtectionGroupPattern::ByResourceType)
            .resource_type(aws_sdk_shield::types::ProtectedResourceType::ApplicationLoadBalancer)
            .members("arn:aws:elasticloadbalancing::123:loadbalancer/app/alb/abc")
            .protection_group_arn("arn:aws:shield::123:protection-group/pg-456")
            .build()
            .expect("pg build");
        let result = ProtectionGroup::from(pg);
        assert_eq!(result.protection_group_id, "pg-456");
        assert_eq!(result.aggregation, Some("MAX".to_string()));
        assert_eq!(result.pattern, Some("BY_RESOURCE_TYPE".to_string()));
        assert_eq!(
            result.resource_type,
            Some("APPLICATION_LOAD_BALANCER".to_string())
        );
        assert_eq!(result.members.len(), 1);
        assert_eq!(
            result.protection_group_arn,
            Some("arn:aws:shield::123:protection-group/pg-456".to_string())
        );
    }

    #[test]
    fn test_attack_summary_from_minimal() {
        let a = aws_sdk_shield::types::AttackSummary::builder()
            .build();
        let result = AttackSummary::from(a);
        assert!(result.attack_id.is_none());
        assert!(result.resource_arn.is_none());
        assert!(result.start_time.is_none());
        assert!(result.end_time.is_none());
        assert!(result.attack_vectors.is_empty());
    }

    #[test]
    fn test_attack_summary_from_full() {
        let vector = aws_sdk_shield::types::AttackVectorDescription::builder()
            .vector_type("UDP_FRAGMENT")
            .build()
            .expect("vector build");
        let a = aws_sdk_shield::types::AttackSummary::builder()
            .attack_id("atk-123")
            .resource_arn("arn:aws:ec2::123:eip/eipalloc-abc")
            .attack_vectors(vector)
            .build();
        let result = AttackSummary::from(a);
        assert_eq!(result.attack_id, Some("atk-123".to_string()));
        assert_eq!(
            result.resource_arn,
            Some("arn:aws:ec2::123:eip/eipalloc-abc".to_string())
        );
        assert_eq!(result.attack_vectors, vec!["UDP_FRAGMENT".to_string()]);
    }
}
