use async_graphql::{Context, Object, Result};

use crate::aws::iot::IotClient;
use crate::schema::iot::types::{IotCertificate, IotPolicy, IotThing, IotThingGroup, IotTopicRule};

#[derive(Default)]
pub struct IotQuery;

#[Object]
impl IotQuery {
    async fn iot_things(
        &self,
        ctx: &Context<'_>,
        thing_type_name: Option<String>,
        attribute_name: Option<String>,
        attribute_value: Option<String>,
    ) -> Result<Vec<IotThing>> {
        let client = ctx.data::<IotClient>()?;
        let items = client
            .list_things(thing_type_name, attribute_name, attribute_value)
            .await?;
        Ok(items.into_iter().map(IotThing::from).collect())
    }

    async fn iot_thing_groups(
        &self,
        ctx: &Context<'_>,
        parent_group: Option<String>,
    ) -> Result<Vec<IotThingGroup>> {
        let client = ctx.data::<IotClient>()?;
        let items = client.list_thing_groups(parent_group).await?;
        Ok(items.into_iter().map(IotThingGroup::from).collect())
    }

    async fn iot_policies(&self, ctx: &Context<'_>) -> Result<Vec<IotPolicy>> {
        let client = ctx.data::<IotClient>()?;
        let items = client.list_policies().await?;
        Ok(items.into_iter().map(IotPolicy::from).collect())
    }

    async fn iot_certificates(
        &self,
        ctx: &Context<'_>,
        ascending_order: Option<bool>,
    ) -> Result<Vec<IotCertificate>> {
        let client = ctx.data::<IotClient>()?;
        let items = client.list_certificates(ascending_order).await?;
        Ok(items.into_iter().map(IotCertificate::from).collect())
    }

    async fn iot_topic_rules(
        &self,
        ctx: &Context<'_>,
        topic_rule_disabled: Option<bool>,
    ) -> Result<Vec<IotTopicRule>> {
        let client = ctx.data::<IotClient>()?;
        let items = client.list_topic_rules(topic_rule_disabled).await?;
        Ok(items.into_iter().map(IotTopicRule::from).collect())
    }
}
