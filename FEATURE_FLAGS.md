# Feature Flags Documentation

This document describes how to use the feature flags system in Vapor to enable/disable AWS services at build time.

This is the reference list of flags. For the surrounding build process — prerequisites, build
cost, cross-compiling, verification — see [BUILDING.md](BUILDING.md).

## Available Features

### Individual Service Features
Each AWS service has its own feature flag that can be enabled/disabled:

- `acm` - Certificate Manager
- `acmpca` - ACM Private CA
- `apigateway` - API Gateway (REST)
- `apigatewayv2` - API Gateway v2 (HTTP/WebSocket)
- `appconfig` - AppConfig
- `apprunner` - App Runner
- `appsync` - AppSync
- `athena` - Athena
- `auditmanager` - Audit Manager
- `autoscaling` - Auto Scaling
- `backup` - Backup
- `batch` - Batch
- `bedrock` - Bedrock
- `budgets` - Budgets
- `cloudformation` - CloudFormation
- `cloudfront` - CloudFront
- `cloudtrail` - CloudTrail
- `cloudwatch` - CloudWatch (includes Logs)
- `cloudwatchlogs` - CloudWatch Logs (pulled in by `cloudwatch`)
- `codeartifact` - CodeArtifact
- `codebuild` - CodeBuild
- `codecommit` - CodeCommit
- `codedeploy` - CodeDeploy
- `codepipeline` - CodePipeline
- `cognitoidentityprovider` - Cognito Identity Provider
- `comprehend` - Comprehend
- `config` - AWS Config
- `connect` - Connect
- `controltower` - Control Tower
- `costexplorer` - Cost Explorer
- `datasync` - DataSync
- `detective` - Detective
- `directconnect` - Direct Connect
- `dms` - Database Migration Service
- `docdb` - DocumentDB
- `dynamodb` - DynamoDB
- `ec2` - EC2 and VPC resources
- `ecr` - ECR
- `ecs` - ECS
- `efs` - EFS
- `eks` - EKS
- `elasticache` - ElastiCache
- `elasticbeanstalk` - Elastic Beanstalk
- `elbv2` - Elastic Load Balancing v2
- `emr` - EMR
- `eventbridge` - EventBridge
- `firehose` - Kinesis Data Firehose
- `fms` - Firewall Manager
- `fsx` - FSx
- `globalaccelerator` - Global Accelerator
- `glue` - Glue
- `guardduty` - GuardDuty
- `health` - Health
- `iam` - IAM
- `inspector2` - Inspector v2
- `iot` - IoT Core
- `kafka` - MSK (Managed Streaming for Kafka); pulls in `ec2` since broker AZ enrichment needs `describe_subnets`
- `keyspaces` - Amazon Keyspaces
- `kinesis` - Kinesis Data Streams
- `kms` - KMS
- `lakeformation` - Lake Formation
- `lambda` - Lambda
- `licensemanager` - License Manager
- `lightsail` - Lightsail
- `macie2` - Macie v2
- `memorydb` - MemoryDB
- `mq` - Amazon MQ
- `neptune` - Neptune
- `networkfirewall` - Network Firewall
- `opensearch` - OpenSearch
- `organizations` - Organizations
- `pinpoint` - Pinpoint
- `polly` - Polly
- `qldb` - QLDB
- `quicksight` - QuickSight
- `ram` - Resource Access Manager
- `rds` - RDS
- `redshift` - Redshift
- `redshiftserverless` - Redshift Serverless
- `rekognition` - Rekognition
- `route53` - Route 53
- `s3` - S3
- `sagemaker` - SageMaker
- `secretsmanager` - Secrets Manager
- `securityhub` - Security Hub
- `servicequotas` - Service Quotas
- `sesv2` - Simple Email Service v2
- `sfn` - Step Functions
- `shield` - Shield
- `sns` - SNS
- `sqs` - SQS
- `ssm` - Systems Manager
- `ssoadmin` - IAM Identity Center (SSO Admin)
- `storagegateway` - Storage Gateway
- `sts` - STS
- `timestream` - Timestream
- `transcribe` - Transcribe
- `transfer` - Transfer Family
- `translate` - Translate
- `wafv2` - WAF v2
- `workspaces` - WorkSpaces
- `xray` - X-Ray

### Feature Groups
Predefined feature groups for common combinations:

- `standard` - Services that answer for any AWS account regardless of what is deployed in it:
  sts, iam, ec2, cloudwatch (and cloudwatchlogs), cloudtrail, costexplorer, budgets,
  servicequotas. None of these are resource-shaped, so `detect-aws-services.sh` cannot
  discover them — it assumes them instead. Deliberately excludes `health` (requires a
  Business/Enterprise support plan) and `organizations` (fails for standalone accounts).
- `basic` - Basic AWS services: ec2, s3, lambda, ssm
- `web` - Web services: ec2, elbv2, s3, lambda, apigateway
- `data` - Data services: s3, dynamodb, redshift, athena, glue
- `monitoring` - Monitoring and management: cloudwatch, cloudwatchlogs, config, inspector2, securityhub
- `devops` - DevOps services: codepipeline, codebuild, codedeploy, cloudformation
- `release` - Curated set of commonly used services, built for prebuilt GitHub releases:
  the `standard` group plus s3, lambda, rds, dynamodb, cloudwatchlogs, sqs, sns, kms,
  secretsmanager, ecs, ecr, eks, elbv2, autoscaling, route53, cloudfront, cloudformation,
  apigateway, apigatewayv2, eventbridge, ssm, kinesis

### Building for Your Own AWS Account

The prebuilt release binaries only include the `release` feature group. If you rely on a service
outside that set, run `./scripts/detect-aws-services.sh` to scan your account (via the Resource
Groups Tagging API) and print a `cargo build` command with only the features you actually need.

That scan reports **taggable resources**, so it has two blind spots. Untagged resources of an
otherwise-ordinary service will not surface, and account-level services have no resources to
find at all — the script assumes the `standard` group to cover the latter. Pass `--no-standard`
if you want a strictly detected build.

## Usage Examples

### Build with default features (ec2, s3, lambda)
```bash
cargo build
```

### Build with specific service features
```bash
cargo build --features "ec2 s3 lambda"
```

### Build with feature groups
```bash
cargo build --features "basic web"
```

### Build with a single service
```bash
cargo build --features "s3"
```

### Build with a custom combination
```bash
cargo build --features "ec2 rds dynamodb"
```

## Compilation Benefits

Using feature flags provides the following benefits:

1. **Reduced compilation time** - Only necessary AWS SDK dependencies are compiled
2. **Smaller binary size** - Unused services are not included in the final binary
3. **Reduced memory usage** - Less memory required during compilation 
4. **Faster builds** - Smaller codebase to compile