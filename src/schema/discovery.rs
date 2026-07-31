//! Makes "unknown field" errors self-healing.
//!
//! vapor's leaf field names mirror the AWS SDK's own names (see
//! `skills/aws-vapor/references/conventions.md`), so a caller guessing `name`
//! on a type AWS calls `FunctionName`
//! guesses wrong. async-graphql's built-in "Did you mean" can't bridge that gap
//! — its Levenshtein threshold is `max(len/2, 1)` and requires a strictly
//! smaller distance, so `name` → `functionName` (distance 8, threshold 6) never
//! suggests. Rather than tune a heuristic, this extension enumerates: on an
//! unknown field it appends the parent type's complete field list, turning two
//! round trips (fail, introspect, retry) into one.
//!
//! Registered on the schema builder, so it covers both the CLI and the server.

use std::sync::Arc;

use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextRequest};
use async_graphql::registry::Registry;
use async_graphql::Response;

/// Appends the parent type's field list to unknown-field validation errors.
pub struct FieldHints;

impl ExtensionFactory for FieldHints {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(FieldHintsExtension)
    }
}

struct FieldHintsExtension;

#[async_trait::async_trait]
impl Extension for FieldHintsExtension {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let mut response = next.run(ctx).await;
        for error in &mut response.errors {
            if let Some(hint) = field_hint(&ctx.schema_env.registry, &error.message) {
                error.message.push_str(&hint);
            }
        }
        response
    }
}

/// The text async-graphql's `FieldsOnCorrectType` rule puts between the
/// offending field and its parent type.
const ON_TYPE: &str = " on type \"";

/// Extracts the parent type from `Unknown field "x" on type "Y".`, ignoring any
/// trailing "Did you mean" clause.
fn unknown_field_parent(message: &str) -> Option<&str> {
    if !message.starts_with("Unknown field \"") {
        return None;
    }
    let (_, rest) = message.split_once(ON_TYPE)?;
    let (type_name, _) = rest.split_once('"')?;
    Some(type_name)
}

/// The sentence to append, or `None` if there's nothing useful to say.
fn field_hint(registry: &Registry, message: &str) -> Option<String> {
    let type_name = unknown_field_parent(message)?;

    // The roots carry 300+ query fields; listing them would bury the answer
    // instead of delivering it. Root-level typos are also the case the built-in
    // suggester actually handles well — a mistyped query name is usually close
    // to the real one, unlike a guessed leaf field.
    if type_name == registry.query_type || Some(type_name) == registry.mutation_type.as_deref() {
        return None;
    }

    let fields = registry.types.get(type_name)?.fields()?;
    if fields.is_empty() {
        return None;
    }

    let names: Vec<&str> = fields.keys().map(String::as_str).collect();
    Some(format!(" Fields on {type_name}: {}.", names.join(", ")))
}

#[cfg(test)]
mod tests {
    use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema, SimpleObject};

    use super::*;

    #[derive(SimpleObject)]
    struct LambdaFunction {
        function_name: String,
        runtime: String,
    }

    struct Query;

    #[Object]
    impl Query {
        async fn lambda_functions(&self) -> Vec<LambdaFunction> {
            Vec::new()
        }
    }

    fn schema() -> Schema<Query, EmptyMutation, EmptySubscription> {
        Schema::build(Query, EmptyMutation, EmptySubscription)
            .extension(FieldHints)
            .finish()
    }

    #[test]
    fn parses_the_parent_type_out_of_the_message() {
        assert_eq!(
            unknown_field_parent(r#"Unknown field "name" on type "LambdaFunction"."#),
            Some("LambdaFunction")
        );
        assert_eq!(
            unknown_field_parent(
                r#"Unknown field "stackNam" on type "CfnStack". Did you mean "stackName"?"#
            ),
            Some("CfnStack")
        );
        assert_eq!(unknown_field_parent("Cannot query field"), None);
    }

    #[tokio::test]
    async fn lists_the_fields_of_the_offending_type() {
        let res = schema().execute("{ lambdaFunctions { name } }").await;
        assert_eq!(res.errors.len(), 1);
        assert_eq!(
            res.errors[0].message,
            r#"Unknown field "name" on type "LambdaFunction". Fields on LambdaFunction: functionName, runtime."#
        );
    }

    #[tokio::test]
    async fn leaves_root_level_typos_to_the_built_in_suggester() {
        let res = schema().execute("{ lambdaFunction }").await;
        assert_eq!(res.errors.len(), 1);
        assert!(
            !res.errors[0].message.contains("Fields on"),
            "root typos should not be enumerated: {}",
            res.errors[0].message
        );
    }

    #[tokio::test]
    async fn leaves_valid_queries_untouched() {
        let res = schema()
            .execute("{ lambdaFunctions { functionName } }")
            .await;
        assert!(res.errors.is_empty());
    }
}
