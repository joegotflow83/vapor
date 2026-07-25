use aws_config::SdkConfig;

use crate::aws::pagination::apply_limit;
use crate::error::VaporError;

#[derive(Debug)]
pub struct LightsailInstanceInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub blueprint_id: Option<String>,
    pub bundle_id: Option<String>,
    pub state: Option<String>,
    pub public_ip_address: Option<String>,
    pub private_ip_address: Option<String>,
    pub location: Option<String>,
    pub created_at: Option<aws_smithy_types::DateTime>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct LightsailEndpointInfo {
    pub port: i32,
    pub address: String,
}

#[derive(Debug)]
pub struct LightsailDatabaseInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    pub state: Option<String>,
    pub master_username: Option<String>,
    pub master_endpoint: Option<LightsailEndpointInfo>,
    pub created_at: Option<aws_smithy_types::DateTime>,
    pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct LightsailInstanceHealthInfo {
    pub instance_name: Option<String>,
    pub instance_health: Option<String>,
}

#[derive(Debug)]
pub struct LightsailLoadBalancerInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub dns_name: Option<String>,
    pub state: Option<String>,
    pub protocol: Option<String>,
    pub instance_port: Option<i32>,
    pub instance_health_summary: Vec<LightsailInstanceHealthInfo>,
    pub created_at: Option<aws_smithy_types::DateTime>,
}

#[derive(Debug)]
pub struct LightsailStaticIpInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub ip_address: Option<String>,
    pub attached_to: Option<String>,
    pub is_attached: bool,
}

pub struct LightsailClient {
    inner: aws_sdk_lightsail::Client,
}

