//! Read-only enforcement for the `vapor read` subcommand.
//!
//! Every AWS-mutating operation in vapor lives on `MutationRoot` (see
//! `schema::aws::registry`), and `QueryRoot` never writes. So "the operation
//! type is `query`" is an exact read-only guarantee, not an approximation —
//! which is what makes it safe for an agent harness to allowlist `vapor read`
//! while still prompting on `vapor query`.

use async_graphql::parser::parse_query;
use async_graphql::parser::types::OperationType;

/// `extensions.code` reported when a write is refused, matching the AWS-code
/// convention the rest of the error surface uses.
pub const READ_ONLY_VIOLATION: &str = "ReadOnlyViolation";

/// Reject any operation that is not a `query`.
///
/// A document that fails to parse is deliberately *not* an error here: the
/// executor will reject it too, with its own message, and no AWS call can
/// happen either way. Deferring keeps malformed-query output identical between
/// `vapor read` and `vapor query` instead of introducing a second dialect.
pub fn ensure_read_only(query: &str) -> Result<(), String> {
    let Ok(doc) = parse_query(query) else {
        return Ok(());
    };

    for (_name, operation) in doc.operations.iter() {
        let kind = match operation.node.ty {
            OperationType::Query => continue,
            OperationType::Mutation => "a mutation",
            OperationType::Subscription => "a subscription",
        };
        return Err(format!(
            "vapor read accepts queries only; got {kind}. \
             Use `vapor query` if a write is genuinely intended."
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_a_plain_query() {
        assert!(ensure_read_only("{ instances { items { id } } }").is_ok());
    }

    #[test]
    fn allows_an_explicitly_named_query() {
        assert!(ensure_read_only("query Running { instances { items { id } } }").is_ok());
    }

    #[test]
    fn allows_introspection() {
        assert!(ensure_read_only("{ __schema { queryType { fields { name } } } }").is_ok());
    }

    #[test]
    fn refuses_a_mutation() {
        let err = ensure_read_only("mutation { stopInstances(ids: [\"i-0abc\"]) { instanceId } }")
            .unwrap_err();
        assert!(err.contains("a mutation"), "unexpected message: {err}");
    }

    #[test]
    fn refuses_a_mutation_hidden_among_queries() {
        let err = ensure_read_only(
            "query A { instances { items { id } } } \
             mutation B { stopInstances(ids: [\"i-0abc\"]) { instanceId } }",
        )
        .unwrap_err();
        assert!(err.contains("a mutation"), "unexpected message: {err}");
    }

    #[test]
    fn refuses_a_subscription() {
        let err = ensure_read_only("subscription { events { id } }").unwrap_err();
        assert!(err.contains("a subscription"), "unexpected message: {err}");
    }

    #[test]
    fn defers_malformed_documents_to_the_executor() {
        assert!(ensure_read_only("{ instances { items { id }").is_ok());
    }
}
