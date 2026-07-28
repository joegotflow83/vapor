# vapor

A GraphQL interface over AWS APIs. Query your AWS infrastructure using GraphQL — either as a one-shot CLI command or as a persistent HTTP server with an interactive playground.

## Features

- **Query mode** — execute a GraphQL query from the command line and get JSON output
- **Server mode** — run a local GraphQL HTTP server with a built-in GraphiQL playground
- **100+ AWS services** — comprehensive coverage across compute, storage, networking, security, AI/ML, analytics, and more
- **EC2 mutations** — start, stop, reboot, terminate, and launch instances
- **Filtering & pagination** — filter by IDs, tags, state, and more; automatically pages all results
- **Standard AWS auth** — uses the AWS SDK default credential chain (env vars, `~/.aws/credentials`, IAM roles, etc.)
- **Feature flags** — compile only the services you need to keep binary size and build time small

## Installation

```bash
cargo build --release
# Binary will be at ./target/release/vapor
```

To build with specific services only:
```bash
cargo build --release --features "ec2 s3 lambda rds"
```

To build with all services:
```bash
cargo build --release --all-features
```

Prebuilt binaries on the [releases page](../../releases) are compiled with the curated `release`
feature group (~26 commonly used services — see [FEATURE_FLAGS.md](FEATURE_FLAGS.md)) rather than
`--all-features`, to keep CI build times reasonable. If you rely on a service outside that set,
build your own binary. `scripts/detect-aws-services.sh` scans your AWS account for services
actually in use and prints a ready-to-run `cargo build` command with the matching features:
```bash
./scripts/detect-aws-services.sh
```

Full build documentation — toolchain prerequisites, choosing features, build cost and OOM
avoidance, cross-compiling, verification, and a step-by-step procedure for AI agents — is in
[BUILDING.md](BUILDING.md).

## Prerequisites

Valid AWS credentials must be available via one of the standard mechanisms:

- Environment variables: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`
- AWS credentials file: `~/.aws/credentials`
- IAM instance profile / ECS task role / IRSA (when running on AWS)
- SSO: `aws sso login --profile <profile>` then `AWS_PROFILE=<profile>`

## Usage

### Query mode

Execute a single GraphQL query and print the result to stdout.

```
vapor query <QUERY> [--region <REGION>] [--format json|compact]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--region` | AWS default | AWS region to target |
| `--format` | `json` | `json` = pretty-printed, `compact` = single-line |

Exit code is non-zero if the GraphQL response contains errors.

### Server mode

Start a GraphQL HTTP server. The GraphiQL interactive playground is available at `http://localhost:<port>/`. The GraphQL endpoint is at `/graphql`.

```
vapor serve [--port <PORT>] [--region <REGION>] [--bind <ADDRESS>] [--auth-token <TOKEN>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--port` | `4000` | TCP port to listen on |
| `--region` | AWS default | AWS region to target |
| `--bind` | `127.0.0.1` | Address to bind to. Non-loopback binds require `--auth-token` (see below) |
| `--auth-token` | none | Bearer token required on every request. Also settable via `VAPOR_AUTH_TOKEN` (not shown in `--help`/errors) |

#### Authentication policy

The server executes AWS-mutating GraphQL operations (e.g. `terminateInstances`, `runInstances`) with its own AWS credentials, so binding it off-loopback without a token is refused at startup:

| Bind | Token set | Behavior |
|------|-----------|----------|
| loopback (`127.0.0.1`, `::1`, `localhost`, ...) | no | Serves openly (default, unchanged local-dev behavior) |
| loopback | yes | Every request must present the token |
| non-loopback (e.g. `0.0.0.0`) | no | Refuses to start (`exit 1`) |
| non-loopback | yes | Every request must present the token |

When a token is set, requests must send it as `Authorization: Bearer <token>` (case-insensitive scheme). Missing or incorrect tokens get a `401` with a `WWW-Authenticate: Bearer` header; the request never reaches the GraphQL executor. Bearer tokens are sent in plaintext over HTTP, so for non-loopback binds put a TLS-terminating reverse proxy in front.

