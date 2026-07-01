use async_graphql::SimpleObject;

/// A generic AWS resource tag. Shared across services — has no AWS SDK
/// dependency of its own, so it isn't gated behind any service feature flag.
#[derive(SimpleObject, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}
