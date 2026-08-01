# Query-root names

Most top-level query names are the service name in camelCase (`s3Buckets`,
`ecsClusters`, `stsCallerIdentity`). The cases below are the ones you cannot
derive. When in doubt, list them instead of guessing:

```bash
vapor read --format compact '{ __schema { queryType { fields { name } } } }'
```

## EC2 and VPC own the bare names

EC2: `instances`, `securityGroups`, `vpcs`, `subnets`, `volumes`, `snapshots`,
`keyPairs`, `elasticIps`, `images`, `launchTemplates`, `launchTemplateVersions`.

VPC (a separate module, same bare style): `routeTables`, `networkAcls`,
`internetGateways`, `natGateways`, `vpcEndpoints`, `transitGateways`,
`vpcFlowLogs`.

## Non-derivable prefixes

| Service | Prefix | Example |
|---|---|---|
| CloudFormation | `cfn` | `cfnStacks`, `cfnExports` |
| API Gateway (REST / v1) | `apigwRest` | `apigwRestApis`, `apigwRestStages` |
| API Gateway v2 (HTTP / WebSocket) | `apiV2` | `apiV2Apis`, `apiV2Routes` |
| Route 53 | `r53` | `r53HostedZones`, `r53Records` |
| Direct Connect | `dx` | `dxConnections` |
| Organizations | `org` | `orgAccounts`, `orgPolicies` |
| DynamoDB | `dynamo` | `dynamoTables`, `dynamoScan` |
| DocumentDB | `docdb` | `docdbClusters` |
| RDS | `db` *and* `rds` | `dbInstances`, `dbClusters`, `dbSnapshots`, but `rdsParameterGroups`, `rdsSubnetGroups` |
| Elastic Beanstalk | `beanstalk` | `beanstalkEnvironments` |
| WAFv2 | `waf` | `wafWebAcls`, `wafIpSets` |
| Secrets Manager | `secret` | `secretValue`, `secretsList` |
| ACM Private CA | *(none)* | `privateCertificateAuthorities` |

RDS is the one service that splits its prefix. The databases themselves take
`db` — it is `dbInstances`, **not** `rdsInstances` — while the two
configuration collections take `rds`.

## Services that take no prefix

These own bare names the way EC2 does:

- **CloudWatch + Logs** — `alarms`, `metrics`, `metricData`, `logGroups`,
  `logStreams`, `logEvents`, `metricFilters`
- **ELBv2** — `loadBalancers`, `targetGroups`, `listeners`, `listenerRules`
- **Auto Scaling** — `autoScalingGroups`, `scalingActivities`
- **Step Functions** — `stateMachines`, `executions`, `executionDetail`
- **SSM** — `parameters`, `documents`, `managedInstances`
- **CodeBuild / CodeDeploy / CodePipeline** — `buildProjects`, `builds`,
  `deployApplications`, `deploymentGroups`, `deployments`, `pipelines`,
  `pipelineExecutions`, `pipelineState`
- **Cost Explorer** — `costAndUsage`, `costForecast`
- **Config** — `configRules`, `complianceByRule`, `complianceByResource`
- **Budgets** — `budgets`, `budgetNotifications`
- **License Manager** — `licenses`, `licenseConfigurations`, `licenseGrants`
- **WorkSpaces** — `workspaces`, `workspaceDirectories`, `workspaceBundles`

## Multi-word services

They split on the word boundary: `codeCommitRepositories`, `codeArtifactDomains`,
`eventBridgeRules`, `quickSightUsers` — not `codecommit…`.
