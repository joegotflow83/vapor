use aws_config::SdkConfig;
use aws_sdk_databasemigration::types::Filter;

use crate::error::VaporError;

#[derive(Debug)]
pub struct DmsReplicationInstanceInfo {
    pub replication_instance_identifier: Option<String>,
    pub replication_instance_arn: Option<String>,
    pub replication_instance_class: Option<String>,
    pub replication_instance_status: Option<String>,
    pub allocated_storage: Option<i32>,
    pub publicly_accessible: Option<bool>,
    pub engine_version: Option<String>,
    pub vpc_security_groups: Vec<String>,
    pub replication_subnet_group_id: Option<String>,
    pub availability_zone: Option<String>,
    pub multi_az: Option<bool>,
}

#[derive(Debug)]
pub struct DmsEndpointInfo {
    pub endpoint_identifier: Option<String>,
    pub endpoint_arn: Option<String>,
    pub endpoint_type: Option<String>,
    pub engine_name: Option<String>,
    pub status: Option<String>,
    pub database_name: Option<String>,
    pub server_name: Option<String>,
    pub port: Option<i32>,
    pub ssl_mode: Option<String>,
}

#[derive(Debug)]
pub struct DmsReplicationTaskInfo {
    pub replication_task_identifier: Option<String>,
    pub replication_task_arn: Option<String>,
    pub status: Option<String>,
    pub migration_type: Option<String>,
    pub source_endpoint_arn: Option<String>,
    pub target_endpoint_arn: Option<String>,
    pub replication_instance_arn: Option<String>,
    pub replication_task_creation_date: Option<aws_smithy_types::DateTime>,
    pub replication_task_start_date: Option<aws_smithy_types::DateTime>,
}

pub struct DmsClient {
    inner: aws_sdk_databasemigration::Client,
}

