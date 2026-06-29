use async_graphql::SimpleObject;

use crate::aws::budgets::{BudgetAmountInfo, BudgetCalculatedSpendInfo, BudgetInfo, BudgetNotificationInfo};

#[derive(SimpleObject, Clone)]
pub struct BudgetAmount {
    pub amount: String,
    pub unit: String,
}

impl From<BudgetAmountInfo> for BudgetAmount {
    fn from(i: BudgetAmountInfo) -> Self {
        Self {
            amount: i.amount,
            unit: i.unit,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BudgetCalculatedSpend {
    pub actual_spend: Option<BudgetAmount>,
    pub forecasted_spend: Option<BudgetAmount>,
}

impl From<BudgetCalculatedSpendInfo> for BudgetCalculatedSpend {
    fn from(i: BudgetCalculatedSpendInfo) -> Self {
        Self {
            actual_spend: i.actual_spend.map(BudgetAmount::from),
            forecasted_spend: i.forecasted_spend.map(BudgetAmount::from),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Budget {
    pub budget_name: String,
    pub budget_type: String,
    pub time_unit: Option<String>,
    pub budget_limit: Option<BudgetAmount>,
    pub calculated_spend: Option<BudgetCalculatedSpend>,
    pub last_updated_time: Option<String>,
    pub budget_exceeded: bool,
}

impl From<BudgetInfo> for Budget {
    fn from(i: BudgetInfo) -> Self {
        Self {
            budget_name: i.budget_name,
            budget_type: i.budget_type,
            time_unit: i.time_unit,
            budget_limit: i.budget_limit.map(BudgetAmount::from),
            calculated_spend: i.calculated_spend.map(BudgetCalculatedSpend::from),
            last_updated_time: i.last_updated_time,
            budget_exceeded: i.budget_exceeded,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct BudgetNotification {
    pub budget_name: String,
    pub notification_type: String,
    pub comparison_operator: String,
    pub threshold: f64,
    pub threshold_type: Option<String>,
    pub notification_state: Option<String>,
}

impl From<BudgetNotificationInfo> for BudgetNotification {
    fn from(i: BudgetNotificationInfo) -> Self {
        Self {
            budget_name: i.budget_name,
            notification_type: i.notification_type,
            comparison_operator: i.comparison_operator,
            threshold: i.threshold,
            threshold_type: i.threshold_type,
            notification_state: i.notification_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::budgets::{BudgetAmountInfo, BudgetCalculatedSpendInfo, BudgetInfo, BudgetNotificationInfo};

    #[test]
    fn test_budget_amount_from() {
        let info = BudgetAmountInfo {
            amount: "1000.00".to_string(),
            unit: "USD".to_string(),
        };
        let result = BudgetAmount::from(info);
        assert_eq!(result.amount, "1000.00");
        assert_eq!(result.unit, "USD");
    }

    #[test]
    fn test_budget_calculated_spend_from_full() {
        let info = BudgetCalculatedSpendInfo {
            actual_spend: Some(BudgetAmountInfo {
                amount: "800.00".to_string(),
                unit: "USD".to_string(),
            }),
            forecasted_spend: Some(BudgetAmountInfo {
                amount: "950.00".to_string(),
                unit: "USD".to_string(),
            }),
        };
        let result = BudgetCalculatedSpend::from(info);
        assert!(result.actual_spend.is_some());
        assert_eq!(result.actual_spend.unwrap().amount, "800.00");
        assert!(result.forecasted_spend.is_some());
    }

    #[test]
    fn test_budget_calculated_spend_from_minimal() {
        let info = BudgetCalculatedSpendInfo {
            actual_spend: None,
            forecasted_spend: None,
        };
        let result = BudgetCalculatedSpend::from(info);
        assert!(result.actual_spend.is_none());
        assert!(result.forecasted_spend.is_none());
    }

    #[test]
    fn test_budget_from_full() {
        let info = BudgetInfo {
            budget_name: "MyBudget".to_string(),
            budget_type: "COST".to_string(),
            time_unit: Some("MONTHLY".to_string()),
            budget_limit: Some(BudgetAmountInfo {
                amount: "1000.00".to_string(),
                unit: "USD".to_string(),
            }),
            calculated_spend: Some(BudgetCalculatedSpendInfo {
                actual_spend: Some(BudgetAmountInfo {
                    amount: "1200.00".to_string(),
                    unit: "USD".to_string(),
                }),
                forecasted_spend: None,
            }),
            last_updated_time: Some("2024-01-15T00:00:00Z".to_string()),
            budget_exceeded: true,
        };
        let result = Budget::from(info);
        assert_eq!(result.budget_name, "MyBudget");
        assert_eq!(result.budget_type, "COST");
        assert_eq!(result.time_unit, Some("MONTHLY".to_string()));
        assert!(result.budget_limit.is_some());
        assert_eq!(result.budget_limit.unwrap().amount, "1000.00");
        assert!(result.budget_exceeded);
        assert!(result.last_updated_time.is_some());
    }

    #[test]
    fn test_budget_from_minimal() {
        let info = BudgetInfo {
            budget_name: "MinimalBudget".to_string(),
            budget_type: "USAGE".to_string(),
            time_unit: None,
            budget_limit: None,
            calculated_spend: None,
            last_updated_time: None,
            budget_exceeded: false,
        };
        let result = Budget::from(info);
        assert_eq!(result.budget_name, "MinimalBudget");
        assert!(!result.budget_exceeded);
        assert!(result.budget_limit.is_none());
        assert!(result.calculated_spend.is_none());
    }

    #[test]
    fn test_budget_notification_from_full() {
        let info = BudgetNotificationInfo {
            budget_name: "MyBudget".to_string(),
            notification_type: "ACTUAL".to_string(),
            comparison_operator: "GREATER_THAN".to_string(),
            threshold: 80.0,
            threshold_type: Some("PERCENTAGE".to_string()),
            notification_state: Some("ALARM".to_string()),
        };
        let result = BudgetNotification::from(info);
        assert_eq!(result.budget_name, "MyBudget");
        assert_eq!(result.notification_type, "ACTUAL");
        assert_eq!(result.comparison_operator, "GREATER_THAN");
        assert_eq!(result.threshold, 80.0);
        assert_eq!(result.threshold_type, Some("PERCENTAGE".to_string()));
        assert_eq!(result.notification_state, Some("ALARM".to_string()));
    }

    #[test]
    fn test_budget_notification_from_minimal() {
        let info = BudgetNotificationInfo {
            budget_name: "MyBudget".to_string(),
            notification_type: "FORECASTED".to_string(),
            comparison_operator: "GREATER_THAN".to_string(),
            threshold: 100.0,
            threshold_type: None,
            notification_state: None,
        };
        let result = BudgetNotification::from(info);
        assert_eq!(result.notification_type, "FORECASTED");
        assert!(result.threshold_type.is_none());
        assert!(result.notification_state.is_none());
    }
}
