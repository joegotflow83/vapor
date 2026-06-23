use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct AthenaWorkgroup {
    pub name: String,
    pub state: Option<String>,
    pub description: Option<String>,
    pub creation_time: Option<String>,
    pub output_location: Option<String>,
    pub enforce_workgroup_configuration: bool,
    pub bytes_scanned_cutoff: Option<i64>,
    pub engine_version: Option<String>,
}

impl AthenaWorkgroup {
    pub fn from_sdk(wg: &aws_sdk_athena::types::WorkGroup) -> Self {
        let config = wg.configuration();
        Self {
            name: wg.name().to_string(),
            state: wg.state().map(|s| s.as_str().to_string()),
            description: wg.description().map(|s| s.to_string()),
            creation_time: wg.creation_time().map(|t| t.to_string()),
            output_location: config
                .and_then(|c| c.result_configuration())
                .and_then(|r| r.output_location())
                .map(|s| s.to_string()),
            enforce_workgroup_configuration: config
                .and_then(|c| c.enforce_work_group_configuration())
                .unwrap_or(false),
            bytes_scanned_cutoff: config
                .and_then(|c| c.bytes_scanned_cutoff_per_query()),
            engine_version: config
                .and_then(|c| c.engine_version())
                .and_then(|e| e.effective_engine_version())
                .map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AthenaNamedQuery {
    pub id: String,
    pub name: String,
    pub database: Option<String>,
    pub workgroup: Option<String>,
    pub description: Option<String>,
    pub query_string: String,
}

impl From<aws_sdk_athena::types::NamedQuery> for AthenaNamedQuery {
    fn from(nq: aws_sdk_athena::types::NamedQuery) -> Self {
        Self {
            id: nq.named_query_id().unwrap_or_default().to_string(),
            name: nq.name().to_string(),
            database: Some(nq.database().to_string()),
            workgroup: nq.work_group().map(|s| s.to_string()),
            description: nq.description().map(|s| s.to_string()),
            query_string: nq.query_string().to_string(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AthenaQueryExecution {
    pub id: String,
    pub query: Option<String>,
    pub database: Option<String>,
    pub workgroup: Option<String>,
    pub state: Option<String>,
    pub submission_time: Option<String>,
    pub completion_time: Option<String>,
    pub data_scanned_bytes: Option<i64>,
    pub execution_time_ms: Option<i64>,
    pub output_location: Option<String>,
}

impl From<aws_sdk_athena::types::QueryExecution> for AthenaQueryExecution {
    fn from(qe: aws_sdk_athena::types::QueryExecution) -> Self {
        let status = qe.status();
        let stats = qe.statistics();
        Self {
            id: qe.query_execution_id().unwrap_or_default().to_string(),
            query: qe.query().map(|s| s.to_string()),
            database: qe.query_execution_context().and_then(|c| c.database()).map(|s| s.to_string()),
            workgroup: qe.work_group().map(|s| s.to_string()),
            state: status.and_then(|s| s.state()).map(|s| s.as_str().to_string()),
            submission_time: status.and_then(|s| s.submission_date_time()).map(|t| t.to_string()),
            completion_time: status.and_then(|s| s.completion_date_time()).map(|t| t.to_string()),
            data_scanned_bytes: stats.and_then(|s| s.data_scanned_in_bytes()),
            execution_time_ms: stats.and_then(|s| s.engine_execution_time_in_millis()),
            output_location: qe.result_configuration().and_then(|r| r.output_location()).map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workgroup_from_sdk_minimal() {
        let wg = aws_sdk_athena::types::WorkGroup::builder().name("").build().unwrap();
        let result = AthenaWorkgroup::from_sdk(&wg);
        assert_eq!(result.name, "");
        assert!(result.state.is_none());
        assert!(result.description.is_none());
        assert!(!result.enforce_workgroup_configuration);
        assert!(result.bytes_scanned_cutoff.is_none());
        assert!(result.engine_version.is_none());
    }

    #[test]
    fn test_workgroup_from_sdk_full() {
        let config = aws_sdk_athena::types::WorkGroupConfiguration::builder()
            .enforce_work_group_configuration(true)
            .bytes_scanned_cutoff_per_query(1_000_000)
            .result_configuration(
                aws_sdk_athena::types::ResultConfiguration::builder()
                    .output_location("s3://my-bucket/output/")
                    .build(),
            )
            .engine_version(
                aws_sdk_athena::types::EngineVersion::builder()
                    .effective_engine_version("Athena engine version 3")
                    .build(),
            )
            .build();
        let wg = aws_sdk_athena::types::WorkGroup::builder()
            .name("primary")
            .state(aws_sdk_athena::types::WorkGroupState::Enabled)
            .description("Primary workgroup")
            .configuration(config)
            .build()
            .unwrap();
        let result = AthenaWorkgroup::from_sdk(&wg);
        assert_eq!(result.name, "primary");
        assert_eq!(result.state, Some("ENABLED".to_string()));
        assert_eq!(result.description, Some("Primary workgroup".to_string()));
        assert!(result.enforce_workgroup_configuration);
        assert_eq!(result.bytes_scanned_cutoff, Some(1_000_000));
        assert_eq!(result.output_location, Some("s3://my-bucket/output/".to_string()));
        assert_eq!(result.engine_version, Some("Athena engine version 3".to_string()));
    }

    #[test]
    fn test_named_query_from_sdk() {
        let nq = aws_sdk_athena::types::NamedQuery::builder()
            .named_query_id("nq-123")
            .name("My Query")
            .database("default")
            .work_group("primary")
            .description("A test query")
            .query_string("SELECT 1")
            .build()
            .unwrap();
        let result = AthenaNamedQuery::from(nq);
        assert_eq!(result.id, "nq-123");
        assert_eq!(result.name, "My Query");
        assert_eq!(result.database, Some("default".to_string()));
        assert_eq!(result.workgroup, Some("primary".to_string()));
        assert_eq!(result.description, Some("A test query".to_string()));
        assert_eq!(result.query_string, "SELECT 1");
    }

    #[test]
    fn test_query_execution_from_sdk_minimal() {
        let qe = aws_sdk_athena::types::QueryExecution::builder().build();
        let result = AthenaQueryExecution::from(qe);
        assert_eq!(result.id, "");
        assert!(result.query.is_none());
        assert!(result.state.is_none());
        assert!(result.data_scanned_bytes.is_none());
    }

    #[test]
    fn test_query_execution_from_sdk_full() {
        let status = aws_sdk_athena::types::QueryExecutionStatus::builder()
            .state(aws_sdk_athena::types::QueryExecutionState::Succeeded)
            .build();
        let stats = aws_sdk_athena::types::QueryExecutionStatistics::builder()
            .data_scanned_in_bytes(1024)
            .engine_execution_time_in_millis(500)
            .build();
        let ctx = aws_sdk_athena::types::QueryExecutionContext::builder()
            .database("mydb")
            .build();
        let result_config = aws_sdk_athena::types::ResultConfiguration::builder()
            .output_location("s3://results/query-123.csv")
            .build();
        let qe = aws_sdk_athena::types::QueryExecution::builder()
            .query_execution_id("qe-456")
            .query("SELECT * FROM t")
            .work_group("primary")
            .status(status)
            .statistics(stats)
            .query_execution_context(ctx)
            .result_configuration(result_config)
            .build();
        let result = AthenaQueryExecution::from(qe);
        assert_eq!(result.id, "qe-456");
        assert_eq!(result.query, Some("SELECT * FROM t".to_string()));
        assert_eq!(result.database, Some("mydb".to_string()));
        assert_eq!(result.workgroup, Some("primary".to_string()));
        assert_eq!(result.state, Some("SUCCEEDED".to_string()));
        assert_eq!(result.data_scanned_bytes, Some(1024));
        assert_eq!(result.execution_time_ms, Some(500));
        assert_eq!(result.output_location, Some("s3://results/query-123.csv".to_string()));
    }
}
