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

### ACM (Certificate Manager)

```graphql
acmCertificates(statuses: [String]): [AcmCertificate!]!
acmCertificate(arn: String!): AcmCertificate
```

### ACM Private CA

```graphql
privateCertificateAuthorities: [PrivateCa!]!
privateCertificateAuthority(certificateAuthorityArn: String!): PrivateCa
```

### API Gateway (REST)

```graphql
apigwRestApis: [ApigwRestApi!]!
apigwRestStages(apiId: String!): [ApigwRestStage!]!
apigwRestResources(apiId: String!): [ApigwResource!]!
apigwRestDeployments(apiId: String!): [ApigwDeployment!]!
apigwHttpApis: [ApigwHttpApi!]!
apigwHttpStages(apiId: String!): [ApigwHttpStage!]!
apigwHttpRoutes(apiId: String!): [ApigwHttpRoute!]!
```

### API Gateway v2 (HTTP & WebSocket)

```graphql
apiV2Apis: [ApiV2!]!
apiV2Stages(apiId: String!): [ApiV2Stage!]!
apiV2Routes(apiId: String!): [ApiV2Route!]!
apiV2DomainNames: [ApiV2DomainName!]!
apiV2VpcLinks: [ApiV2VpcLink!]!
```

### AppConfig

```graphql
appconfigApplications: [AppConfigApplication!]!
appconfigEnvironments(applicationId: String!): [AppConfigEnvironment!]!
appconfigProfiles(applicationId: String!): [AppConfigProfile!]!
```

### App Runner

```graphql
appRunnerServices: [AppRunnerService!]!
appRunnerService(serviceArn: String!): AppRunnerService
appRunnerVpcConnectors: [AppRunnerVpcConnector!]!
```

### AppSync

```graphql
appsyncApis: [AppSyncApi!]!
appsyncDataSources(apiId: String!): [AppSyncDataSource!]!
```

### Athena

```graphql
athenaWorkgroups: [AthenaWorkgroup!]!
athenaNamedQueries(workgroup: String): [AthenaNamedQuery!]!
athenaQueryExecutions(workgroup: String, maxResults: Int): [AthenaQueryExecution!]!
```

### Audit Manager

```graphql
auditManagerAssessments: [AuditManagerAssessment!]!
auditManagerFrameworks(frameworkType: String): [AuditManagerFramework!]!
auditManagerControls(controlType: String): [AuditManagerControl!]!
```

### Auto Scaling

```graphql
autoScalingGroups(names: [String]): [AutoScalingGroup!]!
scalingActivities(autoScalingGroupName: String): [ScalingActivity!]!
```

### Backup

```graphql
backupVaults: [BackupVault!]!
backupPlans: [BackupPlan!]!
backupRecoveryPoints(vaultName: String!): [RecoveryPoint!]!
```

### Batch

```graphql
batchJobQueues: [BatchJobQueue!]!
batchComputeEnvironments: [BatchComputeEnvironment!]!
batchJobDefinitions(status: String): [BatchJobDefinition!]!
```

### Bedrock

```graphql
bedrockFoundationModels(provider: String, byOutputModality: String, byInferenceType: String): [BedrockFoundationModel!]!
bedrockCustomModels: [BedrockCustomModel!]!
bedrockGuardrails: [BedrockGuardrail!]!
bedrockModelInvocationLoggingConfig: BedrockModelInvocationLoggingConfig
```

### Budgets

```graphql
budgets(accountId: String!): [Budget!]!
budgetNotifications(accountId: String!, budgetName: String!): [BudgetNotification!]!
```

### CloudFormation

```graphql
cfnStacks(names: [String], statusFilter: [String]): [CfnStack!]!
cfnStackResources(stackName: String!): [CfnStackResource!]!
cfnExports: [CfnExport!]!
```

### CloudFront

```graphql
cloudfrontDistributions: [CfDistribution!]!
cloudfrontDistribution(id: String!): CfDistribution
```

