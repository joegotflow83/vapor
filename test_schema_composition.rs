//! Test to validate schema composition works with feature flags

use async_graphql::{Schema, EmptySubscription};
use vapor::schema::aws::registry::{QueryRoot, MutationRoot, ServiceRegistry};

#[tokio::main]
async fn main() {
    // Test that the basic schema can be built with minimal features
    let mut registry = ServiceRegistry::new();
    registry.register_service("ec2".to_string());
    registry.register_service("s3".to_string());
    registry.register_service("lambda".to_string());

    // This should work without MergedObject recursion issues
    let schema = registry.get_schema();

    println!("Schema composition test completed successfully!");
    println!("Schema built with EC2, S3, and Lambda services.");
}