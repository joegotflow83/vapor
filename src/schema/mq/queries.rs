use async_graphql::{Context, Object, Result};

use crate::aws::mq::MqClient;
use crate::schema::mq::types::{MqBroker, MqConfiguration};

#[derive(Default)]
pub struct MqQuery;

#[Object]
impl MqQuery {
    async fn mq_brokers(&self, ctx: &Context<'_>) -> Result<Vec<MqBroker>> {
        let client = ctx.data::<MqClient>()?;
        let brokers = client.list_brokers().await?;
        Ok(brokers.into_iter().map(MqBroker::from).collect())
    }

    async fn mq_broker(&self, ctx: &Context<'_>, broker_id: String) -> Result<Option<MqBroker>> {
        let client = ctx.data::<MqClient>()?;
        let broker = client.describe_broker(&broker_id).await?;
        Ok(broker.map(MqBroker::from))
    }

    async fn mq_configurations(&self, ctx: &Context<'_>) -> Result<Vec<MqConfiguration>> {
        let client = ctx.data::<MqClient>()?;
        let configs = client.list_configurations().await?;
        Ok(configs.into_iter().map(MqConfiguration::from).collect())
    }
}