### CloudTrail

```graphql
cloudtrailTrails: [Trail!]!
cloudtrailEvents(startTime: String!, endTime: String!, eventName: String, username: String): [CloudTrailEvent!]!
```

### CloudWatch & Logs

```graphql
metrics(namespace: String, metricName: String, dimensions: [DimensionFilter]): [Metric!]!
metricData(queries: [MetricDataQuery!]!, timeRange: TimeRange!): [MetricResult!]!
alarms(names: [String], namePrefix: String, state: AlarmState): [Alarm!]!
logGroups(prefix: String): [LogGroup!]!
logStreams(logGroupName: String!, prefix: String, orderBy: String): [LogStream!]!
metricFilters(logGroupName: String): [MetricFilter!]!
logEvents(logGroupName: String!, logStreamName: String, filterPattern: String, timeRange: TimeRange, limit: Int): [LogEvent!]!
```

### CodeArtifact

```graphql
codeArtifactDomains: [CodeArtifactDomain!]!
codeArtifactRepositories(domain: String!, domainOwner: String): [CodeArtifactRepository!]!
codeArtifactPackages(domain: String!, repository: String!, format: String): [CodeArtifactPackage!]!
```

### CodeBuild

```graphql
buildProjects(names: [String]): [BuildProject!]!
builds(projectName: String!): [Build!]!
```

### CodeCommit

```graphql
codeCommitRepositories: [CodeCommitRepository!]!
codeCommitRepository(repositoryName: String!): CodeCommitRepository
codeCommitBranches(repositoryName: String!): [CodeCommitBranch!]!
codeCommitPullRequests(repositoryName: String!, pullRequestStatus: String): [CodeCommitPullRequest!]!
```

### CodeDeploy

```graphql
deployApplications: [DeployApplication!]!
deploymentGroups(applicationName: String!): [DeploymentGroup!]!
deployments(applicationName: String, deploymentGroupName: String): [Deployment!]!
```

### CodePipeline

```graphql
pipelines: [Pipeline!]!
pipelineExecutions(pipelineName: String!): [PipelineExecution!]!
pipelineState(pipelineName: String!): [StageState!]!
```

### Cognito

```graphql
cognitoUserPools: [UserPool!]!
cognitoUserPoolClients(userPoolId: String!): [UserPoolClient!]!
```

### Comprehend

```graphql
comprehendEntityRecognizers(statusFilter: String): [ComprehendEntityRecognizer!]!
comprehendDocumentClassifiers(statusFilter: String): [ComprehendDocumentClassifier!]!
comprehendEndpoints: [ComprehendEndpoint!]!
```

### Config

```graphql
configRules(names: [String]): [ConfigRule!]!
complianceByRule(ruleNames: [String], complianceTypes: [String]): [ComplianceSummary!]!
complianceByResource(resourceType: String, complianceTypes: [String]): [ComplianceByResource!]!
```

### Connect

```graphql
connectInstances: [ConnectInstance!]!
connectQueues(instanceId: String!, queueTypes: [String]): [ConnectQueue!]!
connectContactFlows(instanceId: String!, contactFlowTypes: [String]): [ConnectContactFlow!]!
connectUsers(instanceId: String!): [ConnectUser!]!
```

### Control Tower

```graphql
controlTowerLandingZones: [ControlTowerLandingZone!]!
controlTowerEnabledControls(targetIdentifier: String): [EnabledControl!]!
```

### Cost Explorer

```graphql
costAndUsage(start: String!, end: String!, granularity: String!, groupBy: [String]): [CostAndUsageResult!]!
costForecast(start: String!, end: String!, granularity: String!): [ForecastResult!]!
```

### DataSync

```graphql
dataSyncAgents: [DataSyncAgent!]!
dataSyncLocations: [DataSyncLocation!]!
dataSyncTasks: [DataSyncTask!]!
dataSyncTaskExecutions(taskArn: String!): [DataSyncTaskExecution!]!
```

