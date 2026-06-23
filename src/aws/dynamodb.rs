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

    /// List all DynamoDB table names with name-based cursor pagination.
    pub async fn list_tables(&self) -> Result<Vec<String>, VaporError> {
        let mut all_names: Vec<String> = Vec::new();
        let mut exclusive_start: Option<String> = None;

        loop {
            let mut req = self.inner.list_tables();
            if let Some(ref name) = exclusive_start {
                req = req.exclusive_start_table_name(name);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_names.extend(output.table_names().iter().map(|s| s.to_string()));

            match output.last_evaluated_table_name() {
                Some(name) => exclusive_start = Some(name.to_string()),
                None => break,
            }
        }

        Ok(all_names)
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        output
            .table()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk(format!("Table '{}' not found", name)))
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
        req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))
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