impl LightsailClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_lightsail::Client::new(config),
        }
    }

    /// Lists Lightsail instances, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `GetInstances` has
    /// no `max_results`-equivalent input field at all (verified against
    /// pinned `aws-sdk-lightsail` 1.114.0's `_get_instances_input.rs` — only
    /// `page_token`) — same caveat class as `codedeploy.rs::list_applications`/
    /// `polly.rs::describe_voices`: `limit` can only be enforced via
    /// client-side `apply_limit` truncation, so when that trips mid-page the
    /// returned `next_token` is still AWS's *next*-page token, permanently
    /// skipping whatever was truncated off the current page.
    pub async fn get_instances(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LightsailInstanceInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut page_token = next_token;

        loop {
            let mut req = self.inner.get_instances();
            if let Some(ref tok) = page_token {
                req = req.page_token(tok);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for inst in output.instances() {
                items.push(LightsailInstanceInfo {
                    name: inst.name().map(|s| s.to_string()),
                    arn: inst.arn().map(|s| s.to_string()),
                    blueprint_id: inst.blueprint_id().map(|s| s.to_string()),
                    bundle_id: inst.bundle_id().map(|s| s.to_string()),
                    state: inst.state().and_then(|s| s.name()).map(|s| s.to_string()),
                    public_ip_address: inst.public_ip_address().map(|s| s.to_string()),
                    private_ip_address: inst.private_ip_address().map(|s| s.to_string()),
                    location: inst
                        .location()
                        .and_then(|l| l.availability_zone())
                        .map(|s| s.to_string()),
                    created_at: inst.created_at().cloned(),
                    tags: inst
                        .tags()
                        .iter()
                        .map(|t| {
                            (
                                t.key().unwrap_or_default().to_string(),
                                t.value().unwrap_or_default().to_string(),
                            )
                        })
                        .collect(),
                });
            }
            page_token = match output.next_page_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            if apply_limit(&mut items, limit) || page_token.is_none() {
                break;
            }
        }

        Ok((items, page_token))
    }

    /// Lists Lightsail relational databases, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `GetRelationalDatabases` has no `max_results`-equivalent input field
    /// (same caveat class as `get_instances` above).
    pub async fn get_relational_databases(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LightsailDatabaseInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut page_token = next_token;

        loop {
            let mut req = self.inner.get_relational_databases();
            if let Some(ref tok) = page_token {
                req = req.page_token(tok);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for db in output.relational_databases() {
                items.push(LightsailDatabaseInfo {
                    name: db.name().map(|s| s.to_string()),
                    arn: db.arn().map(|s| s.to_string()),
                    engine: db.engine().map(|s| s.to_string()),
                    engine_version: db.engine_version().map(|s| s.to_string()),
                    state: db.state().map(|s| s.to_string()),
                    master_username: db.master_username().map(|s| s.to_string()),
                    master_endpoint: db.master_endpoint().map(|ep| LightsailEndpointInfo {
                        port: ep.port().unwrap_or(0),
                        address: ep.address().unwrap_or_default().to_string(),
                    }),
                    created_at: db.created_at().cloned(),
                    tags: db
                        .tags()
                        .iter()
                        .map(|t| {
                            (
                                t.key().unwrap_or_default().to_string(),
                                t.value().unwrap_or_default().to_string(),
                            )
                        })
                        .collect(),
                });
            }
            page_token = match output.next_page_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            if apply_limit(&mut items, limit) || page_token.is_none() {
                break;
            }
        }

        Ok((items, page_token))
    }

    /// Lists Lightsail load balancers, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `GetLoadBalancers`
    /// has no `max_results`-equivalent input field (same caveat class as
    /// `get_instances` above).
    pub async fn get_load_balancers(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LightsailLoadBalancerInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut page_token = next_token;

        loop {
            let mut req = self.inner.get_load_balancers();
            if let Some(ref tok) = page_token {
                req = req.page_token(tok);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for lb in output.load_balancers() {
                items.push(LightsailLoadBalancerInfo {
                    name: lb.name().map(|s| s.to_string()),
                    arn: lb.arn().map(|s| s.to_string()),
                    dns_name: lb.dns_name().map(|s| s.to_string()),
                    state: lb.state().map(|s| s.as_str().to_string()),
                    protocol: lb.protocol().map(|s| s.as_str().to_string()),
                    instance_port: lb.instance_port(),
                    instance_health_summary: lb
                        .instance_health_summary()
                        .iter()
                        .map(|h| LightsailInstanceHealthInfo {
                            instance_name: h.instance_name().map(|s| s.to_string()),
                            instance_health: h.instance_health().map(|s| s.as_str().to_string()),
                        })
                        .collect(),
                    created_at: lb.created_at().cloned(),
                });
            }
            page_token = match output.next_page_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            if apply_limit(&mut items, limit) || page_token.is_none() {
                break;
            }
        }

        Ok((items, page_token))
    }

    /// Lists Lightsail static IPs, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `GetStaticIps` has
    /// no `max_results`-equivalent input field (same caveat class as
    /// `get_instances` above).
    pub async fn get_static_ips(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<LightsailStaticIpInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut page_token = next_token;

        loop {
            let mut req = self.inner.get_static_ips();
            if let Some(ref tok) = page_token {
                req = req.page_token(tok);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for ip in output.static_ips() {
                items.push(LightsailStaticIpInfo {
                    name: ip.name().map(|s| s.to_string()),
                    arn: ip.arn().map(|s| s.to_string()),
                    ip_address: ip.ip_address().map(|s| s.to_string()),
                    attached_to: ip.attached_to().map(|s| s.to_string()),
                    is_attached: ip.is_attached().unwrap_or(false),
                });
            }
            page_token = match output.next_page_token() {
                Some(tok) if !tok.is_empty() => Some(tok.to_string()),
                _ => None,
            };

            if apply_limit(&mut items, limit) || page_token.is_none() {
                break;
            }
        }

        Ok((items, page_token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use aws_smithy_types::DateTime;

    const ENDPOINT: &str = "https://lightsail.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn get_instances_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"instances":[{"name":"my-instance","arn":"arn:aws:lightsail:us-east-1:123456789012:Instance/instance-id","blueprintId":"amazon_linux_2","bundleId":"nano_2_0","state":{"code":16,"name":"running"},"publicIpAddress":"1.2.3.4","privateIpAddress":"10.0.0.5","location":{"availabilityZone":"us-east-1a","regionName":"us-east-1"},"createdAt":1700000000,"tags":[{"key":"Env","value":"prod"}]}]}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_instances(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let inst = &items[0];
        assert_eq!(inst.name, Some("my-instance".to_string()));
        assert_eq!(
            inst.arn,
            Some("arn:aws:lightsail:us-east-1:123456789012:Instance/instance-id".to_string())
        );
        assert_eq!(inst.blueprint_id, Some("amazon_linux_2".to_string()));
        assert_eq!(inst.bundle_id, Some("nano_2_0".to_string()));
        assert_eq!(inst.state, Some("running".to_string()));
        assert_eq!(inst.public_ip_address, Some("1.2.3.4".to_string()));
        assert_eq!(inst.private_ip_address, Some("10.0.0.5".to_string()));
        assert_eq!(inst.location, Some("us-east-1a".to_string()));
        assert_eq!(inst.created_at, Some(DateTime::from_secs(1700000000)));
        assert_eq!(inst.tags, vec![("Env".to_string(), "prod".to_string())]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_instances_maps_minimal_instance_with_no_optional_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"instances":[{}]}"#),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.get_instances(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let inst = &items[0];
        assert_eq!(inst.name, None);
        assert_eq!(inst.arn, None);
        assert_eq!(inst.blueprint_id, None);
        assert_eq!(inst.bundle_id, None);
        assert_eq!(inst.state, None);
        assert_eq!(inst.public_ip_address, None);
        assert_eq!(inst.private_ip_address, None);
        assert_eq!(inst.location, None);
        assert_eq!(inst.created_at, None);
        assert_eq!(inst.tags, Vec::new());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_instances_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"pageToken":"cursor-a"}"#),
            json_response(200, r#"{"instances":[{"name":"instance-2"}]}"#),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_instances(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, Some("instance-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_instances_stops_at_limit_and_returns_resume_token() {
        // `GetInstances` has no `MaxResults`-equivalent input field at all, so
        // `limit` is enforced purely client-side (durable gotcha 13's
        // client-truncate category) — the canned response must return more
        // than `limit` items to prove truncation actually happens.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"instances":[{"name":"instance-1"},{"name":"instance-2"},{"name":"instance-3"}],"nextPageToken":"page2"}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_instances(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, Some("instance-1".to_string()));
        assert_eq!(items[1].name, Some("instance-2".to_string()));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_instances_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"instances":[{"name":"instance-1"}],"nextPageToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pageToken":"p2"}"#),
                json_response(200, r#"{"instances":[{"name":"instance-2"}]}"#),
            ),
        ]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_instances(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, Some("instance-1".to_string()));
        assert_eq!(items[1].name, Some("instance-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidInputException", "bad input"),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let err = client.get_instances(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "bad input");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_relational_databases_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"relationalDatabases":[{"name":"my-db","arn":"arn:aws:lightsail:us-east-1:123456789012:RelationalDatabase/db-id","engine":"mysql","engineVersion":"8.0","state":"available","masterUsername":"dbadmin","masterEndpoint":{"port":3306,"address":"my-db.abcdefg.us-east-1.rds.amazonaws.com"},"createdAt":1700000000,"tags":[{"key":"Env","value":"prod"}]}]}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_relational_databases(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let db = &items[0];
        assert_eq!(db.name, Some("my-db".to_string()));
        assert_eq!(
            db.arn,
            Some("arn:aws:lightsail:us-east-1:123456789012:RelationalDatabase/db-id".to_string())
        );
        assert_eq!(db.engine, Some("mysql".to_string()));
        assert_eq!(db.engine_version, Some("8.0".to_string()));
        assert_eq!(db.state, Some("available".to_string()));
        assert_eq!(db.master_username, Some("dbadmin".to_string()));
        let endpoint = db.master_endpoint.as_ref().unwrap();
        assert_eq!(endpoint.port, 3306);
        assert_eq!(
            endpoint.address,
            "my-db.abcdefg.us-east-1.rds.amazonaws.com"
        );
        assert_eq!(db.created_at, Some(DateTime::from_secs(1700000000)));
        assert_eq!(db.tags, vec![("Env".to_string(), "prod".to_string())]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_relational_databases_maps_minimal_database_with_no_optional_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"relationalDatabases":[{}]}"#),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.get_relational_databases(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let db = &items[0];
        assert_eq!(db.name, None);
        assert_eq!(db.arn, None);
        assert_eq!(db.engine, None);
        assert_eq!(db.engine_version, None);
        assert_eq!(db.state, None);
        assert_eq!(db.master_username, None);
        assert!(db.master_endpoint.is_none());
        assert_eq!(db.created_at, None);
        assert_eq!(db.tags, Vec::new());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_relational_databases_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"pageToken":"cursor-a"}"#),
            json_response(200, r#"{"relationalDatabases":[{"name":"db-2"}]}"#),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_relational_databases(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, Some("db-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_relational_databases_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"relationalDatabases":[{"name":"db-1"},{"name":"db-2"},{"name":"db-3"}],"nextPageToken":"page2"}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_relational_databases(Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, Some("db-1".to_string()));
        assert_eq!(items[1].name, Some("db-2".to_string()));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_relational_databases_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"relationalDatabases":[{"name":"db-1"}],"nextPageToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pageToken":"p2"}"#),
                json_response(200, r#"{"relationalDatabases":[{"name":"db-2"}]}"#),
            ),
        ]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_relational_databases(Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, Some("db-1".to_string()));
        assert_eq!(items[1].name, Some("db-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_relational_databases_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidInputException", "bad input"),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_relational_databases(None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "bad input");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_load_balancers_lists_all_when_no_limit() {
        // `LoadBalancerState`'s wire values are lowercase snake_case
        // ("active_impaired") while `LoadBalancerProtocol`'s are uppercase
        // ("HTTP_HTTPS") in the same response object — durable gotcha 15
        // recurrence, verified against the pinned SDK's
        // `types/_load_balancer_state.rs`/`_load_balancer_protocol.rs`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"loadBalancers":[{"name":"my-lb","arn":"arn:aws:lightsail:us-east-1:123456789012:LoadBalancer/lb-id","dnsName":"my-lb.abcdefg.us-east-1.elb.amazonaws.com","state":"active_impaired","protocol":"HTTP_HTTPS","instancePort":80,"instanceHealthSummary":[{"instanceName":"my-instance","instanceHealth":"healthy"}],"createdAt":1700000000}]}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_load_balancers(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let lb = &items[0];
        assert_eq!(lb.name, Some("my-lb".to_string()));
        assert_eq!(
            lb.arn,
            Some("arn:aws:lightsail:us-east-1:123456789012:LoadBalancer/lb-id".to_string())
        );
        assert_eq!(
            lb.dns_name,
            Some("my-lb.abcdefg.us-east-1.elb.amazonaws.com".to_string())
        );
        assert_eq!(lb.state, Some("active_impaired".to_string()));
        assert_eq!(lb.protocol, Some("HTTP_HTTPS".to_string()));
        assert_eq!(lb.instance_port, Some(80));
        assert_eq!(lb.instance_health_summary.len(), 1);
        assert_eq!(
            lb.instance_health_summary[0].instance_name,
            Some("my-instance".to_string())
        );
        assert_eq!(
            lb.instance_health_summary[0].instance_health,
            Some("healthy".to_string())
        );
        assert_eq!(lb.created_at, Some(DateTime::from_secs(1700000000)));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_load_balancers_maps_minimal_load_balancer_with_no_optional_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"loadBalancers":[{}]}"#),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.get_load_balancers(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let lb = &items[0];
        assert_eq!(lb.name, None);
        assert_eq!(lb.arn, None);
        assert_eq!(lb.dns_name, None);
        assert_eq!(lb.state, None);
        assert_eq!(lb.protocol, None);
        assert_eq!(lb.instance_port, None);
        assert!(lb.instance_health_summary.is_empty());
        assert_eq!(lb.created_at, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_load_balancers_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"pageToken":"cursor-a"}"#),
            json_response(200, r#"{"loadBalancers":[{"name":"lb-2"}]}"#),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_load_balancers(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, Some("lb-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_load_balancers_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"loadBalancers":[{"name":"lb-1"},{"name":"lb-2"},{"name":"lb-3"}],"nextPageToken":"page2"}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_load_balancers(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, Some("lb-1".to_string()));
        assert_eq!(items[1].name, Some("lb-2".to_string()));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_load_balancers_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"loadBalancers":[{"name":"lb-1"}],"nextPageToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pageToken":"p2"}"#),
                json_response(200, r#"{"loadBalancers":[{"name":"lb-2"}]}"#),
            ),
        ]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_load_balancers(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, Some("lb-1".to_string()));
        assert_eq!(items[1].name, Some("lb-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_load_balancers_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidInputException", "bad input"),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let err = client.get_load_balancers(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "bad input");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_static_ips_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"staticIps":[{"name":"my-static-ip","arn":"arn:aws:lightsail:us-east-1:123456789012:StaticIp/ip-id","ipAddress":"1.2.3.4","attachedTo":"my-instance","isAttached":true}]}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_static_ips(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let ip = &items[0];
        assert_eq!(ip.name, Some("my-static-ip".to_string()));
        assert_eq!(
            ip.arn,
            Some("arn:aws:lightsail:us-east-1:123456789012:StaticIp/ip-id".to_string())
        );
        assert_eq!(ip.ip_address, Some("1.2.3.4".to_string()));
        assert_eq!(ip.attached_to, Some("my-instance".to_string()));
        assert!(ip.is_attached);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_static_ips_maps_minimal_static_ip_with_no_optional_fields() {
        // `is_attached` has no `_correct_errors`-driven default-fill (gotcha
        // 16); the wrapper's own `.unwrap_or(false)` is what turns an absent
        // `isAttached` into `false` here, not an SDK-side default.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"staticIps":[{}]}"#),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.get_static_ips(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let ip = &items[0];
        assert_eq!(ip.name, None);
        assert_eq!(ip.arn, None);
        assert_eq!(ip.ip_address, None);
        assert_eq!(ip.attached_to, None);
        assert!(!ip.is_attached);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_static_ips_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"pageToken":"cursor-a"}"#),
            json_response(200, r#"{"staticIps":[{"name":"ip-2"}]}"#),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_static_ips(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, Some("ip-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_static_ips_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"staticIps":[{"name":"ip-1"},{"name":"ip-2"},{"name":"ip-3"}],"nextPageToken":"page2"}"#,
            ),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_static_ips(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, Some("ip-1".to_string()));
        assert_eq!(items[1].name, Some("ip-2".to_string()));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_static_ips_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"staticIps":[{"name":"ip-1"}],"nextPageToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"pageToken":"p2"}"#),
                json_response(200, r#"{"staticIps":[{"name":"ip-2"}]}"#),
            ),
        ]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_static_ips(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, Some("ip-1".to_string()));
        assert_eq!(items[1].name, Some("ip-2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_static_ips_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidInputException", "bad input"),
        )]);
        let client = LightsailClient::new(&sdk_config(http_client.clone()));

        let err = client.get_static_ips(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "bad input");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
