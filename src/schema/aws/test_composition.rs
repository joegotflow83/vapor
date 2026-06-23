//! Test module for schema composition functionality

#[cfg(test)]
mod tests {
    use super::super::registry::{MutationRoot, QueryRoot};
    use async_graphql::{EmptySubscription, Schema};

    /// The composed `MergedObject` roots must build without GraphQL type- or
    /// field-name collisions, and the always-present base field must resolve.
    ///
    /// Under `--all-features` the root composes ~68 services; async-graphql's
    /// type-registry construction recurses through the deeply-nested
    /// `MergedObject` tuple and overflows the test harness's default 2 MiB
    /// thread stack, so build it on a thread with a roomier stack.
    #[test]
    fn test_schema_composition_builds() {
        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let schema = Schema::build(
                    QueryRoot::default(),
                    MutationRoot::default(),
                    EmptySubscription,
                )
                .finish();

                let rt = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .expect("build tokio runtime");
                rt.block_on(async { schema.execute("{ placeholder }").await.is_ok() })
            })
            .expect("spawn schema-build thread");

        assert!(handle.join().expect("schema-build thread panicked"));
    }
}