### Detective

```graphql
detectiveGraphs: [DetectiveGraph!]!
detectiveMembers(graphArn: String!): [DetectiveMember!]!
detectiveDatasourcePackages(graphArn: String!): [DetectiveDatasourcePackage!]!
```

### Direct Connect

```graphql
dxConnections: [DxConnection!]!
dxVirtualInterfaces(connectionId: String): [DxVirtualInterface!]!
```

### DMS (Database Migration Service)

```graphql
dmsReplicationInstances: [DmsReplicationInstance!]!
dmsEndpoints(endpointType: String): [DmsEndpoint!]!
dmsReplicationTasks: [DmsReplicationTask!]!
```

### DocumentDB

```graphql
docdbClusters: [DocDbCluster!]!
docdbInstances(clusterId: String): [DocDbInstance!]!
```

### DynamoDB

```graphql
dynamoTables: [String!]!
dynamoTable(name: String!): DynamoTable
dynamoScan(table: String!, filterExpression: String, limit: Int): DynamoScanResult!
```

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
launchTemplates(ids: [String], names: [String]): [LaunchTemplate!]!
launchTemplateVersions(launchTemplateId: String!, versions: [String]): [LaunchTemplateVersion!]!
snapshots(ids: [String], volumeId: String, state: String): [Snapshot!]!
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

### ECR

```graphql
ecrRepositories(names: [String]): [EcrRepository!]!
ecrImages(repositoryName: String!, imageTags: [String], imageDigests: [String]): [EcrImage!]!
ecrImageScanFindings(repositoryName: String!, imageDigest: String!): EcrImageScanFindings!
```

### ECS

```graphql
ecsClusters(clusterArns: [String]): [Cluster!]!
ecsServices(cluster: String!, serviceArns: [String]): [Service!]!
ecsTasks(cluster: String!, serviceArn: String, desiredStatus: String): [Task!]!
ecsTaskDefinition(taskDefinition: String!): TaskDefinition
ecsTaskDefinitions(familyPrefix: String, status: String): [String!]!
```

### EFS

```graphql
efsFileSystems: [EfsFileSystem!]!
efsMountTargets(fileSystemId: String!): [EfsMountTarget!]!
efsAccessPoints(fileSystemId: String): [EfsAccessPoint!]!
```

### EKS

```graphql
eksCluster(name: String!): EksCluster
eksClusters(clusterNames: [String]): [EksCluster!]!
eksNodegroups(cluster: String!, nodegroupNames: [String]): [EksNodegroup!]!
eksFargateProfiles(cluster: String!): [EksFargateProfile!]!
eksAddons(cluster: String!): [EksAddon!]!
```

### ElastiCache

```graphql
elasticacheClusters(clusterId: String): [ElastiCacheCluster!]!
elasticacheReplicationGroups(replicationGroupId: String): [ElastiCacheReplicationGroup!]!
elasticacheSubnetGroups: [ElastiCacheSubnetGroup!]!
```

### Elastic Beanstalk

```graphql
beanstalkApplications(applicationNames: [String]): [BeanstalkApplication!]!
beanstalkEnvironments(applicationName: String, environmentNames: [String], includedDeletedBackTo: String): [BeanstalkEnvironment!]!
beanstalkApplicationVersions(applicationName: String, versionLabels: [String]): [BeanstalkApplicationVersion!]!
```

### ELB (v2)

```graphql
loadBalancers(arns: [String], names: [String]): [LoadBalancer!]!
targetGroups(arns: [String], loadBalancerArn: String): [TargetGroup!]!
targetHealth(targetGroupArn: String!): [TargetHealthInfo!]!
listeners(loadBalancerArn: String!): [Listener!]!
listenerRules(listenerArn: String!): [ListenerRule!]!
```

### EMR

```graphql
emrClusters(states: [String]): [EmrCluster!]!
emrSteps(clusterId: String!): [EmrStep!]!
```

