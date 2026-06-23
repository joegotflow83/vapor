use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct GlueDatabase {
    pub name: String,
    pub catalog_id: Option<String>,
    pub description: Option<String>,
    pub location_uri: Option<String>,
    pub create_time: Option<String>,
}

impl From<&aws_sdk_glue::types::Database> for GlueDatabase {
    fn from(db: &aws_sdk_glue::types::Database) -> Self {
        Self {
            name: db.name().to_string(),
            catalog_id: db.catalog_id().map(|s| s.to_string()),
            description: db.description().map(|s| s.to_string()),
            location_uri: db.location_uri().map(|s| s.to_string()),
            create_time: db.create_time().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GlueTable {
    pub name: String,
    pub database_name: Option<String>,
    pub description: Option<String>,
    pub table_type: Option<String>,
    pub location: Option<String>,
    pub input_format: Option<String>,
    pub output_format: Option<String>,
    pub serde: Option<String>,
    pub columns: Vec<GlueColumn>,
    pub partition_keys: Vec<GlueColumn>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

impl From<&aws_sdk_glue::types::Table> for GlueTable {
    fn from(t: &aws_sdk_glue::types::Table) -> Self {
        let sd = t.storage_descriptor();
        Self {
            name: t.name().to_string(),
            database_name: t.database_name().map(|s| s.to_string()),
            description: t.description().map(|s| s.to_string()),
            table_type: t.table_type().map(|s| s.to_string()),
            location: sd.and_then(|s| s.location().map(|l| l.to_string())),
            input_format: sd.and_then(|s| s.input_format().map(|f| f.to_string())),
            output_format: sd.and_then(|s| s.output_format().map(|f| f.to_string())),
            serde: sd
                .and_then(|s| s.serde_info())
                .and_then(|si| si.serialization_library().map(|l| l.to_string())),
            columns: sd
                .map(|s| s.columns().iter().map(GlueColumn::from).collect())
                .unwrap_or_default(),
            partition_keys: t.partition_keys().iter().map(GlueColumn::from).collect(),
            create_time: t.create_time().map(|ts| ts.to_string()),
            update_time: t.update_time().map(|ts| ts.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GlueColumn {
    pub name: String,
    pub col_type: Option<String>,
    pub comment: Option<String>,
}

impl From<&aws_sdk_glue::types::Column> for GlueColumn {
    fn from(c: &aws_sdk_glue::types::Column) -> Self {
        Self {
            name: c.name().to_string(),
            col_type: c.r#type().map(|s| s.to_string()),
            comment: c.comment().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GlueCrawler {
    pub name: String,
    pub role: Option<String>,
    pub database_name: Option<String>,
    pub state: Option<String>,
    pub schedule: Option<String>,
    pub last_crawl_status: Option<String>,
    pub last_crawl_time: Option<String>,
    pub creation_time: Option<String>,
}

impl From<&aws_sdk_glue::types::Crawler> for GlueCrawler {
    fn from(c: &aws_sdk_glue::types::Crawler) -> Self {
        let last_crawl = c.last_crawl();
        Self {
            name: c.name().unwrap_or_default().to_string(),
            role: c.role().map(|s| s.to_string()),
            database_name: c.database_name().map(|s| s.to_string()),
            state: c.state().map(|s| s.as_str().to_string()),
            schedule: c.schedule().and_then(|s| s.schedule_expression().map(|e| e.to_string())),
            last_crawl_status: last_crawl.and_then(|l| l.status().map(|s| s.as_str().to_string())),
            last_crawl_time: last_crawl.and_then(|l| l.start_time().map(|t| t.to_string())),
            creation_time: c.creation_time().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GlueJob {
    pub name: String,
    pub role: Option<String>,
    pub command_name: Option<String>,
    pub max_capacity: Option<f64>,
    pub worker_type: Option<String>,
    pub number_of_workers: Option<i32>,
    pub glue_version: Option<String>,
    pub last_run_status: Option<String>,
    pub created_on: Option<String>,
    pub last_modified_on: Option<String>,
}

impl GlueJob {
    pub fn from_sdk(job: &aws_sdk_glue::types::Job, last_run_status: Option<String>) -> Self {
        Self {
            name: job.name().unwrap_or_default().to_string(),
            role: job.role().map(|s| s.to_string()),
            command_name: job.command().and_then(|c| c.name().map(|n| n.to_string())),
            max_capacity: job.max_capacity(),
            worker_type: job.worker_type().map(|w| w.as_str().to_string()),
            number_of_workers: job.number_of_workers(),
            glue_version: job.glue_version().map(|s| s.to_string()),
            last_run_status,
            created_on: job.created_on().map(|t| t.to_string()),
            last_modified_on: job.last_modified_on().map(|t| t.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glue_database_from_sdk() {
        let db = aws_sdk_glue::types::Database::builder()
            .name("my_db")
            .catalog_id("123456789012")
            .description("Test database")
            .location_uri("s3://bucket/path")
            .build()
            .unwrap();
        let result = GlueDatabase::from(&db);
        assert_eq!(result.name, "my_db");
        assert_eq!(result.catalog_id, Some("123456789012".to_string()));
        assert_eq!(result.description, Some("Test database".to_string()));
        assert_eq!(result.location_uri, Some("s3://bucket/path".to_string()));
    }

    #[test]
    fn test_glue_database_minimal() {
        let db = aws_sdk_glue::types::Database::builder()
            .name("minimal")
            .build()
            .unwrap();
        let result = GlueDatabase::from(&db);
        assert_eq!(result.name, "minimal");
        assert!(result.catalog_id.is_none());
        assert!(result.description.is_none());
        assert!(result.location_uri.is_none());
        assert!(result.create_time.is_none());
    }

    #[test]
    fn test_glue_column_from_sdk() {
        let col = aws_sdk_glue::types::Column::builder()
            .name("id")
            .r#type("bigint")
            .comment("Primary key")
            .build()
            .unwrap();
        let result = GlueColumn::from(&col);
        assert_eq!(result.name, "id");
        assert_eq!(result.col_type, Some("bigint".to_string()));
        assert_eq!(result.comment, Some("Primary key".to_string()));
    }

    #[test]
    fn test_glue_table_from_sdk() {
        let col = aws_sdk_glue::types::Column::builder()
            .name("col1")
            .r#type("string")
            .build()
            .unwrap();
        let partition_key = aws_sdk_glue::types::Column::builder()
            .name("dt")
            .r#type("string")
            .build()
            .unwrap();
        let serde_info = aws_sdk_glue::types::SerDeInfo::builder()
            .serialization_library("org.apache.hadoop.hive.serde2.lazy.LazySimpleSerDe")
            .build();
        let sd = aws_sdk_glue::types::StorageDescriptor::builder()
            .location("s3://bucket/table")
            .input_format("org.apache.hadoop.mapred.TextInputFormat")
            .output_format("org.apache.hadoop.hive.ql.io.HiveIgnoreKeyTextOutputFormat")
            .serde_info(serde_info)
            .columns(col)
            .build();
        let table = aws_sdk_glue::types::Table::builder()
            .name("my_table")
            .database_name("my_db")
            .table_type("EXTERNAL_TABLE")
            .storage_descriptor(sd)
            .partition_keys(partition_key)
            .build()
            .unwrap();
        let result = GlueTable::from(&table);
        assert_eq!(result.name, "my_table");
        assert_eq!(result.database_name, Some("my_db".to_string()));
        assert_eq!(result.table_type, Some("EXTERNAL_TABLE".to_string()));
        assert_eq!(result.location, Some("s3://bucket/table".to_string()));
        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0].name, "col1");
        assert_eq!(result.partition_keys.len(), 1);
        assert_eq!(result.partition_keys[0].name, "dt");
        assert!(result.serde.is_some());
    }

    #[test]
    fn test_glue_table_no_storage_descriptor() {
        let table = aws_sdk_glue::types::Table::builder()
            .name("bare_table")
            .build()
            .unwrap();
        let result = GlueTable::from(&table);
        assert_eq!(result.name, "bare_table");
        assert!(result.location.is_none());
        assert!(result.columns.is_empty());
        assert!(result.partition_keys.is_empty());
    }

    #[test]
    fn test_glue_crawler_from_sdk() {
        let crawler = aws_sdk_glue::types::Crawler::builder()
            .name("my-crawler")
            .role("arn:aws:iam::123456789012:role/GlueRole")
            .database_name("my_db")
            .build();
        let result = GlueCrawler::from(&crawler);
        assert_eq!(result.name, "my-crawler");
        assert_eq!(
            result.role,
            Some("arn:aws:iam::123456789012:role/GlueRole".to_string())
        );
        assert_eq!(result.database_name, Some("my_db".to_string()));
    }

    #[test]
    fn test_glue_crawler_minimal() {
        let crawler = aws_sdk_glue::types::Crawler::builder().build();
        let result = GlueCrawler::from(&crawler);
        assert_eq!(result.name, "");
        assert!(result.role.is_none());
        assert!(result.state.is_none());
        assert!(result.last_crawl_status.is_none());
    }

    #[test]
    fn test_glue_job_from_sdk() {
        let cmd = aws_sdk_glue::types::JobCommand::builder()
            .name("glueetl")
            .build();
        let job = aws_sdk_glue::types::Job::builder()
            .name("my-job")
            .role("arn:aws:iam::123456789012:role/GlueRole")
            .command(cmd)
            .max_capacity(10.0)
            .glue_version("3.0")
            .build();
        let result = GlueJob::from_sdk(&job, Some("SUCCEEDED".to_string()));
        assert_eq!(result.name, "my-job");
        assert_eq!(result.command_name, Some("glueetl".to_string()));
        assert_eq!(result.max_capacity, Some(10.0));
        assert_eq!(result.glue_version, Some("3.0".to_string()));
        assert_eq!(result.last_run_status, Some("SUCCEEDED".to_string()));
    }

    #[test]
    fn test_glue_job_minimal() {
        let job = aws_sdk_glue::types::Job::builder().build();
        let result = GlueJob::from_sdk(&job, None);
        assert_eq!(result.name, "");
        assert!(result.role.is_none());
        assert!(result.command_name.is_none());
        assert!(result.last_run_status.is_none());
    }
}
