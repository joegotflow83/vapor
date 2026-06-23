# Plan: wire up the remaining services

This is the rollout plan for connecting every remaining AWS service end-to-end,
using the pattern established for EC2. For the *how* of a single service, see
[`wiring.md`](./wiring.md); this document is the *what's left, in what order,
and the gotchas*.

## Status

| | Count |
|---|---|
| Services with a real client (`src/aws/<svc>.rs`) | 66 |
| Services with a real resolver (`src/schema/<svc>/`) | 66 |
| **Wired into the schema roots** | **1 (ec2)** |
| **Remaining to wire** | **65 + `vpc`** |

The good news from the inventory:

- **Every** resolver is already real — all 66 `queries.rs` pull a client via
  `ctx.data::<…>()`. None are placeholder `Ok(vec![])` stubs. The work is pure
  wiring, not implementation.
- **Every** client uses the identical constructor `Client::new(config: &SdkConfig)`.
- **Zero** top-level field-name collisions across all 218 resolver fields, so the
  `MergedObject` roots compose without any renaming.

So wiring a service is exactly the three edits from `wiring.md`, repeated. The
only friction is a set of name mismatches and three special cases, catalogued
below.

## The per-service edit (recap)

For each service, the same three feature-gated edits:

```rust
// 1. src/schema/mod.rs  — declare the resolver module (gate it on the feature!)
#[cfg(feature = "<feature>")]
pub mod <module>;

// 2. src/schema/aws/registry.rs — add to the roots
#[derive(MergedObject, Default)]
pub struct QueryRoot(
    BaseQuery,
    #[cfg(feature = "ec2")] crate::schema::ec2::queries::Ec2Query,
    #[cfg(feature = "<feature>")] crate::schema::<module>::queries::<Svc>Query,  // ← add
);

// 3. src/schema/root.rs — register the client in context
#[cfg(feature = "<feature>")]
{ builder = builder.data(crate::aws::<file>::<Svc>Client::new(config)); }
```

> **Gate the module declaration too.** Each `src/schema/<svc>/queries.rs` imports
> its client from `crate::aws::<svc>`, which is itself `#[cfg(feature = "…")]` in
> `src/aws/mod.rs`. If you declare `pub mod <svc>;` unconditionally, a
> `--no-default-features` build (or any build without that feature) fails to
> compile. Always gate the `pub mod` on the same feature.
>
> *(Note: the existing `pub mod ec2;` in `src/schema/mod.rs` is currently
> ungated. It's harmless for the default build since `ec2` is a default feature,
> but should be wrapped in `#[cfg(feature = "ec2")]` for correctness when this
> pass starts.)*

Only EC2 has a `mutations.rs` today, so `MutationRoot` only gains EC2 for now.
Every other service is query-only until someone authors its mutations — that's
the same step-2 edit, against `MutationRoot`, when the time comes.

## Master mapping table

The three identifiers you need per service. **Bold** = a mismatch to watch (the
module name, feature name, client name, and query name are not all the same
stem).