### EventBridge

```graphql
eventBridgeBuses: [EbEventBus!]!
eventBridgeRules(eventBusName: String): [EbRule!]!
eventBridgeTargets(ruleName: String!, eventBusName: String): [EbTarget!]!
```

### Firehose

```graphql
firehoseDeliveryStreams: [FirehoseDeliveryStream!]!
firehoseDeliveryStream(name: String!): FirehoseDeliveryStream
```

### FMS (Firewall Manager)

```graphql
fmsPolicies: [FmsPolicy!]!
fmsPolicyComplianceStatuses(policyId: String!): [FmsPolicyComplianceStatus!]!
fmsMemberAccounts: [String!]!
```

### FSx

```graphql
fsxFileSystems(fileSystemIds: [String]): [FsxFileSystem!]!
fsxBackups(backupIds: [String], fileSystemId: String): [FsxBackup!]!
fsxStorageVirtualMachines(fileSystemId: String): [FsxStorageVirtualMachine!]!
```

### Global Accelerator

```graphql
globalAccelerators: [Accelerator!]!
globalAcceleratorListeners(acceleratorArn: String!): [GaListener!]!
globalAcceleratorEndpointGroups(listenerArn: String!): [GaEndpointGroup!]!
```

### Glue

```graphql
glueDatabases: [GlueDatabase!]!
glueTables(databaseName: String!): [GlueTable!]!
glueCrawlers: [GlueCrawler!]!
glueJobs: [GlueJob!]!
```

### GuardDuty

```graphql
guarddutyDetectors: [Detector!]!
guarddutyFindings(detectorId: String!, minSeverity: Float, findingType: String, archived: Boolean): [Finding!]!
```

### Health

```graphql
healthEvents(statusCodes: [String], services: [String]): [HealthEvent!]!
```

### IAM

```graphql
iamRoles(pathPrefix: String): [IamRole!]!
iamPolicies(scope: String, pathPrefix: String): [IamPolicy!]!
iamUsers(pathPrefix: String): [IamUser!]!
iamGroups(pathPrefix: String): [IamGroup!]!
iamAttachedRolePolicies(roleName: String!): [IamAttachedPolicy!]!
iamPolicyDocument(policyArn: String!, versionId: String): IamPolicyDocument!
iamRoleInlinePolicies(roleName: String!): [IamInlinePolicy!]!
iamPasswordPolicy: IamPasswordPolicy
iamMfaDevices: [IamMfaDevice!]!
iamAccessKeys: [IamAccessKey!]!
```

`iamPolicies` defaults `scope` to `"Local"` (customer-managed only). Use `"All"` for AWS-managed policies too.

### Inspector

```graphql
inspectorFindings(severity: String, resourceType: String): [InspectorFinding!]!
inspectorCoverage: [InspectorCoverage!]!
```

### IoT

```graphql
iotThings(thingTypeName: String, attributeName: String, attributeValue: String): [IotThing!]!
iotThingGroups(parentGroup: String): [IotThingGroup!]!
iotPolicies: [IotPolicy!]!
iotCertificates(ascendingOrder: Boolean): [IotCertificate!]!
iotTopicRules(topicRuleDisabled: Boolean): [IotTopicRule!]!
```

### Keyspaces (Amazon Keyspaces for Apache Cassandra)

```graphql
keyspacesKeyspaces: [KeyspacesKeyspace!]!
keyspacesTables(keyspaceName: String!): [KeyspacesTable!]!
keyspacesTable(keyspaceName: String!, tableName: String!): KeyspacesTable
```

### Kinesis

```graphql
kinesisStreams: [DataStream!]!
kinesisShards(streamName: String!): [Shard!]!
```

### KMS

```graphql
kmsKeys: [KmsKey!]!
kmsAliases(keyId: String): [KmsAlias!]!
kmsKeyPolicyNames(keyId: String!): [String!]!
kmsKeyPolicy(keyId: String!, policyName: String!): KmsKeyPolicy
```

