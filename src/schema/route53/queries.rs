use async_graphql::{Context, Object, Result};

use crate::aws::route53::Route53Client;
use crate::schema::pagination::Page;
use crate::schema::route53::types::{R53HealthCheck, R53HostedZone, R53ResourceRecordSet};

#[derive(Default)]
pub struct Route53Query;

#[Object]
impl Route53Query {
    /// List Route 53 hosted zones, optionally capped at `limit` results and
    /// resumed from `nextToken`.
    async fn r53_hosted_zones(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<R53HostedZone>> {
        let client = ctx.data::<Route53Client>()?;
        let (zones, next_token) = client.list_hosted_zones(limit, next_token).await?;
        Ok(Page {
            items: zones.into_iter().map(R53HostedZone::from).collect(),
            next_token,
        })
    }

    /// List DNS resource record sets for the given hosted zone ID, optionally
    /// capped at `limit` results and resumed from `nextToken`.
    async fn r53_records(
        &self,
        ctx: &Context<'_>,
        hosted_zone_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<R53ResourceRecordSet>> {
        let client = ctx.data::<Route53Client>()?;
        let (records, next_token) = client
            .list_resource_record_sets(&hosted_zone_id, limit, next_token)
            .await?;
        Ok(Page {
            items: records.into_iter().map(R53ResourceRecordSet::from).collect(),
            next_token,
        })
    }

    /// List Route 53 health checks, optionally capped at `limit` results and
    /// resumed from `nextToken`.
    async fn r53_health_checks(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<R53HealthCheck>> {
        let client = ctx.data::<Route53Client>()?;
        let (checks, next_token) = client.list_health_checks(limit, next_token).await?;
        Ok(Page {
            items: checks.into_iter().map(R53HealthCheck::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::route53::Route53Client;
    use crate::aws::test_util::{request, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::Route53Query;

    const BASE: &str = "https://route53.amazonaws.com/2013-04-01";

    #[tokio::test]
    async fn r53_hosted_zones_maps_fields_and_forwards_next_token() {
        // `route53.rs`'s own internal pagination loop stops once
        // `items.len() >= limit`, so `limit: 1` against a 1-item truncated
        // response satisfies it with a single `ReplayEvent` (gotcha 29).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/hostedzone?maxitems=1"), ""),
            xml_response(
                200,
                "<ListHostedZonesResponse>\
                    <HostedZones>\
                        <HostedZone>\
                            <Id>/hostedzone/Z1D633PJN98FT9</Id>\
                            <Name>example.com.</Name>\
                            <CallerReference>ref-1</CallerReference>\
                            <Config><PrivateZone>false</PrivateZone></Config>\
                            <ResourceRecordSetCount>5</ResourceRecordSetCount>\
                        </HostedZone>\
                    </HostedZones>\
                    <Marker></Marker>\
                    <IsTruncated>true</IsTruncated>\
                    <NextMarker>next-marker-1</NextMarker>\
                    <MaxItems>1</MaxItems>\
                </ListHostedZonesResponse>",
            ),
        )]);
        let schema = build_query_schema(Route53Query)
            .data(Route53Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ r53HostedZones(limit: 1) { items { id name privateZone \
                 recordCount comment } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["r53HostedZones"]["items"];
        assert_eq!(items[0]["id"], "Z1D633PJN98FT9");
        assert_eq!(items[0]["name"], "example.com.");
        assert_eq!(items[0]["privateZone"], false);
        assert_eq!(items[0]["recordCount"], 5);
        assert!(items[0]["comment"].is_null());
        assert_eq!(json["r53HostedZones"]["nextToken"], "next-marker-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn r53_records_maps_fields_for_given_hosted_zone() {
        // No `limit` arg here: the mocked response is non-truncated (no
        // `NextRecordName`), so `list_resource_record_sets`'s internal loop
        // stops after one page regardless — no gotcha-29 risk, and it avoids
        // needing to byte-match the JSON-encoded `RecordSetCursor` token.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/hostedzone/ZTEST123/rrset"), ""),
            xml_response(
                200,
                "<ListResourceRecordSetsResponse>\
                    <ResourceRecordSets>\
                        <ResourceRecordSet>\
                            <Name>www.example.com.</Name>\
                            <Type>A</Type>\
                            <TTL>300</TTL>\
                            <ResourceRecords>\
                                <ResourceRecord><Value>1.2.3.4</Value></ResourceRecord>\
                            </ResourceRecords>\
                        </ResourceRecordSet>\
                    </ResourceRecordSets>\
                    <IsTruncated>false</IsTruncated>\
                    <MaxItems>100</MaxItems>\
                </ListResourceRecordSetsResponse>",
            ),
        )]);
        let schema = build_query_schema(Route53Query)
            .data(Route53Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ r53Records(hostedZoneId: "ZTEST123") { items { name recordType \
                 ttl records aliasTarget { dnsName } } nextToken } }"#
                    .replace('\\', "")
                    .as_str(),
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["r53Records"]["items"];
        assert_eq!(items[0]["name"], "www.example.com.");
        assert_eq!(items[0]["recordType"], "A");
        assert_eq!(items[0]["ttl"], 300);
        assert_eq!(items[0]["records"][0], "1.2.3.4");
        assert!(items[0]["aliasTarget"].is_null());
        assert!(json["r53Records"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn r53_health_checks_maps_fields_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/healthcheck?maxitems=1"), ""),
            xml_response(
                200,
                "<ListHealthChecksResponse>\
                    <HealthChecks>\
                        <HealthCheck>\
                            <Id>abc-123</Id>\
                            <CallerReference>ref-hc</CallerReference>\
                            <HealthCheckConfig>\
                                <Type>HTTPS</Type>\
                                <IPAddress>1.2.3.4</IPAddress>\
                                <Port>443</Port>\
                                <ResourcePath>/health</ResourcePath>\
                                <FailureThreshold>3</FailureThreshold>\
                            </HealthCheckConfig>\
                            <HealthCheckVersion>1</HealthCheckVersion>\
                        </HealthCheck>\
                    </HealthChecks>\
                    <Marker></Marker>\
                    <IsTruncated>true</IsTruncated>\
                    <NextMarker>hc-next</NextMarker>\
                    <MaxItems>1</MaxItems>\
                </ListHealthChecksResponse>",
            ),
        )]);
        let schema = build_query_schema(Route53Query)
            .data(Route53Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ r53HealthChecks(limit: 1) { items { id arn healthCheckVersion \
                 config { ipAddress port healthCheckType resourcePath failureThreshold \
                 regions } } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["r53HealthChecks"]["items"];
        assert_eq!(items[0]["id"], "abc-123");
        assert!(items[0]["arn"].is_null());
        assert_eq!(items[0]["healthCheckVersion"], 1);
        let cfg = &items[0]["config"];
        assert_eq!(cfg["ipAddress"], "1.2.3.4");
        assert_eq!(cfg["port"], 443);
        assert_eq!(cfg["healthCheckType"], "HTTPS");
        assert_eq!(cfg["resourcePath"], "/health");
        assert_eq!(cfg["failureThreshold"], 3);
        assert!(cfg["regions"].as_array().unwrap().is_empty());
        assert_eq!(json["r53HealthChecks"]["nextToken"], "hc-next");
        http_client.relaxed_requests_match();
    }
}
