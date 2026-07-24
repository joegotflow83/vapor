use async_graphql::{Context, Object, Result};

use crate::aws::lightsail::LightsailClient;
use crate::schema::lightsail::types::{
    LightsailDatabase, LightsailInstance, LightsailLoadBalancer, LightsailStaticIp,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct LightsailQuery;

#[Object]
impl LightsailQuery {
    async fn lightsail_instances(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LightsailInstance>> {
        let client = ctx.data::<LightsailClient>()?;
        let (instances, next_token) = client.get_instances(limit, next_token).await?;
        Ok(Page {
            items: instances.into_iter().map(LightsailInstance::from).collect(),
            next_token,
        })
    }

    async fn lightsail_databases(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LightsailDatabase>> {
        let client = ctx.data::<LightsailClient>()?;
        let (dbs, next_token) = client.get_relational_databases(limit, next_token).await?;
        Ok(Page {
            items: dbs.into_iter().map(LightsailDatabase::from).collect(),
            next_token,
        })
    }

    async fn lightsail_load_balancers(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LightsailLoadBalancer>> {
        let client = ctx.data::<LightsailClient>()?;
        let (lbs, next_token) = client.get_load_balancers(limit, next_token).await?;
        Ok(Page {
            items: lbs.into_iter().map(LightsailLoadBalancer::from).collect(),
            next_token,
        })
    }

    async fn lightsail_static_ips(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LightsailStaticIp>> {
        let client = ctx.data::<LightsailClient>()?;
        let (ips, next_token) = client.get_static_ips(limit, next_token).await?;
        Ok(Page {
            items: ips.into_iter().map(LightsailStaticIp::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::lightsail::LightsailClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::LightsailQuery;

    const ENDPOINT: &str = "https://lightsail.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn lightsail_instances_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"instances":[{"name":"my-instance","arn":"arn:aws:lightsail:us-east-1:123456789012:Instance/instance-id","blueprintId":"amazon_linux_2","bundleId":"nano_2_0","state":{"code":16,"name":"running"},"publicIpAddress":"1.2.3.4","privateIpAddress":"10.0.0.5","location":{"availabilityZone":"us-east-1a","regionName":"us-east-1"},"createdAt":1700000000,"tags":[{"key":"Env","value":"prod"}]}]}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LightsailQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ lightsailInstances { items { name arn blueprintId bundleId state publicIpAddress privateIpAddress location createdAt tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let item = &data["lightsailInstances"]["items"][0];
        assert_eq!(item["name"], "my-instance");
        assert_eq!(
            item["arn"],
            "arn:aws:lightsail:us-east-1:123456789012:Instance/instance-id"
        );
        assert_eq!(item["blueprintId"], "amazon_linux_2");
        assert_eq!(item["state"], "running");
        assert_eq!(item["publicIpAddress"], "1.2.3.4");
        assert_eq!(item["location"], "us-east-1a");
        assert_eq!(item["createdAt"], "2023-11-14T22:13:20+00:00");
        assert_eq!(item["tags"][0]["key"], "Env");
        assert_eq!(item["tags"][0]["value"], "prod");
        assert!(data["lightsailInstances"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lightsail_databases_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"relationalDatabases":[{"name":"my-db","arn":"arn:aws:lightsail:us-east-1:123456789012:RelationalDatabase/db-id","engine":"mysql","engineVersion":"8.0","state":"available","masterUsername":"dbadmin","masterEndpoint":{"port":3306,"address":"my-db.abcdefg.us-east-1.rds.amazonaws.com"},"createdAt":1700000000,"tags":[{"key":"Env","value":"prod"}]}]}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LightsailQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ lightsailDatabases { items { name arn engine engineVersion state masterUsername masterEndpoint { port address } createdAt tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let item = &data["lightsailDatabases"]["items"][0];
        assert_eq!(item["name"], "my-db");
        assert_eq!(item["engine"], "mysql");
        assert_eq!(item["engineVersion"], "8.0");
        assert_eq!(item["state"], "available");
        assert_eq!(item["masterUsername"], "dbadmin");
        assert_eq!(item["masterEndpoint"]["port"], 3306);
        assert_eq!(
            item["masterEndpoint"]["address"],
            "my-db.abcdefg.us-east-1.rds.amazonaws.com"
        );
        assert_eq!(item["createdAt"], "2023-11-14T22:13:20+00:00");
        assert_eq!(item["tags"][0]["key"], "Env");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lightsail_load_balancers_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"loadBalancers":[{"name":"my-lb","arn":"arn:aws:lightsail:us-east-1:123456789012:LoadBalancer/lb-id","dnsName":"my-lb.abcdefg.us-east-1.elb.amazonaws.com","state":"active_impaired","protocol":"HTTP_HTTPS","instancePort":80,"instanceHealthSummary":[{"instanceName":"my-instance","instanceHealth":"healthy"}],"createdAt":1700000000}]}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LightsailQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ lightsailLoadBalancers { items { name arn dnsName state protocol instancePort instanceHealthSummary { instanceName instanceHealth } createdAt } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let item = &data["lightsailLoadBalancers"]["items"][0];
        assert_eq!(item["name"], "my-lb");
        assert_eq!(item["dnsName"], "my-lb.abcdefg.us-east-1.elb.amazonaws.com");
        assert_eq!(item["state"], "active_impaired");
        assert_eq!(item["protocol"], "HTTP_HTTPS");
        assert_eq!(item["instancePort"], 80);
        assert_eq!(
            item["instanceHealthSummary"][0]["instanceName"],
            "my-instance"
        );
        assert_eq!(
            item["instanceHealthSummary"][0]["instanceHealth"],
            "healthy"
        );
        assert_eq!(item["createdAt"], "2023-11-14T22:13:20+00:00");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lightsail_static_ips_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"staticIps":[{"name":"my-static-ip","arn":"arn:aws:lightsail:us-east-1:123456789012:StaticIp/ip-id","ipAddress":"1.2.3.4","attachedTo":"my-instance","isAttached":true}]}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(LightsailQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ lightsailStaticIps { items { name arn ipAddress attachedTo isAttached } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let item = &data["lightsailStaticIps"]["items"][0];
        assert_eq!(item["name"], "my-static-ip");
        assert_eq!(
            item["arn"],
            "arn:aws:lightsail:us-east-1:123456789012:StaticIp/ip-id"
        );
        assert_eq!(item["ipAddress"], "1.2.3.4");
        assert_eq!(item["attachedTo"], "my-instance");
        assert_eq!(item["isAttached"], true);
        http_client.relaxed_requests_match();
    }
}
