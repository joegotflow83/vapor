use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct BudgetAmountInfo {
    pub amount: String,
    pub unit: String,
}

pub struct BudgetCalculatedSpendInfo {
    pub actual_spend: Option<BudgetAmountInfo>,
    pub forecasted_spend: Option<BudgetAmountInfo>,
}

pub struct BudgetInfo {
    pub budget_name: String,
    pub budget_type: String,
    pub time_unit: Option<String>,
    pub budget_limit: Option<BudgetAmountInfo>,
    pub calculated_spend: Option<BudgetCalculatedSpendInfo>,
    pub last_updated_time: Option<String>,
    pub budget_exceeded: bool,
}

pub struct BudgetNotificationInfo {
    pub budget_name: String,
    pub notification_type: String,
    pub comparison_operator: String,
    pub threshold: f64,
    pub threshold_type: Option<String>,
    pub notification_state: Option<String>,
}

fn spend_to_info(
    spend: &aws_sdk_budgets::types::Spend,
) -> BudgetAmountInfo {
    BudgetAmountInfo {
        amount: spend.amount().to_string(),
        unit: spend.unit().to_string(),
    }
}

pub struct BudgetsClient {
    inner: aws_sdk_budgets::Client,
}

impl BudgetsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_budgets::Client::new(config),
        }
    }

    pub async fn describe_budgets(
        &self,
        account_id: &str,
    ) -> Result<Vec<BudgetInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .describe_budgets()
                .account_id(account_id)
                .max_results(100);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for budget in output.budgets() {
                let budget_limit = budget.budget_limit().map(spend_to_info);

                let calculated_spend = budget.calculated_spend().map(|cs| {
                    BudgetCalculatedSpendInfo {
                        actual_spend: cs.actual_spend().map(spend_to_info),
                        forecasted_spend: cs.forecasted_spend().map(spend_to_info),
                    }
                });

                let budget_exceeded = {
                    let limit_val = budget
                        .budget_limit()
                        .and_then(|s| s.amount().parse::<f64>().ok());
                    let actual_val = budget
                        .calculated_spend()
                        .and_then(|cs| cs.actual_spend())
                        .and_then(|s| s.amount().parse::<f64>().ok());
                    match (limit_val, actual_val) {
                        (Some(l), Some(a)) => a > l,
                        _ => false,
                    }
                };

                items.push(BudgetInfo {
                    budget_name: budget.budget_name().to_string(),
                    budget_type: budget.budget_type().as_str().to_string(),
                    time_unit: Some(budget.time_unit().as_str().to_string()),
                    budget_limit,
                    calculated_spend,
                    last_updated_time: budget.last_updated_time().map(|t| t.to_string()),
                    budget_exceeded,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_notifications_for_budget(
        &self,
        account_id: &str,
        budget_name: &str,
    ) -> Result<Vec<BudgetNotificationInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .describe_notifications_for_budget()
                .account_id(account_id)
                .budget_name(budget_name)
                .max_results(100);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for notification in output.notifications() {
                items.push(BudgetNotificationInfo {
                    budget_name: budget_name.to_string(),
                    notification_type: notification.notification_type().as_str().to_string(),
                    comparison_operator: notification.comparison_operator().as_str().to_string(),
                    threshold: notification.threshold(),
                    threshold_type: notification
                        .threshold_type()
                        .map(|t| t.as_str().to_string()),
                    notification_state: notification
                        .notification_state()
                        .map(|s| s.as_str().to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
