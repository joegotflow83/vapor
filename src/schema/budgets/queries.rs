use async_graphql::{Context, Object, Result};

use crate::aws::budgets::BudgetsClient;
use crate::schema::budgets::types::{Budget, BudgetNotification};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct BudgetsQuery;

#[Object]
impl BudgetsQuery {
    /// Lists budgets for an account, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn budgets(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Budget>> {
        let client = ctx.data::<BudgetsClient>()?;
        let (items, token) = client.describe_budgets(&account_id, limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(Budget::from).collect(),
            next_token: token,
        })
    }

    /// Lists notifications for a budget, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn budget_notifications(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        budget_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BudgetNotification>> {
        let client = ctx.data::<BudgetsClient>()?;
        let (items, token) = client
            .describe_notifications_for_budget(&account_id, &budget_name, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(BudgetNotification::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::budgets::BudgetsClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    const ENDPOINT: &str = "https://budgets.amazonaws.com/";

    #[tokio::test]
    async fn budgets_resolver_maps_items_and_forwards_account_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"AccountId":"111111111111"}"#),
            json_response(
                200,
                r#"{"Budgets":[{"BudgetName":"my-budget","BudgetType":"COST","TimeUnit":"MONTHLY","BudgetLimit":{"Amount":"100.0","Unit":"USD"},"CalculatedSpend":{"ActualSpend":{"Amount":"150.0","Unit":"USD"}}}]}"#,
            ),
        )]);
        let schema = build_query_schema(BudgetsQuery)
            .data(BudgetsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ budgets(accountId: "111111111111") { items { budgetName budgetType timeUnit budgetExceeded budgetLimit { amount unit } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["budgets"]["items"];
        assert_eq!(items[0]["budgetName"], "my-budget");
        assert_eq!(items[0]["budgetType"], "COST");
        assert_eq!(items[0]["timeUnit"], "MONTHLY");
        assert_eq!(items[0]["budgetExceeded"], true);
        assert_eq!(items[0]["budgetLimit"]["amount"], "100.0");
        assert!(json["budgets"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn budget_notifications_resolver_maps_items_and_forwards_args() {
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
        let schema = build_query_schema(BudgetsQuery)
            .data(BudgetsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ budgetNotifications(accountId: "111111111111", budgetName: "my-budget") { items { notificationType comparisonOperator threshold thresholdType notificationState } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["budgetNotifications"]["items"];
        assert_eq!(items[0]["notificationType"], "ACTUAL");
        assert_eq!(items[0]["comparisonOperator"], "GREATER_THAN");
        assert_eq!(items[0]["threshold"], 80.0);
        assert_eq!(items[0]["thresholdType"], "PERCENTAGE");
        assert_eq!(items[0]["notificationState"], "ALARM");
        assert!(json["budgetNotifications"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
