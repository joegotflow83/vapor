use std::fmt;
use std::sync::Arc;

use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextResolve, ResolveInfo};
use async_graphql::{ServerResult, Value};
use aws_smithy_runtime_api::client::result::SdkError;
use aws_smithy_types::error::display::DisplayErrorContext;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;

#[derive(Debug)]
pub enum VaporError {
    AwsSdk {
        /// AWS error code, e.g. "AccessDeniedException", if available.
        code: Option<String>,
        message: String,
    },
    // Constructed only by feature-gated modules (e.g. `cloudwatch`); unused under the default feature set.
    #[allow(dead_code)]
    InvalidInput(String),
}

impl fmt::Display for VaporError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaporError::AwsSdk {
                code: Some(code),
                message,
            } => write!(f, "AWS SDK error: {code}: {message}"),
            VaporError::AwsSdk { code: None, message } => write!(f, "AWS SDK error: {message}"),
            VaporError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}

impl std::error::Error for VaporError {}

/// Converts an AWS SDK error into a `VaporError`, preserving the AWS error
/// code and message from the error's metadata instead of discarding them via
/// `SdkError`'s generic `Display` impl (which only prints "service error" /
/// "dispatch failure").
pub fn sdk_err<E, R>(e: SdkError<E, R>) -> VaporError
where
    E: ProvideErrorMetadata + std::error::Error + 'static,
    R: std::fmt::Debug,
{
    let code = e.code().map(String::from);
    let message = e
        .message()
        .map(String::from)
        .unwrap_or_else(|| format!("{}", DisplayErrorContext(&e)));
    VaporError::AwsSdk { code, message }
}

/// Surfaces the AWS error code (if any) on the GraphQL error's `extensions.code`
/// field. async-graphql's blanket `From<T: Display>` conversion (used implicitly
/// by every resolver's `?` on a `Result<_, VaporError>`) stashes the original
/// error behind `ServerError::source` before discarding it to a plain message
/// string; this extension recovers it there rather than requiring every
/// resolver to opt in individually (a `From<VaporError> for async_graphql::Error`
/// impl is not possible here — it would conflict with that same blanket impl).
pub struct ErrorCode;

impl ExtensionFactory for ErrorCode {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(ErrorCodeExtension)
    }
}

struct ErrorCodeExtension;

#[async_trait::async_trait]
impl Extension for ErrorCodeExtension {
    async fn resolve(
        &self,
        ctx: &ExtensionContext<'_>,
        info: ResolveInfo<'_>,
        next: NextResolve<'_>,
    ) -> ServerResult<Option<Value>> {
        next.run(ctx, info).await.map_err(|mut err| {
            if let Some(VaporError::AwsSdk { code: Some(code), .. }) =
                err.source.as_ref().and_then(|s| s.downcast_ref::<VaporError>())
            {
                err.extensions
                    .get_or_insert_with(Default::default)
                    .set("code", code.clone());
            }
            err
        })
    }
}

#[cfg(test)]
mod tests {
    use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};

    use super::*;

    struct Query;

    #[Object]
    impl Query {
        async fn with_code(&self) -> async_graphql::Result<String> {
            Err(VaporError::AwsSdk {
                code: Some("AccessDeniedException".to_string()),
                message: "not authorized".to_string(),
            }
            .into())
        }

        async fn without_code(&self) -> async_graphql::Result<String> {
            Err(VaporError::AwsSdk {
                code: None,
                message: "dispatch failure".to_string(),
            }
            .into())
        }
    }

    fn extension_code(res: &async_graphql::Response) -> Option<Value> {
        res.errors[0]
            .extensions
            .as_ref()
            .and_then(|e| e.get("code"))
            .cloned()
    }

    #[tokio::test]
    async fn surfaces_aws_error_code_as_extension() {
        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .extension(ErrorCode)
            .finish();
        let res = schema.execute("{ withCode }").await;
        assert_eq!(res.errors.len(), 1);
        assert_eq!(
            extension_code(&res),
            Some(Value::String("AccessDeniedException".to_string()))
        );
    }

    #[tokio::test]
    async fn leaves_extensions_unset_when_no_aws_code_available() {
        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .extension(ErrorCode)
            .finish();
        let res = schema.execute("{ withoutCode }").await;
        assert_eq!(res.errors.len(), 1);
        assert_eq!(extension_code(&res), None);
    }
}
