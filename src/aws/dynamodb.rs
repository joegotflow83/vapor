#[cfg(feature = "dynamodb")]
use aws_config::SdkConfig;
#[cfg(feature = "dynamodb")]
use aws_sdk_dynamodb::types::AttributeValue;

#[cfg(feature = "dynamodb")]
use crate::error::VaporError;

#[cfg(feature = "dynamodb")]
pub struct DynamodbClient {
    inner: aws_sdk_dynamodb::Client,
}

impl DynamodbClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_dynamodb::Client::new(config),
        }
    }

    /// List DynamoDB table names with resumable, name-based cursor pagination.
    pub async fn list_tables(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut all_names: Vec<String> = Vec::new();
        let mut exclusive_start = next_token;

        loop {
            let mut req = self.inner.list_tables();
            if let Some(ref name) = exclusive_start {
                req = req.exclusive_start_table_name(name);
            }
            if let Some(l) = limit {
                req = req.limit(l - all_names.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            all_names.extend(output.table_names().iter().map(|s| s.to_string()));
            exclusive_start = output.last_evaluated_table_name().map(|s| s.to_string());

            match (&exclusive_start, limit) {
                (None, _) => break,
                (_, Some(l)) if all_names.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((all_names, exclusive_start))
    }

    /// Describe a single DynamoDB table by name.
    pub async fn describe_table(
        &self,
        name: &str,
    ) -> Result<aws_sdk_dynamodb::types::TableDescription, VaporError> {
        let output = self
            .inner
            .describe_table()
            .table_name(name)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        output
            .table()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk { code: None, message: format!("Table '{}' not found", name) })
    }

    /// Scan a DynamoDB table (single page only, not auto-paginated).
    pub async fn scan(
        &self,
        table: &str,
        filter_expression: Option<&str>,
        limit: Option<i32>,
    ) -> Result<aws_sdk_dynamodb::operation::scan::ScanOutput, VaporError> {
        let mut req = self.inner.scan().table_name(table);
        if let Some(expr) = filter_expression {
            req = req.filter_expression(expr);
        }
        if let Some(lim) = limit {
            req = req.limit(lim);
        }
        req.send().await.map_err(crate::error::sdk_err)
    }
}

/// Recursively convert a DynamoDB AttributeValue to a serde_json::Value.
/// Binary values (B, BS) are base64-encoded strings.
pub fn attribute_value_to_json(av: &AttributeValue) -> serde_json::Value {
    match av {
        AttributeValue::S(s) => serde_json::Value::String(s.clone()),
        AttributeValue::N(n) => {
            // Attempt numeric parse; fall back to string to preserve precision
            if let Ok(i) = n.parse::<i64>() {
                serde_json::Value::Number(i.into())
            } else if let Ok(f) = n.parse::<f64>() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(n.clone()))
            } else {
                serde_json::Value::String(n.clone())
            }
        }
        AttributeValue::Bool(b) => serde_json::Value::Bool(*b),
        AttributeValue::Null(_) => serde_json::Value::Null,
        AttributeValue::B(blob) => {
            use base64::Engine;
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(blob.as_ref()))
        }
        AttributeValue::L(list) => {
            serde_json::Value::Array(list.iter().map(attribute_value_to_json).collect())
        }
        AttributeValue::M(map) => {
            let obj: serde_json::Map<String, serde_json::Value> =
                map.iter().map(|(k, v)| (k.clone(), attribute_value_to_json(v))).collect();
            serde_json::Value::Object(obj)
        }
        AttributeValue::Ss(ss) => {
            serde_json::Value::Array(ss.iter().map(|s| serde_json::Value::String(s.clone())).collect())
        }
        AttributeValue::Ns(ns) => {
            serde_json::Value::Array(ns.iter().map(|n| serde_json::Value::String(n.clone())).collect())
        }
        AttributeValue::Bs(bs) => {
            use base64::Engine;
            serde_json::Value::Array(
                bs.iter()
                    .map(|b| serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b.as_ref())))
                    .collect(),
            )
        }
        // Unknown variant fallback
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://dynamodb.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_tables_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(200, r#"{"TableNames":["t1","t2"]}"#),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_tables(None, None).await.unwrap();

        assert_eq!(names, vec!["t1".to_string(), "t2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ExclusiveStartTableName":"cursor-a"}"#),
            json_response(200, r#"{"TableNames":["t3"]}"#),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_tables(None, Some("cursor-a".to_string())).await.unwrap();

        assert_eq!(names, vec!["t3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":2}"#),
            json_response(200, r#"{"TableNames":["a1","a2"],"LastEvaluatedTableName":"a2"}"#),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_tables(Some(2), None).await.unwrap();

        assert_eq!(names, vec!["a1".to_string(), "a2".to_string()]);
        assert_eq!(token, Some("a2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":10}"#),
                json_response(200, r#"{"TableNames":["a1","a2"],"LastEvaluatedTableName":"a2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ExclusiveStartTableName":"a2","Limit":8}"#),
                json_response(200, r#"{"TableNames":["a3"]}"#),
            ),
        ]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_tables(Some(10), None).await.unwrap();

        assert_eq!(names, vec!["a1".to_string(), "a2".to_string(), "a3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let err = client.list_tables(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_table_returns_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"TableName":"my-table"}"#),
            json_response(
                200,
                r#"{"Table":{"TableName":"my-table","TableStatus":"ACTIVE","ItemCount":42}}"#,
            ),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let table = client.describe_table("my-table").await.unwrap();

        assert_eq!(table.table_name(), Some("my-table"));
        assert_eq!(table.table_status(), Some(&aws_sdk_dynamodb::types::TableStatus::Active));
        assert_eq!(table.item_count(), Some(42));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_table_errors_when_response_has_no_table_field() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"TableName":"ghost-table"}"#),
            json_response(200, r#"{}"#),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_table("ghost-table").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, None);
                assert_eq!(message, "Table 'ghost-table' not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_table_propagates_sdk_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"TableName":"missing-table"}"#),
            json_error_response("ResourceNotFoundException", "table not found"),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_table("missing-table").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "table not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn scan_passes_through_filter_expression_and_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TableName":"my-table","Limit":5,"FilterExpression":"attr = :v"}"#,
            ),
            json_response(
                200,
                r#"{"Items":[{"id":{"S":"1"}}],"Count":1,"ScannedCount":1}"#,
            ),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let output = client
            .scan("my-table", Some("attr = :v"), Some(5))
            .await
            .unwrap();

        assert_eq!(output.count(), 1);
        assert_eq!(output.items().len(), 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn scan_omits_optional_fields_when_not_provided() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"TableName":"my-table"}"#),
            json_response(200, r#"{"Items":[],"Count":0,"ScannedCount":0}"#),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let output = client.scan("my-table", None, None).await.unwrap();

        assert_eq!(output.count(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn scan_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"TableName":"my-table"}"#),
            json_error_response("ResourceNotFoundException", "table not found"),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));

        let err = client.scan("my-table", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "table not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[test]
    fn attribute_value_to_json_converts_string() {
        assert_eq!(
            attribute_value_to_json(&AttributeValue::S("hello".to_string())),
            serde_json::Value::String("hello".to_string())
        );
    }

    #[test]
    fn attribute_value_to_json_converts_integer_number() {
        assert_eq!(
            attribute_value_to_json(&AttributeValue::N("42".to_string())),
            serde_json::json!(42)
        );
    }

    #[test]
    fn attribute_value_to_json_converts_float_number() {
        assert_eq!(
            attribute_value_to_json(&AttributeValue::N("3.5".to_string())),
            serde_json::json!(3.5)
        );
    }

    #[test]
    fn attribute_value_to_json_falls_back_to_string_for_unparseable_number() {
        assert_eq!(
            attribute_value_to_json(&AttributeValue::N("not-a-number".to_string())),
            serde_json::Value::String("not-a-number".to_string())
        );
    }

    #[test]
    fn attribute_value_to_json_converts_bool_and_null() {
        assert_eq!(
            attribute_value_to_json(&AttributeValue::Bool(true)),
            serde_json::Value::Bool(true)
        );
        assert_eq!(attribute_value_to_json(&AttributeValue::Null(true)), serde_json::Value::Null);
    }

    #[test]
    fn attribute_value_to_json_base64_encodes_binary() {
        let av = AttributeValue::B(aws_smithy_types::Blob::new(b"hi".to_vec()));
        assert_eq!(
            attribute_value_to_json(&av),
            serde_json::Value::String("aGk=".to_string())
        );
    }

    #[test]
    fn attribute_value_to_json_converts_string_and_number_sets() {
        let ss = AttributeValue::Ss(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(attribute_value_to_json(&ss), serde_json::json!(["a", "b"]));

        let ns = AttributeValue::Ns(vec!["1".to_string(), "2".to_string()]);
        assert_eq!(attribute_value_to_json(&ns), serde_json::json!(["1", "2"]));
    }

    #[test]
    fn attribute_value_to_json_base64_encodes_binary_set() {
        let bs = AttributeValue::Bs(vec![aws_smithy_types::Blob::new(b"hi".to_vec())]);
        assert_eq!(attribute_value_to_json(&bs), serde_json::json!(["aGk="]));
    }

    #[test]
    fn attribute_value_to_json_converts_list_recursively() {
        let list = AttributeValue::L(vec![
            AttributeValue::S("a".to_string()),
            AttributeValue::N("1".to_string()),
        ]);
        assert_eq!(attribute_value_to_json(&list), serde_json::json!(["a", 1]));
    }

    #[test]
    fn attribute_value_to_json_converts_map_recursively() {
        let mut map = std::collections::HashMap::new();
        map.insert("k".to_string(), AttributeValue::S("v".to_string()));
        let av = AttributeValue::M(map);
        assert_eq!(attribute_value_to_json(&av), serde_json::json!({"k": "v"}));
    }
}
