# vapor

A GraphQL interface over AWS APIs. Query your AWS infrastructure using GraphQL — either as a one-shot CLI command or as a persistent HTTP server with an interactive playground.

## Features

- **Query mode** — execute a GraphQL query from the command line and get JSON output
- **Server mode** — run a local GraphQL HTTP server with a built-in GraphiQL playground
- **25 AWS services** — EC2, EKS, ECS, Lambda, S3, RDS, DynamoDB, IAM, KMS, CloudWatch, and more
- **EC2 mutations** — start, stop, reboot, terminate, and launch instances
- **Filtering & pagination** — filter by IDs, tags, state, and more; automatically pages all results
- **Standard AWS auth** — uses the AWS SDK default credential chain (env vars, `~/.aws/credentials`, IAM roles, etc.)

## Installation

```bash
cargo build --release
# Binary will be at ./target/release/vapor
```

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
vapor serve [--port <PORT>] [--region <REGION>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--port` | `4000` | TCP port to listen on |
| `--region` | AWS default | AWS region to target |

## GraphQL Schema

### EC2

```graphql
instances(ids: [String], state: InstanceState, vpcId: String, subnetId: String, tags: [TagFilter]): [Instance!]!
securityGroups(ids: [String], vpcId: String, name: String): [SecurityGroup!]!
vpcs(ids: [String]): [Vpc!]!
subnets(ids: [String], vpcId: String, az: String): [Subnet!]!
volumes(ids: [String], state: String): [Volume!]!
keyPairs(ids: [String], name: String, fingerprint: String): [KeyPair!]!
elasticIps(allocationIds: [String], publicIps: [String], instanceId: String): [ElasticIp!]!
images(ids: [String], owners: [String], name: String, state: String, tags: [TagFilter]): [Image!]!
```

**Mutations:**
```graphql
startInstances(ids: [String!]!): [InstanceStateChange!]!
stopInstances(ids: [String!]!, force: Boolean): [InstanceStateChange!]!
terminateInstances(ids: [String!]!): [InstanceStateChange!]!
rebootInstances(ids: [String!]!): Boolean!
runInstances(input: RunInstancesInput!): [Instance!]!
```

`InstanceState` enum: `PENDING`, `RUNNING`, `SHUTTING_DOWN`, `TERMINATED`, `STOPPING`, `STOPPED`

### EKS

```graphql
eksCluster(name: String!): EksCluster
eksClusters(clusterNames: [String]): [EksCluster!]!
eksNodegroups(cluster: String!, nodegroupNames: [String]): [EksNodegroup!]!
eksFargateProfiles(cluster: String!): [EksFargateProfile!]!
eksAddons(cluster: String!): [EksAddon!]!
```

### ECS

```graphql
ecsClusters(clusterArns: [String]): [Cluster!]!
ecsServices(cluster: String!, serviceArns: [String]): [Service!]!
ecsTasks(cluster: String!, serviceArn: String, desiredStatus: String): [Task!]!
ecsTaskDefinition(taskDefinition: String!): TaskDefinition
ecsTaskDefinitions(familyPrefix: String, status: String): [String!]!
```

### Lambda

```graphql
lambdaFunctions: [LambdaFunction!]!
lambdaAliases(functionName: String!): [LambdaAlias!]!
lambdaEventSourceMappings(functionName: String): [LambdaEventSourceMapping!]!
lambdaLayers: [LambdaLayer!]!
```

### S3

```graphql
s3Buckets: [S3Bucket!]!
s3Bucket(name: String!): S3Bucket
```

### RDS

```graphql
dbInstances(ids: [String]): [DbInstance!]!
dbClusters(ids: [String]): [DbCluster!]!
dbSnapshots(dbInstanceId: String, snapshotType: String): [DbSnapshot!]!
```

### DynamoDB

```graphql
dynamoTables: [String!]!
dynamoTable(name: String!): DynamoTable
dynamoScan(table: String!, filterExpression: String, limit: Int): DynamoScanResult!
```

### IAM

```graphql
iamRoles(pathPrefix: String): [IamRole!]!
iamPolicies(scope: String, pathPrefix: String): [IamPolicy!]!
iamUsers(pathPrefix: String): [IamUser!]!
iamGroups(pathPrefix: String): [IamGroup!]!
iamAttachedRolePolicies(roleName: String!): [IamAttachedPolicy!]!
```

`iamPolicies` defaults `scope` to `"Local"` (customer-managed only). Use `"All"` for AWS-managed policies too.

### KMS

```graphql
kmsKeys: [KmsKey!]!
kmsAliases(keyId: String): [KmsAlias!]!
kmsKeyPolicyNames(keyId: String!): [String!]!
kmsKeyPolicy(keyId: String!, policyName: String!): KmsKeyPolicy
```

### Secrets Manager

```graphql
secretsList: [Secret!]!
secretDescribe(secretId: String!): Secret
secretValue(secretId: String!): SecretValue
```

### CloudWatch & Logs

```graphql
metrics(namespace: String, metricName: String, dimensions: [DimensionFilter]): [Metric!]!
metricData(queries: [MetricDataQuery!]!, timeRange: TimeRange!): [MetricResult!]!
alarms(names: [String], namePrefix: String, state: AlarmState): [Alarm!]!
logGroups(prefix: String): [LogGroup!]!
logStreams(logGroupName: String!, prefix: String, orderBy: String): [LogStream!]!
logEvents(logGroupName: String!, logStreamName: String, filterPattern: String, timeRange: TimeRange, limit: Int): [LogEvent!]!
```

### API Gateway

```graphql
apigwRestApis: [ApigwRestApi!]!
apigwRestStages(apiId: String!): [ApigwRestStage!]!
apigwRestResources(apiId: String!): [ApigwResource!]!
apigwRestDeployments(apiId: String!): [ApigwDeployment!]!
apigwHttpApis: [ApigwHttpApi!]!
apigwHttpStages(apiId: String!): [ApigwHttpStage!]!
apigwHttpRoutes(apiId: String!): [ApigwHttpRoute!]!
```

### ELB (v2)

```graphql
loadBalancers(arns: [String], names: [String]): [LoadBalancer!]!
targetGroups(arns: [String], loadBalancerArn: String): [TargetGroup!]!
targetHealth(targetGroupArn: String!): [TargetHealthInfo!]!
listeners(loadBalancerArn: String!): [Listener!]!
```

### Auto Scaling

```graphql
autoScalingGroups(names: [String]): [AutoScalingGroup!]!
scalingActivities(autoScalingGroupName: String): [ScalingActivity!]!
```

### Route 53

```graphql
r53HostedZones: [R53HostedZone!]!
r53Records(hostedZoneId: String!): [R53ResourceRecordSet!]!
r53HealthChecks: [R53HealthCheck!]!
```

### OpenSearch

```graphql
opensearchDomains: [OpenSearchDomain!]!
opensearchDomain(domainName: String!): OpenSearchDomain
opensearchDomainTags(arn: String!): [Tag!]!
```

### SSM

```graphql
managedInstances(instanceIds: [String], pingStatus: PingStatus, platformType: PlatformType): [ManagedInstance!]!
parameters(names: [String!]!, withDecryption: Boolean): [Parameter!]!
parametersByPath(path: String!, recursive: Boolean, withDecryption: Boolean): [Parameter!]!
parameterMetadata(filters: [ParameterFilter]): [ParameterMeta!]!
documents(owner: String, documentType: String, name: String): [SsmDocument!]!
```

### SNS

```graphql
snsTopics: [SnsTopic!]!
snsTopic(topicArn: String!): SnsTopic
snsSubscriptions(topicArn: String): [SnsSubscription!]!
```

### SQS

```graphql
sqsQueues(prefix: String): [String!]!
sqsQueue(queueUrl: String!): SqsQueue
```

### ACM

```graphql
acmCertificates(statuses: [String]): [AcmCertificate!]!
acmCertificate(arn: String!): AcmCertificate
```

### CloudFormation

```graphql
cfnStacks(names: [String], statusFilter: [String]): [CfnStack!]!
cfnStackResources(stackName: String!): [CfnStackResource!]!
```

### CloudFront

```graphql
cloudfrontDistributions: [CfDistribution!]!
cloudfrontDistribution(id: String!): CfDistribution
```

### ECR

```graphql
ecrRepositories(names: [String]): [EcrRepository!]!
ecrImages(repositoryName: String!, imageTags: [String], imageDigests: [String]): [EcrImage!]!
```

### ElastiCache

```graphql
elasticacheClusters(clusterId: String): [ElastiCacheCluster!]!
elasticacheReplicationGroups(replicationGroupId: String): [ElastiCacheReplicationGroup!]!
elasticacheSubnetGroups: [ElastiCacheSubnetGroup!]!
```

### EventBridge

```graphql
eventBridgeBuses: [EbEventBus!]!
eventBridgeRules(eventBusName: String): [EbRule!]!
eventBridgeTargets(ruleName: String!, eventBusName: String): [EbTarget!]!
```

## Examples

### CLI query examples

**List all running instances:**
```bash
vapor query '{ instances(state: RUNNING) { id instanceType privateIp tags { key value } } }'
```

**List Lambda functions:**
```bash
vapor query '{ lambdaFunctions { functionName runtime memorySize state } }'
```

**List EKS clusters:**
```bash
vapor query '{ eksClusters { name status version endpoint } }'
```

**List S3 buckets with versioning:**
```bash
vapor query '{ s3Buckets { name region versioning } }'
```

**List RDS instances:**
```bash
vapor query '{ dbInstances { dbInstanceIdentifier engine dbInstanceStatus multiAz } }'
```

**Get CloudWatch alarms in ALARM state:**
```bash
vapor query '{ alarms(state: ALARM) { alarmName stateValue metricName } }'
```

**Tail CloudWatch log group:**
```bash
vapor query '{ logGroups(prefix: "/aws/lambda") { name retentionInDays } }'
```

**List IAM roles:**
```bash
vapor query '{ iamRoles { roleName arn createDate } }'
```

**List ACM certificates:**
```bash
vapor query '{ acmCertificates(statuses: ["ISSUED"]) { domainName status notAfter } }'
```

**List SQS queues:**
```bash
vapor query '{ sqsQueues }'
```

**List ECS clusters and services:**
```bash
vapor query '{ ecsClusters { clusterName status activeServicesCount } }'
```

**Target a specific region:**
```bash
vapor query '{ instances(state: RUNNING) { id privateIp } }' --region eu-west-1
```

**Compact output for piping to jq:**
```bash
vapor query '{ lambdaFunctions { functionName runtime } }' --format compact | jq '.data.lambdaFunctions[].functionName'
```

### Mutation examples

**Stop instances:**
```bash
vapor query 'mutation { stopInstances(ids: ["i-0abc123"]) { instanceId previousState currentState } }'
```

**Start instances:**
```bash
vapor query 'mutation { startInstances(ids: ["i-0abc123"]) { instanceId previousState currentState } }'
```

**Terminate instances:**
```bash
vapor query 'mutation { terminateInstances(ids: ["i-0abc123"]) { instanceId currentState } }'
```

**Launch a new instance:**
```bash
vapor query 'mutation {
  runInstances(input: {
    imageId: "ami-0abcdef1234567890"
    instanceType: "t3.micro"
    minCount: 1
    maxCount: 1
    keyName: "my-key"
    subnetId: "subnet-0abc123"
    securityGroupIds: ["sg-0abc123"]
    tags: [{ key: "Name", value: "my-new-instance" }, { key: "Environment", value: "dev" }]
  }) {
    id instanceType state privateIp
  }
}'
```

### Server mode

**Start the server on the default port:**
```bash
vapor serve
# GraphiQL playground: http://localhost:4000/
# GraphQL endpoint:    http://localhost:4000/graphql
```

**Start on a custom port targeting a specific region:**
```bash
vapor serve --port 8080 --region us-west-2
```

**Query the server with curl:**
```bash
curl -s http://localhost:4000/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"{ instances(state: RUNNING) { id instanceType privateIp } }"}' \
  | jq .
