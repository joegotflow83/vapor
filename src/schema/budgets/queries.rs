use async_graphql::{Context, Object, Result};

use crate::aws::budgets::BudgetsClient;
use crate::schema::budgets::types::{Budget, BudgetNotification};

#[derive(Default)]
pub struct BudgetsQuery;

#[Object]
impl BudgetsQuery {
    async fn budgets(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<Budget>> {
        let client = ctx.data::<BudgetsClient>()?;
        let items = client.describe_budgets(&account_id).await?;
        Ok(items.into_iter().map(Budget::from).collect())
    }

    async fn budget_notifications(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        budget_name: String,
    ) -> Result<Vec<BudgetNotification>> {
        let client = ctx.data::<BudgetsClient>()?;
        let items = client
            .describe_notifications_for_budget(&account_id, &budget_name)
            .await?;
        Ok(items.into_iter().map(BudgetNotification::from).collect())
    }
}