| Module (`src/schema/`) | Cargo feature | Client (`src/aws/`) | Query struct | Notes |
|---|---|---|---|---|
| `acm` | `acm` | `AcmClient` | `AcmQuery` | |
| `apigateway` | `apigateway` | `ApiGatewayClient` | `ApiGatewayQuery` | |
| `apigatewayv2` | `apigatewayv2` | `ApiGatewayV2Client` | `ApiGatewayV2Query` | |
| `appconfig` | `appconfig` | `AppConfigClient` | `AppConfigQuery` | |
| `appsync` | `appsync` | `AppSyncClient` | `AppSyncQuery` | |
| **`asg`** | **`autoscaling`** | **`AutoscalingClient`** | **`AsgQuery`** | all four differ |
| `athena` | `athena` | `AthenaClient` | `AthenaQuery` | |
| `backup` | `backup` | `BackupClient` | `BackupQuery` | |
| `batch` | `batch` | `BatchClient` | `BatchQuery` | |
| `cloudformation` | `cloudformation` | `CloudFormationClient` | `CloudFormationQuery` | |
| `cloudfront` | `cloudfront` | `CloudFrontClient` | `CloudFrontQuery` | |
| `cloudtrail` | `cloudtrail` | `CloudTrailClient` | `CloudTrailQuery` | |
| **`cloudwatch`** | `cloudwatch` **+ `cloudwatchlogs`** | `CloudWatchClient` **+ `CloudWatchLogsClient`** | `CloudWatchQuery` | **two clients** — see special cases |
| `codebuild` | `codebuild` | `CodeBuildClient` | `CodeBuildQuery` | |
| `codedeploy` | `codedeploy` | `CodeDeployClient` | `CodeDeployQuery` | |
| `codepipeline` | `codepipeline` | `CodePipelineClient` | `CodePipelineQuery` | |
| **`cognito`** | **`cognitoidentityprovider`** | `CognitoClient` | `CognitoQuery` | feature name differs |
| **`config_svc`** | **`config`** | **`AwsConfigClient`** | **`AwsConfigQuery`** | module & client differ |
| `cost_explorer` | `costexplorer` | `CostExplorerClient` | `CostExplorerQuery` | |
| `direct_connect` | `directconnect` | `DirectConnectClient` | `DirectConnectQuery` | |
| **`documentdb`** | **`docdb`** | `DocumentDbClient` | `DocumentDbQuery` | feature name differs |
| `dynamodb` | `dynamodb` | `DynamodbClient` | `DynamodbQuery` | client is `Dynamodb…` (lowercase d) |
| `ec2` | `ec2` | `Ec2Client` | `Ec2Query` (+`Ec2Mutation`) | **DONE** |
| `ecr` | `ecr` | `EcrClient` | `EcrQuery` | |
| `ecs` | `ecs` | `EcsClient` | `EcsQuery` | |
| `efs` | `efs` | `EfsClient` | `EfsQuery` | |
| `eks` | `eks` | `EksClient` | `EksQuery` | |
| `elasticache` | `elasticache` | `ElastiCacheClient` | `ElastiCacheQuery` | |
| `elbv2` | `elbv2` | `Elbv2Client` | `Elbv2Query` | |
| `emr` | `emr` | `EmrClient` | `EmrQuery` | |
| `eventbridge` | `eventbridge` | `EventBridgeClient` | `EventBridgeQuery` | |
| `firehose` | `firehose` | `FirehoseClient` | `FirehoseQuery` | |
| `global_accelerator` | `globalaccelerator` | `GlobalAcceleratorClient` | `GlobalAcceleratorQuery` | |
| `glue` | `glue` | `GlueClient` | `GlueQuery` | |
| `guardduty` | `guardduty` | `GuardDutyClient` | `GuardDutyQuery` | |
| `health` | `health` | `HealthClient` | `HealthQuery` | |
| `iam` | `iam` | `IamClient` | `IamQuery` | |
| **`inspector`** | **`inspector2`** | `InspectorClient` | `InspectorQuery` | feature name differs |
| `kinesis` | `kinesis` | `KinesisClient` | `KinesisQuery` | |
| `kms` | `kms` | `KmsClient` | `KmsQuery` | |
| `lambda` | `lambda` | `LambdaClient` | `LambdaQuery` | **remove `aws/lambda` stub** |
| **`macie`** | **`macie2`** | `MacieClient` | `MacieQuery` | feature name differs |
| `memorydb` | `memorydb` | `MemoryDbClient` | `MemoryDbQuery` | |
| **`msk`** | **`kafka`** | `MskClient` | `MskQuery` | module & feature differ |
| `neptune` | `neptune` | `NeptuneClient` | `NeptuneQuery` | |
| `network_firewall` | `networkfirewall` | `NetworkFirewallClient` | `NetworkFirewallQuery` | |
| `opensearch` | `opensearch` | `OpenSearchClient` | `OpenSearchQuery` | |
| `organizations` | `organizations` | `OrganizationsClient` | `OrganizationsQuery` | |
| `rds` | `rds` | `RdsClient` | `RdsQuery` | |
| `redshift` | `redshift` | `RedshiftClient` | `RedshiftQuery` | |
| `redshift_serverless` | `redshiftserverless` | `RedshiftServerlessClient` | `RedshiftServerlessQuery` | |
| `route53` | `route53` | `Route53Client` | `Route53Query` | |
| `s3` | `s3` | `S3Client` | `S3Query` | **remove `aws/s3` stub** |
| `sagemaker` | `sagemaker` | `SageMakerClient` | `SageMakerQuery` | |
| `secrets_manager` | `secretsmanager` | `SecretsManagerClient` | `SecretsManagerQuery` | |
| `security_hub` | `securityhub` | `SecurityHubClient` | `SecurityHubQuery` | |
| `service_quotas` | `servicequotas` | `ServiceQuotasClient` | `ServiceQuotasQuery` | |
| `shield` | `shield` | `ShieldClient` | `ShieldQuery` | |
| `sns` | `sns` | `SnsClient` | `SnsQuery` | |
| `sqs` | `sqs` | `SqsClient` | `SqsQuery` | |
| `ssm` | `ssm` | `SsmClient` | `SsmQuery` | **remove `aws/ssm` stub** |
| **`step_functions`** | **`sfn`** | `StepFunctionsClient` | `StepFunctionsQuery` | module & feature differ |
| `sts` | `sts` | `StsClient` | `StsQuery` | returns a single object, not a list |
| `transfer` | `transfer` | `TransferClient` | `TransferQuery` | |
| **`vpc`** | *(none — uses `ec2`)* | `Ec2Client` | `VpcQuery` | see special cases |
| `wafv2` | `wafv2` | `WafV2Client` | `Wafv2Query` | client `WafV2…`, query `Wafv2…` |