```

## Project Structure

```
vapor/
├── src/
│   ├── main.rs           # CLI entry point (clap, query/serve subcommands)
│   ├── server.rs         # Axum HTTP server + GraphiQL handler
│   ├── error.rs          # VaporError type
│   ├── aws/
│   │   ├── mod.rs
│   │   ├── config.rs     # AWS SDK config loader (region, retry, timeouts)
│   │   ├── ec2.rs
│   │   ├── eks.rs
│   │   ├── ecs.rs
│   │   ├── lambda.rs
│   │   ├── s3.rs
│   │   ├── rds.rs
│   │   ├── dynamodb.rs
│   │   ├── iam.rs
│   │   ├── kms.rs
│   │   ├── cloudwatch.rs
│   │   ├── cloudwatch_logs.rs
│   │   ├── apigateway.rs
│   │   ├── elbv2.rs
│   │   ├── autoscaling.rs
│   │   ├── route53.rs
│   │   ├── opensearch.rs
│   │   ├── ssm.rs
│   │   ├── sns.rs
│   │   ├── sqs.rs
│   │   ├── secrets_manager.rs
│   │   ├── acm.rs
│   │   ├── cloudformation.rs
│   │   ├── cloudfront.rs
│   │   ├── ecr.rs
│   │   ├── elasticache.rs
│   │   └── eventbridge.rs
│   └── schema/
│       ├── mod.rs
│       ├── root.rs       # Schema assembly (QueryRoot, MutationRoot)
│       ├── ec2/          # types.rs, queries.rs, mutations.rs
│       ├── eks/
│       ├── ecs/
│       ├── lambda/
│       ├── s3/
│       ├── rds/
│       ├── dynamodb/
│       ├── iam/
│       ├── kms/
│       ├── cloudwatch/
│       ├── apigateway/
│       ├── elbv2/
│       ├── asg/
│       ├── route53/
│       ├── opensearch/
│       ├── ssm/
│       ├── sns/
│       ├── sqs/
│       ├── secrets_manager/
│       ├── acm/
│       ├── cloudformation/
│       ├── cloudfront/
│       ├── ecr/
│       ├── elasticache/
│       └── eventbridge/
└── Cargo.toml
```

## Configuration

AWS SDK configuration is loaded via the standard credential chain. Retry is configured at 3 attempts with a 30-second per-attempt timeout.

To use a named AWS profile:
```bash
AWS_PROFILE=my-profile vapor serve
```

To use explicit credentials:
```bash
AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... vapor query '{ s3Buckets { name } }'
```
