use async_graphql::SimpleObject;

use crate::aws::dynamodb::attribute_value_to_json;

// === Helper Types ===

/// DynamoDB key schema element (partition key or sort key definition).
#[derive(SimpleObject, Clone)]
pub struct DynamoKeySchema {
    /// The attribute name acting as the key.
    pub attribute_name: String,
    /// Key type: HASH (partition key) or RANGE (sort key).
    pub key_type: String,
}

impl From<&aws_sdk_dynamodb::types::KeySchemaElement> for DynamoKeySchema {
    fn from(ks: &aws_sdk_dynamodb::types::KeySchemaElement) -> Self {
        Self {
            attribute_name: ks.attribute_name().to_string(),
            key_type: ks.key_type().as_str().to_string(),
        }
    }
}

/// DynamoDB attribute definition (name + scalar type).
#[derive(SimpleObject, Clone)]
pub struct DynamoAttributeDefinition {
    pub attribute_name: String,
    /// Scalar attribute type: S (string), N (number), or B (binary).
    pub attribute_type: String,
}

impl From<&aws_sdk_dynamodb::types::AttributeDefinition> for DynamoAttributeDefinition {
    fn from(ad: &aws_sdk_dynamodb::types::AttributeDefinition) -> Self {
        Self {
            attribute_name: ad.attribute_name().to_string(),
            attribute_type: ad.attribute_type().as_str().to_string(),
        }
    }
}

/// A DynamoDB Global Secondary Index with its current status and throughput.
#[derive(SimpleObject, Clone)]
pub struct DynamoGlobalSecondaryIndex {
    pub index_name: String,
    pub key_schema: Vec<DynamoKeySchema>,
    pub projection_type: Option<String>,
    pub non_key_attributes: Vec<String>,
    pub index_status: Option<String>,
    pub read_capacity_units: Option<i64>,
    pub write_capacity_units: Option<i64>,
    pub item_count: Option<i64>,
    pub index_arn: Option<String>,
}