## Special cases

**`cloudwatch` — two clients.** `cloudwatch/queries.rs` pulls both
`CloudWatchClient` and `CloudWatchLogsClient`. Register both in `root.rs`, each
under its own feature, and ensure both features are enabled when you build it:

```rust
#[cfg(feature = "cloudwatch")]
{ builder = builder.data(crate::aws::cloudwatch::CloudWatchClient::new(config)); }
#[cfg(feature = "cloudwatchlogs")]
{ builder = builder.data(crate::aws::cloudwatch_logs::CloudWatchLogsClient::new(config)); }
```

There is no separate `cloudwatchlogs` resolver module — its queries live inside
`CloudWatchQuery`. (Client file is `src/aws/cloudwatch_logs.rs`.)

**`vpc` — no feature of its own.** `VpcQuery` exposes the VPC-adjacent EC2 calls
(`route_tables`, `network_acls`, `internet_gateways`, `nat_gateways`,
`vpc_endpoints`, `transit_gateways`, `vpc_flow_logs`) and reuses `Ec2Client`.
These do **not** overlap with `Ec2Query`'s fields, so it composes cleanly. Gate
the module and the root field on `feature = "ec2"` and register **no** new client
(the `Ec2Client` registered for EC2 is reused):

```rust
// src/schema/mod.rs
#[cfg(feature = "ec2")]
pub mod vpc;

// registry.rs QueryRoot
#[cfg(feature = "ec2")] crate::schema::vpc::queries::VpcQuery,
// root.rs: nothing to add — Ec2Client is already registered
```

**`s3` / `lambda` / `ssm` — competing stub modules.** Real resolvers live at
`src/schema/{s3,lambda,ssm}/`, but legacy stubs still exist at
`src/schema/aws/{s3,lambda,ssm}/` and are declared in `src/schema/aws/mod.rs`.
When wiring these three:

1. Declare the real module in `src/schema/mod.rs` (gated).
2. **Remove** the stub `pub mod {s3,lambda,ssm};` lines from
   `src/schema/aws/mod.rs` and delete those stub directories.

The stubs are currently unreachable (nothing in the roots references them), so
there is no GraphQL type-name clash today — but deleting them removes the dead
code and prevents confusion over which `Bucket`/`Function`/`Parameter` is real.

## Phased rollout

Wire and verify in small batches; run `cargo check` after each batch with the
batch's features enabled. Phase 1 makes the **default build** (`ec2 s3 lambda`)
fully real, which is the highest-value milestone.

### Phase 1 — finish the default build  *(do first)*
`s3`, `lambda` (both default features), plus `ssm` and `vpc` (free — `vpc` rides
on `ec2`). Includes removing the `aws/{s3,lambda,ssm}` stubs.
After this, `cargo run -- query …` works with no extra `--features`.

### Phase 2 — compute & containers
`ecs`, `eks`, `ecr`, `batch`, `elbv2`, `asg` (autoscaling).

### Phase 3 — storage & data
`dynamodb`, `rds`, `efs`, `elasticache`, `redshift`, `redshift_serverless`,
`memorydb`, `neptune`, `documentdb`, `athena`, `glue`, `emr`, `kinesis`,
`firehose`, `msk`.

### Phase 4 — networking & edge
`route53`, `cloudfront`, `apigateway`, `apigatewayv2`, `global_accelerator`,
`direct_connect`, `network_firewall`.

### Phase 5 — security & identity
`iam`, `kms`, `secrets_manager`, `acm`, `cognito`, `guardduty`, `inspector`,
`security_hub`, `macie`, `shield`, `wafv2`, `sts`.