```bash
VAPOR_AUTH_TOKEN=changeme vapor serve --bind 0.0.0.0
curl -H "Authorization: Bearer $VAPOR_AUTH_TOKEN" http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ s3Buckets { items { name } nextToken } }"}'
```

## GraphQL Schema

The full GraphQL query reference for all 102 services — one page per
service, field names, arguments, and types — is generated directly from the
live schema (so it can never drift the way hand-written docs do) and
published at **https://joegotflow83.github.io/vapor/**. It's rebuilt by
`.github/workflows/docs.yml` on every push to `master` via
`cargo run --all-features --bin gen-docs` (see `src/bin/gen_docs.rs`).

> Upgrading from 0.2.x? Every list query now returns a page (`items` +
> `nextToken`) instead of a bare list, and date/time fields are now the
> `DateTime` scalar instead of `String` — see [MIGRATION.md](MIGRATION.md).

## Examples

A handful of representative queries below; every one is quoted verbatim from a curated
file under `docs/examples/` that the `gen-docs` build validates against the live schema —
see the [full query reference](https://joegotflow83.github.io/vapor/) for all 102 services.

### CLI query examples

**Plain list — S3 buckets:**
```bash
vapor query '{ s3Buckets { items { name region versioning } nextToken } }'
```

**Filtered list — running EC2 instances:**
```bash
vapor query '{ instances(state: RUNNING) { items { id instanceType privateIp tags { key value } } nextToken } }'
```

**Non-paginated singleton — caller identity:**
```bash
vapor query '{ stsCallerIdentity { account userId arn } }'
```

**Nested selection — CloudWatch alarms with their metric:**
```bash
vapor query '{ alarms(state: ALARM) { items { name state metric { metricName } } nextToken } }'
```

### Mutation example

**Stop instances:**
```bash
vapor query 'mutation { stopInstances(ids: ["i-0abc123"]) { instanceId previousState currentState } }'
```

### Server mode

**Start the server on the default port:**
```bash
vapor serve
# GraphiQL playground: http://localhost:4000/
# GraphQL endpoint:    http://localhost:4000/graphql
```

**Query the server with curl:**
```bash
curl -s http://localhost:4000/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"{ stsCallerIdentity { account userId arn } }"}' \
  | jq .
```

## Project Structure

```
vapor/
├── src/
│   ├── main.rs           # CLI entry point (clap, query/serve subcommands)
│   ├── server.rs         # Axum HTTP server + GraphiQL handler
│   ├── error.rs          # VaporError type
│   ├── aws/              # AWS SDK client wrappers (one file per service)
│   │   ├── mod.rs
│   │   ├── config.rs     # AWS SDK config loader (region, retry, timeouts)
│   │   ├── ec2.rs
│   │   ├── eks.rs
│   │   ├── ecs.rs
│   │   ├── lambda.rs
│   │   ├── s3.rs
│   │   ├── rds.rs
│   │   └── ...           # 100+ service modules
│   └── schema/           # GraphQL types and resolvers (one directory per service)
│       ├── mod.rs
│       ├── root.rs       # Schema assembly (QueryRoot, MutationRoot)
│       ├── aws/          # Schema registry
│       ├── ec2/          # types.rs, queries.rs, mutations.rs
│       ├── eks/
│       ├── ecs/
│       ├── lambda/
│       ├── s3/
│       ├── rds/
│       └── ...           # 100+ service schema modules
└── Cargo.toml
```

## Feature Flags

Each service is gated behind a Cargo feature flag. Build only what you need:

| Feature | Service |
|---------|---------|
| `acm` | Certificate Manager |
| `acmpca` | ACM Private CA |
| `apigateway` | API Gateway (REST) |
| `apigatewayv2` | API Gateway v2 (HTTP/WebSocket) |
| `appconfig` | AppConfig |
| `apprunner` | App Runner |
| `appsync` | AppSync |
| `athena` | Athena |
| `auditmanager` | Audit Manager |
| `autoscaling` | Auto Scaling |
| `backup` | Backup |
| `batch` | Batch |
| `bedrock` | Bedrock |
| `budgets` | Budgets |
| `cloudformation` | CloudFormation |
| `cloudfront` | CloudFront |
| `cloudtrail` | CloudTrail |
| `cloudwatch` | CloudWatch + Logs |
| `codeartifact` | CodeArtifact |
| `codebuild` | CodeBuild |
| `codecommit` | CodeCommit |
| `codedeploy` | CodeDeploy |
| `codepipeline` | CodePipeline |
| `cognitoidentityprovider` | Cognito |
| `comprehend` | Comprehend |
| `config` | AWS Config |
| `connect` | Connect |
| `controltower` | Control Tower |
| `costexplorer` | Cost Explorer |
| `datasync` | DataSync |
| `detective` | Detective |
| `directconnect` | Direct Connect |
| `dms` | Database Migration Service |
| `docdb` | DocumentDB |
| `dynamodb` | DynamoDB |
| `ec2` | EC2 + VPC resources |
| `ecr` | ECR |
| `ecs` | ECS |
| `efs` | EFS |
| `eks` | EKS |
| `elasticache` | ElastiCache |
| `elasticbeanstalk` | Elastic Beanstalk |
| `elbv2` | ELB v2 |
| `emr` | EMR |
| `eventbridge` | EventBridge |
| `firehose` | Firehose |
| `fms` | Firewall Manager |
| `fsx` | FSx |
| `globalaccelerator` | Global Accelerator |
| `glue` | Glue |
| `guardduty` | GuardDuty |
| `health` | Health |
| `iam` | IAM |
| `inspector2` | Inspector v2 |
| `iot` | IoT |
| `kafka` | MSK (Managed Kafka) — pulls in `ec2` |
| `keyspaces` | Keyspaces |
| `kinesis` | Kinesis |
| `kms` | KMS |
| `lakeformation` | Lake Formation |
| `lambda` | Lambda |
| `licensemanager` | License Manager |
| `lightsail` | Lightsail |
| `macie2` | Macie |
| `memorydb` | MemoryDB |
| `mq` | Amazon MQ |
| `neptune` | Neptune |
| `networkfirewall` | Network Firewall |
| `opensearch` | OpenSearch |
| `organizations` | Organizations |
| `pinpoint` | Pinpoint |
| `polly` | Polly |
| `qldb` | QLDB |
| `quicksight` | QuickSight |
| `ram` | Resource Access Manager |
| `rds` | RDS |
| `redshift` | Redshift |
| `redshiftserverless` | Redshift Serverless |
| `rekognition` | Rekognition |
| `route53` | Route 53 |
| `s3` | S3 |
| `sagemaker` | SageMaker |
| `secretsmanager` | Secrets Manager |
| `securityhub` | Security Hub |
| `servicequotas` | Service Quotas |
| `sesv2` | SES v2 |
| `shield` | Shield |
| `sfn` | Step Functions |
| `sns` | SNS |
| `sqs` | SQS |
| `ssm` | Systems Manager |
| `ssoadmin` | SSO Admin |
| `storagegateway` | Storage Gateway |
| `sts` | STS |
| `timestream` | Timestream |
| `transcribe` | Transcribe |
| `transfer` | Transfer Family |
| `translate` | Translate |
| `wafv2` | WAF v2 |
| `workspaces` | WorkSpaces |
| `xray` | X-Ray |

**Feature groups:**

| Group | Includes |
|-------|---------|
| `basic` | ec2, s3, lambda, ssm |
| `web` | ec2, elbv2, s3, lambda, apigateway |
| `data` | s3, dynamodb, redshift, athena, glue |
| `monitoring` | cloudwatch, config, inspector2, securityhub |
| `devops` | codepipeline, codebuild, codedeploy, cloudformation |
| `release` | ec2, s3, lambda, iam, rds, dynamodb, cloudwatch, cloudwatchlogs, sqs, sns, kms, secretsmanager, ecs, ecr, eks, elbv2, autoscaling, route53, cloudfront, cloudformation, sts, apigateway, apigatewayv2, eventbridge, ssm, kinesis — used for prebuilt GitHub releases |

## Configuration

AWS SDK configuration is loaded via the standard credential chain. Retry is configured at 3 attempts with a 30-second per-attempt timeout.

To use a named AWS profile:
```bash
AWS_PROFILE=my-profile vapor serve
```

To use explicit credentials:
```bash
AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... vapor query '{ s3Buckets { items { name region versioning } nextToken } }'
```
