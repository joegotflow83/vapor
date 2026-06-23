# Architecture Changes for Vapor Project

## Problem Statement

The current vapor project has a monolithic GraphQL schema that attempts to merge queries for over 40 AWS services using async-graphql's `MergedObject` derive macro. This approach exceeds the Rust compiler's recursion limits and causes compilation failures.

## Current Issues

1. **Schema Size**: The `QueryRoot` struct contains 40+ different AWS service query types
2. **Recursion Limit Exceeded**: async-graphql's `MergedObject` macro fails with `E0275` error due to excessive type nesting
3. **Performance**: Large schema increases compilation time and memory usage
4. **Maintainability**: Single monolithic schema is hard to maintain and extend

## Proposed Architecture Changes

### 1. Modular Schema Design

**Approach**: Split the large schema into smaller, service-specific schemas that can be composed at runtime.

```rust
// Instead of one massive QueryRoot struct:
#[derive(MergedObject, Default)]
pub struct QueryRoot(
    Ec2Query,
    SsmQuery,
    CloudWatchQuery,
    // ... 35+ more services
);

// Use a layered approach:
pub struct SchemaBuilder {
    pub ec2: Ec2Schema,
    pub ssm: SsmSchema,
    pub cloudwatch: CloudWatchSchema,
    // ... other service schemas
}
```

### 2. Service-Specific Modules

**Create separate modules for each AWS service group**:

```
src/
├── schema/
│   ├── mod.rs
│   ├── root.rs          # Main entry point
│   ├── aws/
│   │   ├── ec2/
│   │   │   ├── mod.rs
│   │   │   ├── queries.rs
│   │   │   └── types.rs
│   │   ├── ssm/
│   │   │   ├── mod.rs
│   │   │   ├── queries.rs
│   │   │   └── types.rs
│   │   └── ... other services
│   └── shared/
│       ├── types.rs
│       └── utils.rs
```

### 3. Dynamic Schema Composition

**Implement a schema composition pattern**:

```rust
// In src/schema/root.rs
pub struct SchemaConfig {
    pub enabled_services: Vec<String>,
    pub service_clients: ServiceClients,
}

pub fn build_schema(config: SchemaConfig) -> AppSchema {
    let mut schema_builder = Schema::build(QueryRoot::default(), MutationRoot::default(), EmptySubscription);
    
    // Conditionally add schemas based on configuration
    if config.enabled_services.contains(&"ec2".to_string()) {
        schema_builder = schema_builder.data(ec2_client);
    }
    
    // ... other service additions
    
    schema_builder.finish()
}
```

### 4. Service Registration Pattern

**Create a registry pattern for services**:

```rust
// src/schema/registry.rs
pub struct ServiceRegistry {
    services: HashMap<String, Box<dyn Service>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            services: HashMap::new(),
        };
        
        // Register services dynamically
        registry.register_service("ec2", Ec2Service::new());
        registry.register_service("ssm", SsmService::new());
        // ... other services
        
        registry
    }
    
    pub fn get_schema(&self) -> AppSchema {
        // Build schema from registered services
        // This allows for runtime composition
    }
}
```

### 5. Feature Flag System

**Add feature flags to enable/disable service groups**:

```rust
// Cargo.toml configuration
[features]
default = ["ec2", "ssm", "cloudwatch"]
full = ["ec2", "ssm", "cloudwatch", "rds", "ecs", "lambda", "s3", "dynamodb", "kms", "iam", "sns", "sqs", "apigateway", "elb", "autoscaling", "route53", "elasticache", "redshift", "firehose", "kinesis", "glue", "athena", "config", "guardduty", "inspector", "securityhub"]
minimal = ["ec2", "ssm"]
```

### 6. Lazy Loading of Schemas

**Implement lazy loading for service schemas**:

```rust
// src/schema/mod.rs
pub struct LazySchema {
    schema: Option<AppSchema>,
    service_config: ServiceConfig,
}

impl LazySchema {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            schema: None,
            service_config: config,
        }
    }
    
    pub fn get_schema(&mut self) -> &AppSchema {
        if self.schema.is_none() {
            self.schema = Some(build_schema(self.service_config.clone()));
        }
        self.schema.as_ref().unwrap()
    }
}
```

## Implementation Plan

### Phase 1: Refactor Schema Structure (Week 1)
- Create service-specific modules under `src/schema/aws/`
- Move existing query implementations to appropriate service directories
- Implement basic schema composition framework

### Phase 2: Service Registry Implementation (Week 2)
- Create service registration and loading system
- Implement dynamic schema building based on enabled services
- Add configuration options for service selection

### Phase 3: Feature Flag System (Week 3)
- Implement feature flags for service groups
- Add build-time optimization for different feature combinations
- Update documentation with usage examples

### Phase 4: Testing and Validation (Week 4)
- Test all service combinations work properly
- Verify compilation performance improvements
- Validate runtime performance

## Benefits of This Approach

1. **Reduced Compilation Time**: Smaller, modular schemas compile faster
2. **Improved Memory Usage**: Less memory required during compilation
3. **Better Maintainability**: Each AWS service is isolated in its own module
4. **Runtime Flexibility**: Enable/disable services at runtime
5. **Easier Testing**: Individual service schemas can be tested independently
6. **Scalability**: Easy to add new AWS services without affecting existing code

## Migration Strategy

1. **Backward Compatibility**: Maintain current API endpoints
2. **Gradual Rollout**: Implement changes incrementally
3. **Configuration Migration**: Provide configuration options for existing users
4. **Documentation Updates**: Update user guides and examples

This modular approach will resolve the compilation issues while maintaining all functionality of the original vapor application.