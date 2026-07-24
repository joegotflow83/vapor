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
    pub last_updated_time: Option<aws_smithy_types::DateTime>,
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

fn spend_to_info(spend: &aws_sdk_budgets::types::Spend) -> BudgetAmountInfo {
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

    /// Lists budgets for an account, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `limit` is handed
    /// to AWS via `DescribeBudgetsInput::max_results` so a capped page
    /// boundary lands exactly on the returned token.
    pub async fn describe_budgets(
        &self,
        account_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<BudgetInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_budgets().account_id(account_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for budget in output.budgets.unwrap_or_default() {
                let budget_limit = budget.budget_limit.as_ref().map(spend_to_info);

                let calculated_spend =
                    budget
                        .calculated_spend
                        .as_ref()
                        .map(|cs| BudgetCalculatedSpendInfo {
                            actual_spend: cs.actual_spend.as_ref().map(spend_to_info),
                            forecasted_spend: cs.forecasted_spend.as_ref().map(spend_to_info),
                        });

                let budget_exceeded = {
                    let limit_val = budget
                        .budget_limit
                        .as_ref()
                        .and_then(|s| s.amount.parse::<f64>().ok());
                    let actual_val = budget
                        .calculated_spend
                        .as_ref()
                        .and_then(|cs| cs.actual_spend.as_ref())
                        .and_then(|s| s.amount.parse::<f64>().ok());
                    match (limit_val, actual_val) {
                        (Some(l), Some(a)) => a > l,
                        _ => false,
                    }
                };

                items.push(BudgetInfo {
                    budget_name: budget.budget_name,
                    budget_type: budget.budget_type.as_str().to_string(),
                    time_unit: Some(budget.time_unit.as_str().to_string()),
                    budget_limit,
                    calculated_spend,
                    last_updated_time: budget.last_updated_time,
                    budget_exceeded,
                });
            }

            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists notifications for a budget, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`. `limit`
    /// is handed to AWS via `DescribeNotificationsForBudgetInput::max_results`
    /// so a capped page boundary lands exactly on the returned token.
    pub async fn describe_notifications_for_budget(
        &self,
        account_id: &str,
        budget_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<BudgetNotificationInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .describe_notifications_for_budget()
                .account_id(account_id)
                .budget_name(budget_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for notification in output.notifications.unwrap_or_default() {
                items.push(BudgetNotificationInfo {
                    budget_name: budget_name.to_string(),
                    notification_type: notification.notification_type.as_str().to_string(),
                    comparison_operator: notification.comparison_operator.as_str().to_string(),
                    threshold: notification.threshold,
                    threshold_type: notification.threshold_type.map(|t| t.as_str().to_string()),
                    notification_state: notification
                        .notification_state
                        .map(|s| s.as_str().to_string()),
                });
            }

            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    // Global endpoint (no region segment) — verified against pinned
    // `aws-sdk-budgets` 1.112.0's `config/endpoint.rs` us-east-1/no-FIPS/
    // no-dualstack fixture, unlike most other files in this sweep whose
    // endpoint host is `{service}.{region}.amazonaws.com`.
    const ENDPOINT: &str = "https://budgets.amazonaws.com/";

    #[tokio::test]
    async fn describe_budgets_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AccountId":"111111111111"}"#),
            json_response(
                200,
                r#"{"Budgets":[{"BudgetName":"my-budget","BudgetType":"COST","TimeUnit":"MONTHLY","BudgetLimit":{"Amount":"100.0","Unit":"USD"},"CalculatedSpend":{"ActualSpend":{"Amount":"150.0","Unit":"USD"},"ForecastedSpend":{"Amount":"200.0","Unit":"USD"}},"LastUpdatedTime":1700000000}]}"#,
            ),
        )]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let (budgets, token) = client
            .describe_budgets("111111111111", None, None)
            .await
            .unwrap();

        assert_eq!(budgets.len(), 1);
        let b = &budgets[0];
        assert_eq!(b.budget_name, "my-budget");
        assert_eq!(b.budget_type, "COST");
        assert_eq!(b.time_unit, Some("MONTHLY".to_string()));
        assert_eq!(b.budget_limit.as_ref().unwrap().amount, "100.0");
        assert_eq!(b.budget_limit.as_ref().unwrap().unit, "USD");
        let cs = b.calculated_spend.as_ref().unwrap();
        assert_eq!(cs.actual_spend.as_ref().unwrap().amount, "150.0");
        assert_eq!(cs.forecasted_spend.as_ref().unwrap().amount, "200.0");
        assert!(b.last_updated_time.is_some());
        assert!(b.budget_exceeded, "150.0 actual > 100.0 limit");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_budgets_not_exceeded_when_actual_under_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AccountId":"111111111111"}"#),
            json_response(
                200,
                r#"{"Budgets":[{"BudgetName":"my-budget","BudgetType":"COST","TimeUnit":"MONTHLY","BudgetLimit":{"Amount":"100.0","Unit":"USD"},"CalculatedSpend":{"ActualSpend":{"Amount":"10.0","Unit":"USD"}}}]}"#,
            ),
        )]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let (budgets, _token) = client
            .describe_budgets("111111111111", None, None)
            .await
            .unwrap();

        assert!(!budgets[0].budget_exceeded);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_budgets_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"AccountId":"111111111111","NextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"Budgets":[]}"#),
        )]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let (budgets, token) = client
            .describe_budgets("111111111111", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(budgets.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_budgets_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AccountId":"111111111111","MaxResults":2}"#),
            json_response(
                200,
                r#"{"Budgets":[{"BudgetName":"b1","BudgetType":"COST","TimeUnit":"MONTHLY"},{"BudgetName":"b2","BudgetType":"COST","TimeUnit":"MONTHLY"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let (budgets, token) = client
            .describe_budgets("111111111111", Some(2), None)
            .await
            .unwrap();

        assert_eq!(budgets.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_budgets_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"AccountId":"111111111111","MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Budgets":[{"BudgetName":"b1","BudgetType":"COST","TimeUnit":"MONTHLY"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"AccountId":"111111111111","NextToken":"p2","MaxResults":9}"#,
                ),
                json_response(
                    200,
                    r#"{"Budgets":[{"BudgetName":"b2","BudgetType":"COST","TimeUnit":"MONTHLY"}]}"#,
                ),
            ),
        ]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let (budgets, token) = client
            .describe_budgets("111111111111", Some(10), None)
            .await
            .unwrap();

        assert_eq!(budgets.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_budgets_propagates_error() {
        // InvalidParameterException is not throttling-retryable (unlike
        // ThrottlingException, also modeled on this op) — see apigateway.rs's
        // retry-pitfall precedent for why a non-retryable exception is
        // required in a single-event replay list.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AccountId":"bad-account"}"#),
            json_error_response("InvalidParameterException", "invalid account id"),
        )]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let result = client.describe_budgets("bad-account", None, None).await;

        match result {
            Ok(_) => panic!("expected an error, got Ok"),
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "invalid account id");
            }
            Err(other) => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_notifications_for_budget_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"AccountId":"111111111111","BudgetName":"my-budget"}"#,
            ),
            json_response(
                200,
                r#"{"Notifications":[{"NotificationType":"ACTUAL","ComparisonOperator":"GREATER_THAN","Threshold":80.0,"ThresholdType":"PERCENTAGE","NotificationState":"ALARM"}]}"#,
            ),
        )]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let (notifications, token) = client
            .describe_notifications_for_budget("111111111111", "my-budget", None, None)
            .await
            .unwrap();

        assert_eq!(notifications.len(), 1);
        let n = &notifications[0];
        assert_eq!(n.budget_name, "my-budget");
        assert_eq!(n.notification_type, "ACTUAL");
        assert_eq!(n.comparison_operator, "GREATER_THAN");
        assert_eq!(n.threshold, 80.0);
        assert_eq!(n.threshold_type, Some("PERCENTAGE".to_string()));
        assert_eq!(n.notification_state, Some("ALARM".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_notifications_for_budget_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"AccountId":"111111111111","BudgetName":"my-budget","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Notifications":[{"NotificationType":"FORECASTED","ComparisonOperator":"LESS_THAN","Threshold":50.0}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let (notifications, token) = client
            .describe_notifications_for_budget("111111111111", "my-budget", Some(1), None)
            .await
            .unwrap();

        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].threshold_type, None);
        assert_eq!(notifications[0].notification_state, None);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_notifications_for_budget_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"AccountId":"111111111111","BudgetName":"missing-budget"}"#,
            ),
            json_error_response("NotFoundException", "budget not found"),
        )]);
        let client = BudgetsClient::new(&sdk_config(http_client.clone()));

        let result = client
            .describe_notifications_for_budget("111111111111", "missing-budget", None, None)
            .await;

        match result {
            Ok(_) => panic!("expected an error, got Ok"),
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("NotFoundException".to_string()));
                assert_eq!(message, "budget not found");
            }
            Err(other) => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