impl From<&aws_sdk_dynamodb::types::GlobalSecondaryIndexDescription> for DynamoGlobalSecondaryIndex {
    fn from(gsi: &aws_sdk_dynamodb::types::GlobalSecondaryIndexDescription) -> Self {
        let key_schema = gsi.key_schema().iter().map(DynamoKeySchema::from).collect();
        let projection_type = gsi
            .projection()
            .and_then(|p| p.projection_type())
            .map(|pt| pt.as_str().to_string());
        let non_key_attributes = gsi
            .projection()
            .map(|p| p.non_key_attributes().iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let read_capacity_units = gsi
            .provisioned_throughput()
            .and_then(|pt| pt.read_capacity_units());
        let write_capacity_units = gsi
            .provisioned_throughput()
            .and_then(|pt| pt.write_capacity_units());

        Self {
            index_name: gsi.index_name().unwrap_or_default().to_string(),
            key_schema,
            projection_type,
            non_key_attributes,
            index_status: gsi.index_status().map(|s| s.as_str().to_string()),
            read_capacity_units,
            write_capacity_units,
            item_count: gsi.item_count(),
            index_arn: gsi.index_arn().map(|s| s.to_string()),
        }
    }
}

/// A DynamoDB Local Secondary Index.
#[derive(SimpleObject, Clone)]
pub struct DynamoLocalSecondaryIndex {
    pub index_name: String,
    pub key_schema: Vec<DynamoKeySchema>,
    pub projection_type: Option<String>,
    pub non_key_attributes: Vec<String>,
    pub item_count: Option<i64>,
    pub index_arn: Option<String>,
}

impl From<&aws_sdk_dynamodb::types::LocalSecondaryIndexDescription> for DynamoLocalSecondaryIndex {
    fn from(lsi: &aws_sdk_dynamodb::types::LocalSecondaryIndexDescription) -> Self {
        let key_schema = lsi.key_schema().iter().map(DynamoKeySchema::from).collect();
        let projection_type = lsi
            .projection()
            .and_then(|p| p.projection_type())
            .map(|pt| pt.as_str().to_string());
        let non_key_attributes = lsi
            .projection()
            .map(|p| p.non_key_attributes().iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        Self {
            index_name: lsi.index_name().unwrap_or_default().to_string(),
            key_schema,
            projection_type,
            non_key_attributes,
            item_count: lsi.item_count(),
            index_arn: lsi.index_arn().map(|s| s.to_string()),
        }
    }
}

// === Output Types ===

/// A DynamoDB table with full metadata including indexes and stream settings.
#[derive(SimpleObject, Clone)]
pub struct DynamoTable {
    pub name: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    /// ISO 8601 creation timestamp.
    pub creation_date_time: Option<String>,
    pub billing_mode: Option<String>,
    pub read_capacity_units: Option<i64>,
    pub write_capacity_units: Option<i64>,
    pub item_count: Option<i64>,
    pub table_size_bytes: Option<i64>,
    pub key_schema: Vec<DynamoKeySchema>,
    pub attribute_definitions: Vec<DynamoAttributeDefinition>,
    pub global_secondary_indexes: Vec<DynamoGlobalSecondaryIndex>,
    pub local_secondary_indexes: Vec<DynamoLocalSecondaryIndex>,
    pub stream_enabled: Option<bool>,
    pub stream_view_type: Option<String>,
    pub stream_arn: Option<String>,
}

impl From<aws_sdk_dynamodb::types::TableDescription> for DynamoTable {
    fn from(t: aws_sdk_dynamodb::types::TableDescription) -> Self {
        let key_schema = t.key_schema().iter().map(DynamoKeySchema::from).collect();
        let attribute_definitions = t
            .attribute_definitions()
            .iter()
            .map(DynamoAttributeDefinition::from)
            .collect();
        let global_secondary_indexes = t
            .global_secondary_indexes()
            .iter()
            .map(DynamoGlobalSecondaryIndex::from)
            .collect();
        let local_secondary_indexes = t
            .local_secondary_indexes()
            .iter()
            .map(DynamoLocalSecondaryIndex::from)
            .collect();

        let billing_mode = t
            .billing_mode_summary()
            .and_then(|bms| bms.billing_mode())
            .map(|bm| bm.as_str().to_string());
        let read_capacity_units = t
            .provisioned_throughput()
            .and_then(|pt| pt.read_capacity_units());
        let write_capacity_units = t
            .provisioned_throughput()
            .and_then(|pt| pt.write_capacity_units());

        let stream_enabled = t.stream_specification().map(|ss| ss.stream_enabled());
        let stream_view_type = t
            .stream_specification()
            .and_then(|ss| ss.stream_view_type())
            .map(|svt| svt.as_str().to_string());
        let creation_date_time = t
            .creation_date_time()
            .map(|dt| dt.to_string());

        Self {
            name: t.table_name().unwrap_or_default().to_string(),
            arn: t.table_arn().map(|s| s.to_string()),
            status: t.table_status().map(|s| s.as_str().to_string()),
            creation_date_time,
            billing_mode,
            read_capacity_units,
            write_capacity_units,
            item_count: t.item_count(),
            table_size_bytes: t.table_size_bytes(),
            key_schema,
            attribute_definitions,
            global_secondary_indexes,
            local_secondary_indexes,
            stream_enabled,
            stream_view_type,
            stream_arn: t.latest_stream_arn().map(|s| s.to_string()),
        }
    }
}

/// Result of a DynamoDB Scan operation (single page).
/// Items are serialized as JSON strings for flexibility.
#[derive(SimpleObject, Clone)]
pub struct DynamoScanResult {
    /// Each item is a JSON-serialized map of attribute name → value.
    pub items: Vec<String>,
    pub count: i32,
    pub scanned_count: i32,
    /// JSON-serialized last evaluated key for manual pagination, if present.
    pub last_evaluated_key: Option<String>,
}

impl DynamoScanResult {
    pub fn from_scan_output(output: aws_sdk_dynamodb::operation::scan::ScanOutput) -> Self {
        let items: Vec<String> = output
            .items()
            .iter()
            .map(|item| {
                let json_map: serde_json::Map<String, serde_json::Value> = item
                    .iter()
                    .map(|(k, v)| (k.clone(), attribute_value_to_json(v)))
                    .collect();
                serde_json::Value::Object(json_map).to_string()
            })
            .collect();

        let last_evaluated_key = output
            .last_evaluated_key()
            .filter(|lek| !lek.is_empty())
            .map(|lek| {
                let lek_map: serde_json::Map<String, serde_json::Value> = lek
                    .iter()
                    .map(|(k, v)| (k.clone(), attribute_value_to_json(v)))
                    .collect();
                serde_json::Value::Object(lek_map).to_string()
            });

        Self {
            items,
            count: output.count(),
            scanned_count: output.scanned_count(),
            last_evaluated_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::dynamodb::attribute_value_to_json;
    use aws_sdk_dynamodb::types::AttributeValue;

    #[test]
    fn test_attribute_value_string() {
        let av = AttributeValue::S("hello".to_string());
        assert_eq!(attribute_value_to_json(&av), serde_json::Value::String("hello".to_string()));
    }

    #[test]
    fn test_attribute_value_number_integer() {
        let av = AttributeValue::N("42".to_string());
        assert_eq!(attribute_value_to_json(&av), serde_json::json!(42i64));
    }

    #[test]
    fn test_attribute_value_number_float() {
        let av = AttributeValue::N("3.14".to_string());
        let result = attribute_value_to_json(&av);
        // Should be a number
        assert!(result.is_number() || result.is_string());
    }

    #[test]
    fn test_attribute_value_bool() {
        let av = AttributeValue::Bool(true);
        assert_eq!(attribute_value_to_json(&av), serde_json::Value::Bool(true));
    }

    #[test]
    fn test_attribute_value_null() {
        let av = AttributeValue::Null(true);
        assert_eq!(attribute_value_to_json(&av), serde_json::Value::Null);
    }

    #[test]
    fn test_attribute_value_list() {
        let av = AttributeValue::L(vec![
            AttributeValue::S("a".to_string()),
            AttributeValue::N("1".to_string()),
        ]);
        let result = attribute_value_to_json(&av);
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], serde_json::Value::String("a".to_string()));
    }