### Lake Formation

```graphql
lakeFormationResources: [LakeFormationResource!]!
lakeFormationPermissions(principal: String, resourceType: String): [LakeFormationPermission!]!
lakeFormationSettings: LakeFormationSettings
```

### Lambda

```graphql
lambdaFunctions: [LambdaFunction!]!
lambdaAliases(functionName: String!): [LambdaAlias!]!
lambdaEventSourceMappings(functionName: String): [LambdaEventSourceMapping!]!
lambdaLayers: [LambdaLayer!]!
lambdaFunctionPolicy(functionName: String!): String
```

### License Manager

```graphql
licenseConfigurations: [LicenseConfiguration!]!
licenses: [License!]!
licenseGrants: [LicenseGrant!]!
```

### Lightsail

```graphql
lightsailInstances: [LightsailInstance!]!
lightsailDatabases: [LightsailDatabase!]!
lightsailLoadBalancers: [LightsailLoadBalancer!]!
lightsailStaticIps: [LightsailStaticIp!]!
```

### Macie

```graphql
macieFindings(severity: String, findingType: String): [MacieFinding!]!
macieBucketSummaries: [MacieBucketSummary!]!
```

### MemoryDB

```graphql
memorydbClusters: [MemoryDbCluster!]!
memorydbSubnetGroups: [MemoryDbSubnetGroup!]!
```

### MQ (Amazon MQ)

```graphql
mqBrokers: [MqBroker!]!
mqBroker(brokerId: String!): MqBroker
mqConfigurations: [MqConfiguration!]!
```

### MSK (Managed Streaming for Kafka)

```graphql
mskClusters: [MskCluster!]!
mskBrokerNodes(clusterArn: String!): [BrokerNode!]!
```

### Neptune

```graphql
neptuneClusters: [NeptuneCluster!]!
neptuneInstances(clusterId: String): [NeptuneInstance!]!
```

### Network Firewall

```graphql
networkFirewalls: [Firewall!]!
networkFirewallPolicies: [FirewallPolicy!]!
networkFirewallRuleGroups(ruleGroupType: String): [RuleGroup!]!
```

### OpenSearch

```graphql
opensearchDomains: [OpenSearchDomain!]!
opensearchDomain(domainName: String!): OpenSearchDomain
opensearchDomainTags(arn: String!): [Tag!]!
```

### Organizations

```graphql
orgAccounts: [OrgAccount!]!
orgOrganizationalUnits(parentId: String!): [OrganizationalUnit!]!
orgPolicies(policyType: String!): [OrgPolicy!]!
```

### Pinpoint

```graphql
pinpointApps: [PinpointApp!]!
pinpointCampaigns(applicationId: String!): [PinpointCampaign!]!
pinpointSegments(applicationId: String!): [PinpointSegment!]!
```

### Polly

```graphql
pollyVoices(languageCode: String, engine: String): [PollyVoice!]!
pollyLexicons: [PollyLexicon!]!
pollySpeechSynthesisTasks(status: String): [PollySpeechSynthesisTask!]!
```

### QLDB (Quantum Ledger Database)

```graphql
qldbLedgers: [QldbLedger!]!
qldbLedger(name: String!): QldbLedger
qldbJournalExports(ledgerName: String!): [QldbJournalExport!]!
```

### QuickSight

```graphql
quickSightUsers(awsAccountId: String!, namespace: String): [QuickSightUser!]!
quickSightDashboards(awsAccountId: String!): [QuickSightDashboard!]!
quickSightDataSets(awsAccountId: String!): [QuickSightDataSet!]!
quickSightDataSources(awsAccountId: String!): [QuickSightDataSource!]!
```

### RAM (Resource Access Manager)

```graphql
ramResourceShares(resourceOwner: String): [RamResourceShare!]!
ramResources(resourceOwner: String!, resourceShareArns: [String], resourceType: String): [RamResource!]!
ramPrincipals(resourceOwner: String!, resourceShareArns: [String]): [RamPrincipal!]!
```