impl DmsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_databasemigration::Client::new(config),
        }
    }

    /// Lists replication instances, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `DescribeReplicationInstances`
    /// has both `max_records` and `marker` (verified against pinned
    /// `aws-sdk-databasemigration` 1.116.0's
    /// `operation/describe_replication_instances/_describe_replication_instances_input.rs`),
    /// but `max_records` is constrained to `[20, 100]` (same class as
    /// `documentdb.rs::describe_db_clusters`) — a `limit` below 20 can't be
    /// requested exactly, so it's clamped server-side and truncated
    /// client-side; when that happens the returned marker points past the
    /// whole fetched page, not just the truncated prefix, so resuming skips
    /// whatever was truncated off (same caveat as cost_explorer.rs/
    /// documentdb.rs/neptune.rs).
    pub async fn describe_replication_instances(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DmsReplicationInstanceInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_replication_instances();
            if let Some(l) = limit {
                req = req.max_records((l - items.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for ri in output.replication_instances() {
                let vpc_security_groups = ri
                    .vpc_security_groups()
                    .iter()
                    .filter_map(|sg| sg.vpc_security_group_id().map(|s| s.to_string()))
                    .collect();
                items.push(DmsReplicationInstanceInfo {
                    replication_instance_identifier: ri
                        .replication_instance_identifier()
                        .map(|s| s.to_string()),
                    replication_instance_arn: ri.replication_instance_arn().map(|s| s.to_string()),
                    replication_instance_class: ri
                        .replication_instance_class()
                        .map(|s| s.to_string()),
                    replication_instance_status: ri
                        .replication_instance_status()
                        .map(|s| s.to_string()),
                    allocated_storage: Some(ri.allocated_storage()),
                    publicly_accessible: Some(ri.publicly_accessible()),
                    engine_version: ri.engine_version().map(|s| s.to_string()),
                    vpc_security_groups,
                    replication_subnet_group_id: ri
                        .replication_subnet_group()
                        .and_then(|g| g.replication_subnet_group_identifier())
                        .map(|s| s.to_string()),
                    availability_zone: ri.availability_zone().map(|s| s.to_string()),
                    multi_az: Some(ri.multi_az()),
                });
            }

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            items.truncate(l.max(0) as usize);
        }

        Ok((items, marker))
    }

    /// Lists endpoints, optionally filtered by `endpoint_type`, capped at
    /// `limit` results (default unlimited), and resumed from `next_token`.
    /// Same `max_records` `[20, 100]`-clamp caveat as
    /// `describe_replication_instances` above.
    pub async fn describe_endpoints(
        &self,
        endpoint_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DmsEndpointInfo>, Option<String>), VaporError> {
        let filter = match endpoint_type {
            Some(ref et) => Some(
                Filter::builder()
                    .name("endpoint-type")
                    .values(et.to_lowercase())
                    .build()
                    .map_err(|e| VaporError::AwsSdk {
                        code: None,
                        message: e.to_string(),
                    })?,
            ),
            None => None,
        };

        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_endpoints();
            if let Some(ref f) = filter {
                req = req.filters(f.clone());
            }
            if let Some(l) = limit {
                req = req.max_records((l - items.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for ep in output.endpoints() {
                items.push(DmsEndpointInfo {
                    endpoint_identifier: ep.endpoint_identifier().map(|s| s.to_string()),
                    endpoint_arn: ep.endpoint_arn().map(|s| s.to_string()),
                    endpoint_type: ep.endpoint_type().map(|t| t.as_str().to_string()),
                    engine_name: ep.engine_name().map(|s| s.to_string()),
                    status: ep.status().map(|s| s.to_string()),
                    database_name: ep.database_name().map(|s| s.to_string()),
                    server_name: ep.server_name().map(|s| s.to_string()),
                    port: ep.port(),
                    ssl_mode: ep.ssl_mode().map(|m| m.as_str().to_string()),
                });
            }

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            items.truncate(l.max(0) as usize);
        }

        Ok((items, marker))
    }

    /// Lists replication tasks, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. Same `max_records`
    /// `[20, 100]`-clamp caveat as `describe_replication_instances` above.
    pub async fn describe_replication_tasks(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DmsReplicationTaskInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_replication_tasks();
            if let Some(l) = limit {
                req = req.max_records((l - items.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for task in output.replication_tasks() {
                items.push(DmsReplicationTaskInfo {
                    replication_task_identifier: task
                        .replication_task_identifier()
                        .map(|s| s.to_string()),
                    replication_task_arn: task.replication_task_arn().map(|s| s.to_string()),
                    status: task.status().map(|s| s.to_string()),
                    migration_type: task.migration_type().map(|t| t.as_str().to_string()),
                    source_endpoint_arn: task.source_endpoint_arn().map(|s| s.to_string()),
                    target_endpoint_arn: task.target_endpoint_arn().map(|s| s.to_string()),
                    replication_instance_arn: task
                        .replication_instance_arn()
                        .map(|s| s.to_string()),
                    replication_task_creation_date: task.replication_task_creation_date().cloned(),
                    replication_task_start_date: task.replication_task_start_date().cloned(),
                });
            }

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            items.truncate(l.max(0) as usize);
        }

        Ok((items, marker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://dms.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_replication_instances_happy_path_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ReplicationInstances":[{"ReplicationInstanceIdentifier":"my-repl","ReplicationInstanceArn":"arn:aws:dms:us-east-1:1:rep:abc","ReplicationInstanceClass":"dms.t3.medium","ReplicationInstanceStatus":"available","AllocatedStorage":50,"PubliclyAccessible":true,"EngineVersion":"3.5.2","VpcSecurityGroups":[{"VpcSecurityGroupId":"sg-1","Status":"active"}],"ReplicationSubnetGroup":{"ReplicationSubnetGroupIdentifier":"my-subnet-group"},"AvailabilityZone":"us-east-1a","MultiAZ":false}]}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client
            .describe_replication_instances(None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].replication_instance_identifier,
            Some("my-repl".to_string())
        );
        assert_eq!(items[0].allocated_storage, Some(50));
        assert_eq!(items[0].publicly_accessible, Some(true));
        assert_eq!(items[0].vpc_security_groups, vec!["sg-1".to_string()]);
        assert_eq!(
            items[0].replication_subnet_group_id,
            Some("my-subnet-group".to_string())
        );
        assert_eq!(items[0].multi_az, Some(false));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_instances_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Marker":"cursor-a"}"#),
            json_response(200, r#"{"ReplicationInstances":[]}"#),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client
            .describe_replication_instances(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_instances_stops_at_limit_below_min_max_records_clamp() {
        // `limit` of 2 is below the [20,100] MaxRecords floor, so the request
        // still asks for 20 records; the client truncates locally and keeps
        // AWS's marker even though it points past the whole fetched page.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxRecords":20}"#),
            json_response(
                200,
                r#"{"ReplicationInstances":[
                    {"ReplicationInstanceIdentifier":"a"},
                    {"ReplicationInstanceIdentifier":"b"},
                    {"ReplicationInstanceIdentifier":"c"}
                ],"Marker":"page2"}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client
            .describe_replication_instances(Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_instances_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxRecords":100}"#),
                json_response(
                    200,
                    r#"{"ReplicationInstances":[{"ReplicationInstanceIdentifier":"a"}],"Marker":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxRecords":99,"Marker":"p2"}"#),
                json_response(
                    200,
                    r#"{"ReplicationInstances":[{"ReplicationInstanceIdentifier":"b"}]}"#,
                ),
            ),
        ]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client
            .describe_replication_instances(Some(100), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("ResourceNotFoundFault", "no such replication instance"),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_replication_instances(None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundFault".to_string()));
                assert_eq!(message, "no such replication instance");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_endpoints_happy_path_with_type_filter_lowercased() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"Filters":[{"Name":"endpoint-type","Values":["source"]}]}"#,
            ),
            json_response(
                200,
                r#"{"Endpoints":[{"EndpointIdentifier":"my-src","EndpointArn":"arn:aws:dms:us-east-1:1:endpoint:abc","EndpointType":"source","EngineName":"postgres","Status":"active","DatabaseName":"mydb","ServerName":"db.example.com","Port":5432,"SslMode":"require"}]}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client
            .describe_endpoints(Some("SOURCE".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].endpoint_identifier, Some("my-src".to_string()));
        assert_eq!(items[0].endpoint_type, Some("source".to_string()));
        assert_eq!(items[0].engine_name, Some("postgres".to_string()));
        assert_eq!(items[0].port, Some(5432));
        assert_eq!(items[0].ssl_mode, Some("require".to_string()));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_endpoints_happy_path_without_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(200, r#"{"Endpoints":[]}"#),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client.describe_endpoints(None, None, None).await.unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_endpoints_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxRecords":20}"#),
            json_response(
                200,
                r#"{"Endpoints":[
                    {"EndpointIdentifier":"a"},
                    {"EndpointIdentifier":"b"},
                    {"EndpointIdentifier":"c"}
                ],"Marker":"page2"}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client
            .describe_endpoints(None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_endpoints_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("ResourceNotFoundFault", "no such endpoint"),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_endpoints(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundFault".to_string()));
                assert_eq!(message, "no such endpoint");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_tasks_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ReplicationTasks":[{"ReplicationTaskIdentifier":"my-task","ReplicationTaskArn":"arn:aws:dms:us-east-1:1:task:abc","Status":"running","MigrationType":"full-load-and-cdc","SourceEndpointArn":"arn:src","TargetEndpointArn":"arn:tgt","ReplicationInstanceArn":"arn:inst","ReplicationTaskCreationDate":1750000000,"ReplicationTaskStartDate":1750000100}]}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client.describe_replication_tasks(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].replication_task_identifier,
            Some("my-task".to_string())
        );
        assert_eq!(items[0].status, Some("running".to_string()));
        assert_eq!(
            items[0].migration_type,
            Some("full-load-and-cdc".to_string())
        );
        assert_eq!(items[0].source_endpoint_arn, Some("arn:src".to_string()));
        assert_eq!(items[0].target_endpoint_arn, Some("arn:tgt".to_string()));
        assert!(items[0].replication_task_creation_date.is_some());
        assert!(items[0].replication_task_start_date.is_some());
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_tasks_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxRecords":20}"#),
            json_response(
                200,
                r#"{"ReplicationTasks":[
                    {"ReplicationTaskIdentifier":"a"},
                    {"ReplicationTaskIdentifier":"b"},
                    {"ReplicationTaskIdentifier":"c"}
                ],"Marker":"page2"}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client
            .describe_replication_tasks(Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_tasks_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Marker":"cursor-b"}"#),
            json_response(200, r#"{"ReplicationTasks":[]}"#),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let (items, marker) = client
            .describe_replication_tasks(None, Some("cursor-b".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_tasks_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("ResourceNotFoundFault", "no such replication task"),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_replication_tasks(None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundFault".to_string()));
                assert_eq!(message, "no such replication task");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