    #[test]
    fn test_attribute_value_map() {
        let mut map = std::collections::HashMap::new();
        map.insert("key".to_string(), AttributeValue::S("val".to_string()));
        let av = AttributeValue::M(map);
        let result = attribute_value_to_json(&av);
        assert!(result.is_object());
        assert_eq!(result["key"], serde_json::Value::String("val".to_string()));
    }

    #[test]
    fn test_attribute_value_string_set() {
        let av = AttributeValue::Ss(vec!["a".to_string(), "b".to_string()]);
        let result = attribute_value_to_json(&av);
        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_attribute_value_number_set() {
        let av = AttributeValue::Ns(vec!["1".to_string(), "2".to_string()]);
        let result = attribute_value_to_json(&av);
        assert!(result.is_array());
    }

    #[test]
    fn test_dynamo_key_schema_from() {
        let ks = aws_sdk_dynamodb::types::KeySchemaElement::builder()
            .attribute_name("pk")
            .key_type(aws_sdk_dynamodb::types::KeyType::Hash)
            .build()
            .unwrap();
        let result = DynamoKeySchema::from(&ks);
        assert_eq!(result.attribute_name, "pk");
        assert_eq!(result.key_type, "HASH");
    }

    #[test]
    fn test_dynamo_attribute_definition_from() {
        let ad = aws_sdk_dynamodb::types::AttributeDefinition::builder()
            .attribute_name("pk")
            .attribute_type(aws_sdk_dynamodb::types::ScalarAttributeType::S)
            .build()
            .unwrap();
        let result = DynamoAttributeDefinition::from(&ad);
        assert_eq!(result.attribute_name, "pk");
        assert_eq!(result.attribute_type, "S");
    }

    #[test]
    fn test_dynamo_scan_result_empty() {
        // Build a minimal ScanOutput from parts
        let output = aws_sdk_dynamodb::operation::scan::ScanOutput::builder()
            .count(0)
            .scanned_count(0)
            .build();
        let result = DynamoScanResult::from_scan_output(output);
        assert_eq!(result.count, 0);
        assert_eq!(result.scanned_count, 0);
        assert!(result.items.is_empty());
        assert!(result.last_evaluated_key.is_none());
    }

    #[test]
    fn test_dynamo_scan_result_with_items() {
        let mut item = std::collections::HashMap::new();
        item.insert("id".to_string(), AttributeValue::S("row1".to_string()));
        item.insert("count".to_string(), AttributeValue::N("5".to_string()));

        let output = aws_sdk_dynamodb::operation::scan::ScanOutput::builder()
            .set_items(Some(vec![item]))
            .count(1)
            .scanned_count(1)
            .build();
        let result = DynamoScanResult::from_scan_output(output);
        assert_eq!(result.count, 1);
        assert_eq!(result.items.len(), 1);
        // Verify item is valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result.items[0]).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed["id"], serde_json::Value::String("row1".to_string()));
    }
}