### RDS

```graphql
dbInstances(ids: [String]): [DbInstance!]!
dbClusters(ids: [String]): [DbCluster!]!
dbSnapshots(dbInstanceId: String, snapshotType: String): [DbSnapshot!]!
rdsParameterGroups: [DbParameterGroup!]!
rdsSubnetGroups: [DbSubnetGroup!]!
```

### Redshift

```graphql
redshiftClusters: [RedshiftCluster!]!
redshiftSnapshots(clusterIdentifier: String, snapshotType: String): [RedshiftSnapshot!]!
```

### Redshift Serverless

```graphql
redshiftServerlessNamespaces: [RedshiftServerlessNamespace!]!
redshiftServerlessWorkgroups: [RedshiftServerlessWorkgroup!]!
```

### Rekognition

```graphql
rekognitionCollections: [RekognitionCollection!]!
rekognitionProjects: [RekognitionProject!]!
rekognitionStreamProcessors: [RekognitionStreamProcessor!]!
```

### Route 53

```graphql
r53HostedZones: [R53HostedZone!]!
r53Records(hostedZoneId: String!): [R53ResourceRecordSet!]!
r53HealthChecks: [R53HealthCheck!]!
```

### S3

```graphql
s3Buckets: [S3Bucket!]!
s3Bucket(name: String!): S3Bucket
s3BucketPolicy(name: String!): String
```

### SageMaker

```graphql
sagemakerEndpoints(statusFilter: String): [SageMakerEndpoint!]!
sagemakerTrainingJobs(statusFilter: String, maxResults: Int): [SageMakerTrainingJob!]!
sagemakerModels: [SageMakerModel!]!
```

### Secrets Manager

```graphql
secretsList: [Secret!]!
secretDescribe(secretId: String!): Secret
secretValue(secretId: String!): SecretValue
secretResourcePolicy(secretId: String!): String
```

### Security Hub

```graphql
securityHubFindings(severityLabel: String, workflowStatus: String, recordState: String, maxResults: Int): [SecurityHubFinding!]!
```

### Service Quotas

```graphql
serviceQuotas(serviceCode: String!): [ServiceQuota!]!
serviceQuotaServices: [String!]!
```

### SES (Simple Email Service v2)

```graphql
sesIdentities(pageSize: Int): [SesIdentity!]!
sesIdentity(identity: String!): SesIdentity
sesConfigurationSets: [SesConfigurationSet!]!
sesEmailTemplates: [SesEmailTemplate!]!
sesSuppressedDestinations(reasons: [String], startDate: String, endDate: String): [SesSuppressedDestination!]!
sesAccountDetails: SesAccountDetails
```

### Shield

