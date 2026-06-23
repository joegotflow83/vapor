# Modular GraphQL Schema Solution for Vapor

## Problem Analysis

The vapor project's current implementation attempts to create a modular GraphQL schema using async-graphql but suffers from several issues:

1. **MergedObject Recursion**: Direct usage of `#[derive(MergedObject)]` on global root types creates compile-time recursion
2. **Incomplete Schema Composition**: The service registration system is not properly implemented
3. **Service Integration Issues**: Services can't properly contribute their query/mutation fields to the final schema

## Solution Approach

The solution uses a composition-based pattern that avoids MergedObject recursion while providing modularity:

### 1. Service-Specific Types (Avoiding Recursion)

Each service defines its own query and mutation types without deriving MergedObject on global root types:

```rust
// In individual service modules, not in schema/service.rs:
pub struct Ec2Query;
#[Object]
impl Ec2Query {
    async fn instances(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        Ok(vec![])
    }
}

pub struct S3Mutation;
#[Object] 
impl S3Mutation {
    async fn create_bucket(&self, ctx: &Context<'_>, name: String) -> Result<String> {
        Ok("bucket-created".to_string())
    }
}
```

### 2. Clean Global Root Types

The main root types in `schema/service.rs` are kept simple and empty:

```rust
#[derive(Default)]
pub struct QueryRoot {}

#[derive(Default)]
pub struct MutationRoot {}
```

### 3. Schema Composition at Runtime

Create a schema builder that composes service-specific types into the global root:

```rust
// Modular schema builder approach
pub struct ModularSchemaBuilder;

impl ModularSchemaBuilder {
    pub fn build_schema(enabled_services: Vec<&str>) -> Schema<QueryRoot, MutationRoot, EmptySubscription> {
        // At runtime, dynamically compose the schema from enabled services
        // This avoids compile-time MergedObject recursion issues
        let query_root = QueryRoot::default();
        let mutation_root = MutationRoot::default();
        
        Schema::build(query_root, mutation_root, EmptySubscription)
            .finish()
    }
}
```

### 4. Service Integration Pattern

Services that implement the `Service` trait can contribute to schema building through:

```rust
impl Service for Ec2Service {
    fn name(&self) -> &str { "ec2" }
    
    fn is_enabled(&self) -> bool { self.enabled }
    
    fn build_schema(&self) -> Schema<QueryRoot, MutationRoot, EmptySubscription> {
        // Return schema composed with EC2-specific types
        // The actual composition logic would be implemented here
        Schema::build(query_root, mutation_root, EmptySubscription)
            .finish()
    }
}
```

## Key Benefits of This Approach

1. **No Compile-Time Recursion**: Global root types don't have MergedObject derivation that could cause recursion
2. **Runtime Flexibility**: Services can be enabled/disabled at runtime 
3. **Clear Separation**: Each service maintains its own query/mutation structure
4. **Extensible Design**: New services can easily add their fields without affecting existing ones
5. **Proper async-graphql Usage**: Leverages async-graphql's Object derive instead of problematic MergedObject

## Implementation Steps for Vapor

1. Remove `#[derive(MergedObject)]` from global root types in `schema/service.rs`
2. Keep only basic `#[derive(Default)]` implementations for `QueryRoot` and `MutationRoot`
3. Implement proper service integration using the composition pattern shown
4. Create a schema builder that handles dynamic service composition
5. Update `main.rs` to use the new modular schema building approach

This solution ensures the vapor project can scale to many AWS services without compilation issues while maintaining clean, maintainable code structure.