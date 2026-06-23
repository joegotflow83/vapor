# Feature Flags Documentation

This document describes how to use the feature flags system in Vapor to enable/disable AWS services at build time.

## Available Features

### Individual Service Features
Each AWS service has its own feature flag that can be enabled/disabled:

- `ec2` - EC2 service
- `s3` - S3 service  
- `lambda` - Lambda service
- `ssm` - Systems Manager service
- `cloudwatch` - CloudWatch service
- `cloudwatchlogs` - CloudWatch Logs service
- `autoscaling` - Auto Scaling service
- `elbv2` - Elastic Load Balancing v2 service
- `rds` - RDS service
- `ecs` - ECS service
- `kms` - KMS service
- `sqs` - SQS service
- `sns` - SNS service
- `secretsmanager` - Secrets Manager service
- `route53` - Route 53 service
- `dynamodb` - DynamoDB service
- `opensearch` - OpenSearch service
- `eks` - EKS service
- `acm` - ACM service
- `iam` - IAM service
- `ecr` - ECR service
- `cloudfront` - CloudFront service
- `elasticache` - ElastiCache service
- `cloudformation` - CloudFormation service
- `eventbridge` - EventBridge service
- `guardduty` - GuardDuty service
- `config` - AWS Config service
- `codepipeline` - CodePipeline service
- `codebuild` - CodeBuild service
- `codedeploy` - CodeDeploy service
- `kinesis` - Kinesis service
- `firehose` - Firehose service
- `kafka` - Kafka service
- `redshift` - Redshift service
- `redshiftserverless` - Redshift Serverless service
- `memorydb` - MemoryDB service
- `sfn` - Step Functions service
- `costexplorer` - Cost Explorer service
- `sts` - STS service
- `cloudtrail` - CloudTrail service
- `wafv2` - WAF v2 service
- `glue` - Glue service
- `efs` - EFS service
- `athena` - Athena service
- `organizations` - Organizations service
- `cognitoidentityprovider` - Cognito Identity Provider service
- `appsync` - AppSync service
- `backup` - Backup service
- `servicequotas` - Service Quotas service
- `health` - Health service
- `inspector2` - Inspector v2 service
- `securityhub` - Security Hub service
- `neptune` - Neptune service
- `docdb` - DocumentDB service
- `emr` - EMR service
- `sagemaker` - SageMaker service
- `batch` - Batch service
- `apigateway` - API Gateway service
- `apigatewayv2` - API Gateway v2 service
- `transfer` - Transfer service
- `appconfig` - AppConfig service
- `globalaccelerator` - Global Accelerator service
- `directconnect` - Direct Connect service
- `macie2` - Macie v2 service
- `networkfirewall` - Network Firewall service
- `shield` - Shield service

### Feature Groups
Predefined feature groups for common combinations:

- `basic` - Basic AWS services: ec2, s3, lambda, ssm
- `web` - Web services: ec2, elbv2, s3, lambda, apigateway
- `data` - Data services: s3, dynamodb, redshift, athena, glue
- `monitoring` - Monitoring and management: cloudwatch, cloudwatchlogs, config, inspector2, securityhub
- `devops` - DevOps services: codepipeline, codebuild, codedeploy, cloudformation

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