```graphql
shieldSubscription: ShieldSubscription
shieldProtections(resourceArn: String): [ShieldProtection!]!
shieldProtectionGroups: [ProtectionGroup!]!
shieldAttacks(resourceArns: [String], startTime: String, endTime: String): [AttackSummary!]!
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

### SSM

```graphql
managedInstances(instanceIds: [String], pingStatus: PingStatus, platformType: PlatformType): [ManagedInstance!]!
parameters(names: [String!]!, withDecryption: Boolean): [Parameter!]!
parametersByPath(path: String!, recursive: Boolean, withDecryption: Boolean): [Parameter!]!
parameterMetadata(filters: [ParameterFilter]): [ParameterMeta!]!
documents(owner: String, documentType: String, name: String): [SsmDocument!]!
```

### SSO Admin

```graphql
ssoInstances: [SsoInstance!]!
ssoPermissionSets(instanceArn: String!): [SsoPermissionSet!]!
ssoAccountAssignments(instanceArn: String!, accountId: String!, permissionSetArn: String!): [SsoAccountAssignment!]!
```

### Step Functions

```graphql
stateMachines: [StateMachine!]!
executions(stateMachineArn: String!, statusFilter: String): [Execution!]!
executionDetail(executionArn: String!): ExecutionDetail!
```

### Storage Gateway

```graphql
storageGateways: [StorageGatewayGateway!]!
storageGatewayVolumes(gatewayArn: String!): [StorageGatewayVolume!]!
storageGatewayFileShares(gatewayArn: String!): [StorageGatewayFileShare!]!
```

### STS

```graphql
stsCallerIdentity: CallerIdentity!
```

### Timestream

```graphql
timestreamDatabases: [TimestreamDatabase!]!
timestreamTables(databaseName: String!): [TimestreamTable!]!
```

### Transcribe

```graphql
transcribeJobs(statusEquals: String, jobNameContains: String): [TranscriptionJob!]!
transcribeVocabularies(stateEquals: String): [TranscribeVocabulary!]!
transcribeLanguageModels(statusEquals: String): [TranscribeLanguageModel!]!
```

### Transfer Family

```graphql
transferServers: [TransferServer!]!
transferUsers(serverId: String!): [TransferUser!]!
```

### Translate

```graphql
translateTerminologies: [TranslateTerminology!]!
translateParallelData: [TranslateParallelData!]!
translateTextTranslationJobs(filter: TranslateJobFilterInput): [TranslateTextTranslationJob!]!
```

### VPC Resources

```graphql
routeTables(vpcId: String, ids: [String]): [RouteTable!]!
networkAcls(vpcId: String, ids: [String]): [NetworkAcl!]!
internetGateways(vpcId: String, ids: [String]): [InternetGateway!]!
natGateways(vpcId: String, ids: [String], state: String): [NatGateway!]!
vpcEndpoints(vpcId: String, ids: [String], serviceName: String): [VpcEndpoint!]!
transitGateways(ids: [String]): [TransitGateway!]!
vpcFlowLogs(resourceId: String): [VpcFlowLog!]!
```

### WAF v2

```graphql
wafWebAcls(scope: WafScope!): [WebAcl!]!
wafIpSets(scope: WafScope!): [WafIpSet!]!
wafRuleGroups(scope: WafScope!): [WafRuleGroup!]!
```

`WafScope` enum: `REGIONAL`, `CLOUDFRONT`

### WorkSpaces

```graphql
workspaces(directoryId: String, userName: String, bundleId: String): [Workspace!]!
workspaceDirectories: [WorkspaceDirectory!]!
workspaceBundles(owner: String): [WorkspaceBundle!]!
```

### X-Ray

```graphql
xrayGroups: [XRayGroup!]!
xraySamplingRules: [XRaySamplingRule!]!
xrayEncryptionConfig: XRayEncryptionConfig
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

**List S3 buckets:**
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

**List CloudWatch log groups:**
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

**Get GuardDuty findings:**
```bash
vapor query '{ guarddutyDetectors { detectorId } }' # get a detector ID first
vapor query '{ guarddutyFindings(detectorId: "abc123", minSeverity: 7.0) { id title severity } }'
```

**Inspect Bedrock foundation models:**
```bash
vapor query '{ bedrockFoundationModels { modelId modelName providerName inputModalities outputModalities } }'
```

**Check AWS cost and usage:**
```bash
vapor query '{ costAndUsage(start: "2024-01-01", end: "2024-02-01", granularity: "MONTHLY") { timePeriodStart timePeriodEnd total { amount unit } } }'
```

**List Step Functions state machines:**
```bash
vapor query '{ stateMachines { name stateMachineArn type status } }'
```

**List Glue databases and tables:**
```bash
vapor query '{ glueDatabases { name description } }'
vapor query '{ glueTables(databaseName: "my-db") { name tableType } }'
```

**Get caller identity:**
```bash
vapor query '{ stsCallerIdentity { account userId arn } }'
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
| `kafka` | MSK (Managed Kafka) |
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