### Phase 6 — ops, management & misc
`cloudwatch` (+logs), `cloudtrail`, `config_svc`, `cloudformation`,
`codepipeline`, `codebuild`, `codedeploy`, `step_functions`, `eventbridge`,
`sns`, `sqs`, `service_quotas`, `health`, `organizations`, `appconfig`,
`appsync`, `cost_explorer`, `sagemaker`, `transfer`.

## Verification

Because the code is feature-gated, you must build with a service's feature to
exercise it:

```bash
# after each batch — compile with the batch's features
cargo check --features "ssm,dynamodb,rds,iam"

# live smoke test against an account with credentials
cargo run --features "iam" -- query '{ users { userName arn } }'

# whole surface at once
cargo run --features "<all enabled features>" -- serve   # GraphiQL at :4000
```

A batch is done when `cargo check` is clean for its features and a representative
live query returns data. Per the project's `CLAUDE.md`, default to `cargo check`;
leave `cargo build`/`test` to the developer.

## Cleanup (after the rollout)

Once services are wired, delete the dead scaffolding the inventory surfaced.
These produce the bulk of the current `never used` warnings and no longer serve a
purpose:

- `src/schema/modular_schema.rs` — abandoned placeholder roots.
- `src/schema/service.rs`, `src/schema/feature_config.rs` — unused "enabled
  services" abstractions superseded by Cargo features.
- `src/schema/aws/{ec2,s3,lambda,ssm}/` — stub modules; remove their declarations
  from `src/schema/aws/mod.rs`.
- `src/schema/aws/registry.rs`'s `ServiceRegistry` shim and
  `src/schema/aws/test_composition.rs` — once nothing depends on them, drop both
  (or rewrite the test to use `build_schema`).
- `src/schema/ec2/query_module.rs` — orphaned, references a non-existent
  `base_module`; never declared.
- `build_default_schema` in `src/schema/root.rs` — kept only for transitional API
  compatibility.

## Risks & watch-items

- **Field-name collisions (future):** none today, but a *newly authored* resolver
  or a future mutation could clash. `Schema::build` panics at startup on a
  duplicate field; a fast `cargo run -- query '{ placeholder }'` after each batch
  catches it immediately.
- **Forgetting step 3:** a registered field with no registered client compiles
  fine but fails at request time with "context data not found". The live smoke
  test is what catches this; `cargo check` will not.
- **Feature/module name drift:** use the master table verbatim — `config_svc`,
  `asg`, `msk`, `step_functions`, `cognito`, `documentdb`, `inspector`, `macie`
  and the `wafv2`/`WafV2` casing are the easy ones to get wrong.
- **`--no-default-features` builds:** only safe if every `pub mod <svc>;` and root
  field is feature-gated. Keep the gating consistent across all three edit sites.

## Progress checklist

Phase 1 — default build
- [x] `s3` (remove `aws/s3` stub)
- [x] `lambda` (remove `aws/lambda` stub)
- [x] `ssm` (remove `aws/ssm` stub)
- [x] `vpc` (gate on `ec2`, reuse `Ec2Client`)

Phase 2 — compute & containers
- [x] `ecs` · [x] `eks` · [x] `ecr` · [x] `batch` · [x] `elbv2` · [x] `asg`

Phase 3 — storage & data
- [x] `dynamodb` · [x] `rds` · [x] `efs` · [x] `elasticache` · [x] `redshift`
- [x] `redshift_serverless` · [x] `memorydb` · [x] `neptune` · [x] `documentdb`
- [x] `athena` · [x] `glue` · [x] `emr` · [x] `kinesis` · [x] `firehose` · [x] `msk`

Phase 4 — networking & edge
- [x] `route53` · [x] `cloudfront` · [x] `apigateway` · [x] `apigatewayv2`
- [x] `global_accelerator` · [x] `direct_connect` · [x] `network_firewall`

Phase 5 — security & identity
- [x] `iam` · [x] `kms` · [x] `secrets_manager` · [x] `acm` · [x] `cognito`
- [x] `guardduty` · [x] `inspector` · [x] `security_hub` · [x] `macie`
- [x] `shield` · [x] `wafv2` · [x] `sts`

Phase 6 — ops, management & misc
- [x] `cloudwatch` (+`cloudwatchlogs`) · [x] `cloudtrail` · [x] `config_svc`
- [x] `cloudformation` · [x] `codepipeline` · [x] `codebuild` · [x] `codedeploy`
- [x] `step_functions` · [x] `eventbridge` · [x] `sns` · [x] `sqs`
- [x] `service_quotas` · [x] `health` · [x] `organizations` · [x] `appconfig`
- [x] `appsync` · [x] `cost_explorer` · [x] `sagemaker` · [x] `transfer`
