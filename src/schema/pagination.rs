//! Generic resumable-pagination wrapper for list-returning GraphQL queries.
//!
//! Every list query is migrating (schema v2, `specs/plan-2-schema-v2-pagination-timestamps.md`)
//! from returning a bare `[T!]!` to `{ items: [T!]!, nextToken: String }`, so
//! a `limit` that truncates a result set always comes with a token to resume
//! from — today `limit` silently drops everything past the cap with no way
//! to continue. `next_token` is the AWS SDK's own opaque continuation token
//! passed straight through (no wrapping/encoding layer); token stability
//! across vapor versions is not guaranteed.
//!
//! async-graphql's generics support requires one `#[graphql(concrete(...))]`
//! line per instantiation actually used in the schema — those are added
//! here as each service converts to this type, not all at once, since
//! registering a concrete type that no resolver returns yet would be dead
//! code.

use async_graphql::{OutputType, SimpleObject};

#[cfg(feature = "kinesis")]
use crate::schema::kinesis::types::{DataStream, Shard};
#[cfg(feature = "auditmanager")]
use crate::schema::audit_manager::types::{
    AuditManagerAssessment, AuditManagerControl, AuditManagerFramework,
};
#[cfg(feature = "budgets")]
use crate::schema::budgets::types::{Budget, BudgetNotification};
#[cfg(feature = "comprehend")]
use crate::schema::comprehend::types::{
    ComprehendDocumentClassifier, ComprehendEndpoint, ComprehendEntityRecognizer,
};
#[cfg(feature = "shield")]
use crate::schema::shield::types::{AttackSummary, ProtectionGroup, ShieldProtection};
#[cfg(feature = "codeartifact")]
use crate::schema::codeartifact::types::{
    CodeArtifactDomain, CodeArtifactPackage, CodeArtifactRepository,
};
#[cfg(feature = "eventbridge")]
use crate::schema::eventbridge::types::{EbEventBus, EbRule, EbTarget};
#[cfg(feature = "costexplorer")]
use crate::schema::cost_explorer::types::CostAndUsageResult;
#[cfg(feature = "mq")]
use crate::schema::mq::types::{MqBroker, MqConfiguration};
#[cfg(feature = "pinpoint")]
use crate::schema::pinpoint::types::{PinpointApp, PinpointCampaign, PinpointSegment};
#[cfg(feature = "polly")]
use crate::schema::polly::types::{PollyLexicon, PollySpeechSynthesisTask, PollyVoice};
#[cfg(feature = "licensemanager")]
use crate::schema::license_manager::types::{License, LicenseConfiguration, LicenseGrant};
#[cfg(feature = "elasticbeanstalk")]
use crate::schema::elastic_beanstalk::types::{BeanstalkApplicationVersion, BeanstalkEnvironment};
#[cfg(feature = "apigatewayv2")]
use crate::schema::apigatewayv2::types::{ApiV2, ApiV2DomainName, ApiV2Route, ApiV2Stage, ApiV2VpcLink};
#[cfg(feature = "storagegateway")]
use crate::schema::storage_gateway::types::{StorageGatewayFileShare, StorageGatewayGateway, StorageGatewayVolume};
#[cfg(feature = "autoscaling")]
use crate::schema::asg::types::{AutoScalingGroup, ScalingActivity};
#[cfg(feature = "servicequotas")]
use crate::schema::service_quotas::types::ServiceQuota;
#[cfg(feature = "health")]
use crate::schema::health::types::HealthEvent;
#[cfg(feature = "appsync")]
use crate::schema::appsync::types::{AppSyncApi, AppSyncDataSource};
#[cfg(feature = "transfer")]
use crate::schema::transfer::types::{TransferServer, TransferUser};
#[cfg(feature = "directconnect")]
use crate::schema::direct_connect::types::{DxConnection, DxVirtualInterface};
#[cfg(feature = "redshiftserverless")]
use crate::schema::redshift_serverless::types::{RedshiftServerlessNamespace, RedshiftServerlessWorkgroup};
#[cfg(feature = "neptune")]
use crate::schema::neptune::types::{NeptuneCluster, NeptuneInstance};
#[cfg(feature = "inspector2")]
use crate::schema::inspector::types::{InspectorCoverage, InspectorFinding};
#[cfg(feature = "docdb")]
use crate::schema::documentdb::types::{DocDbCluster, DocDbInstance};
#[cfg(feature = "securityhub")]
use crate::schema::security_hub::types::SecurityHubFinding;
#[cfg(feature = "detective")]
use crate::schema::detective::types::{DetectiveDatasourcePackage, DetectiveGraph, DetectiveMember};
#[cfg(feature = "firehose")]
use crate::schema::firehose::types::FirehoseDeliveryStream;
#[cfg(feature = "controltower")]
use crate::schema::control_tower::types::{ControlTowerLandingZone, EnabledControl};
#[cfg(feature = "kafka")]
use crate::schema::msk::types::{BrokerNode, MskCluster};
#[cfg(feature = "appconfig")]
use crate::schema::appconfig::types::{AppConfigApplication, AppConfigEnvironment, AppConfigProfile};
#[cfg(feature = "memorydb")]
use crate::schema::memorydb::types::{MemoryDbCluster, MemoryDbSubnetGroup};
#[cfg(feature = "globalaccelerator")]
use crate::schema::global_accelerator::types::{Accelerator, GaEndpointGroup, GaListener};
#[cfg(feature = "cognitoidentityprovider")]
use crate::schema::cognito::types::{UserPool, UserPoolClient};
#[cfg(feature = "backup")]
use crate::schema::backup::types::{BackupPlan, BackupVault, RecoveryPoint};
#[cfg(feature = "organizations")]
use crate::schema::organizations::types::{OrgAccount, OrgPolicy, OrganizationalUnit};
#[cfg(feature = "codebuild")]
use crate::schema::codebuild::types::{Build, BuildProject};
#[cfg(feature = "codepipeline")]
use crate::schema::codepipeline::types::{Pipeline, PipelineExecution};
#[cfg(feature = "macie2")]
use crate::schema::macie::types::{MacieBucketSummary, MacieFinding};
#[cfg(feature = "acm")]
use crate::schema::acm::types::AcmCertificate;
#[cfg(feature = "sns")]
use crate::schema::sns::types::{SnsSubscription, SnsTopic};
#[cfg(feature = "batch")]
use crate::schema::batch::types::{BatchComputeEnvironment, BatchJobDefinition, BatchJobQueue};
#[cfg(feature = "acmpca")]
use crate::schema::acm_pca::types::PrivateCa;
#[cfg(feature = "redshift")]
use crate::schema::redshift::types::{RedshiftCluster, RedshiftSnapshot};
#[cfg(feature = "cloudtrail")]
use crate::schema::cloudtrail::types::CloudTrailEvent;
#[cfg(feature = "fms")]
use crate::schema::fms::types::{FmsPolicy, FmsPolicyComplianceStatus};
#[cfg(feature = "emr")]
use crate::schema::emr::types::{EmrCluster, EmrStep};
#[cfg(feature = "qldb")]
use crate::schema::qldb::types::{QldbJournalExport, QldbLedger};
#[cfg(feature = "ram")]
use crate::schema::ram::types::{RamPrincipal, RamResource, RamResourceShare};
#[cfg(feature = "ssoadmin")]
use crate::schema::sso_admin::types::{SsoAccountAssignment, SsoInstance, SsoPermissionSet};
#[cfg(feature = "timestream")]
use crate::schema::timestream::types::{TimestreamDatabase, TimestreamTable};
#[cfg(feature = "sagemaker")]
use crate::schema::sagemaker::types::{SageMakerEndpoint, SageMakerModel, SageMakerTrainingJob};
#[cfg(feature = "codedeploy")]
use crate::schema::codedeploy::types::{DeployApplication, Deployment, DeploymentGroup};
#[cfg(feature = "cloudwatch")]
use crate::schema::cloudwatch::types::{
    Alarm, LogEvent, LogGroup, LogStream, Metric, MetricFilter, MetricResult,
};
#[cfg(feature = "efs")]
use crate::schema::efs::types::{EfsAccessPoint, EfsFileSystem, EfsMountTarget};
#[cfg(feature = "route53")]
use crate::schema::route53::types::{R53HealthCheck, R53HostedZone, R53ResourceRecordSet};
#[cfg(feature = "config")]
use crate::schema::config_svc::types::{ComplianceByResource, ComplianceSummary, ConfigRule};
#[cfg(feature = "elasticache")]
use crate::schema::elasticache::types::{
    ElastiCacheCluster, ElastiCacheReplicationGroup, ElastiCacheSubnetGroup,
};
#[cfg(feature = "cloudfront")]
use crate::schema::cloudfront::types::CfDistribution;
#[cfg(feature = "secretsmanager")]
use crate::schema::secrets_manager::types::Secret;
#[cfg(feature = "lightsail")]
use crate::schema::lightsail::types::{
    LightsailDatabase, LightsailInstance, LightsailLoadBalancer, LightsailStaticIp,
};
#[cfg(feature = "apigateway")]
use crate::schema::apigateway::types::{ApigwDeployment, ApigwResource, ApigwRestApi};
#[cfg(feature = "xray")]
use crate::schema::xray::types::{XRayGroup, XRaySamplingRule};
#[cfg(feature = "glue")]
use crate::schema::glue::types::{GlueCrawler, GlueDatabase, GlueJob, GlueTable};
#[cfg(feature = "dms")]
use crate::schema::dms::types::{DmsEndpoint, DmsReplicationInstance, DmsReplicationTask};
#[cfg(feature = "wafv2")]
use crate::schema::wafv2::types::{WafIpSet, WafRuleGroup, WebAcl};
#[cfg(feature = "elbv2")]
use crate::schema::elbv2::types::{Listener, ListenerRule, LoadBalancer, TargetGroup};
#[cfg(feature = "kms")]
use crate::schema::kms::types::{KmsAlias, KmsKey};
#[cfg(feature = "guardduty")]
use crate::schema::guardduty::types::{Detector, Finding};
#[cfg(feature = "athena")]
use crate::schema::athena::types::{AthenaNamedQuery, AthenaQueryExecution, AthenaWorkgroup};
#[cfg(feature = "rekognition")]
use crate::schema::rekognition::types::{
    RekognitionCollection, RekognitionProject, RekognitionStreamProcessor,
};
#[cfg(feature = "keyspaces")]
use crate::schema::keyspaces::types::{KeyspacesKeyspace, KeyspacesTable};
#[cfg(feature = "fsx")]
use crate::schema::fsx::types::{FsxBackup, FsxFileSystem, FsxStorageVirtualMachine};
#[cfg(feature = "transcribe")]
use crate::schema::transcribe::types::{
    TranscribeLanguageModel, TranscribeVocabulary, TranscriptionJob,
};
#[cfg(feature = "cloudformation")]
use crate::schema::cloudformation::types::{CfnExport, CfnStack, CfnStackResource};
#[cfg(feature = "s3")]
use crate::schema::s3::types::S3Bucket;
#[cfg(feature = "sfn")]
use crate::schema::step_functions::types::{Execution, StateMachine};
#[cfg(feature = "translate")]
use crate::schema::translate::types::{
    TranslateParallelData, TranslateTerminology, TranslateTextTranslationJob,
};
#[cfg(feature = "workspaces")]
use crate::schema::workspaces::types::{Workspace, WorkspaceBundle, WorkspaceDirectory};
#[cfg(feature = "quicksight")]
use crate::schema::quicksight::types::{
    QuickSightDashboard, QuickSightDataSet, QuickSightDataSource, QuickSightUser,
};
#[cfg(feature = "datasync")]
use crate::schema::datasync::types::{
    DataSyncAgent, DataSyncLocation, DataSyncTask, DataSyncTaskExecution,
};
#[cfg(feature = "lakeformation")]
use crate::schema::lake_formation::types::{LakeFormationPermission, LakeFormationResource};
#[cfg(feature = "lambda")]
use crate::schema::lambda::types::{
    LambdaAlias, LambdaEventSourceMapping, LambdaFunction, LambdaLayer,
};
#[cfg(feature = "codecommit")]
use crate::schema::codecommit::types::{
    CodeCommitBranch, CodeCommitPullRequest, CodeCommitRepository,
};
#[cfg(feature = "iot")]
use crate::schema::iot::types::{IotCertificate, IotPolicy, IotThing, IotThingGroup, IotTopicRule};
#[cfg(feature = "sesv2")]
use crate::schema::ses::types::{
    SesConfigurationSet, SesEmailTemplate, SesIdentity, SesSuppressedDestination,
};
#[cfg(feature = "connect")]
use crate::schema::connect::types::{
    ConnectContactFlow, ConnectInstance, ConnectQueue, ConnectUser,
};
#[cfg(feature = "bedrock")]
use crate::schema::bedrock::types::{BedrockCustomModel, BedrockGuardrail};
#[cfg(feature = "ecr")]
use crate::schema::ecr::types::{EcrImage, EcrRepository};
#[cfg(feature = "networkfirewall")]
use crate::schema::network_firewall::types::{Firewall, FirewallPolicy, RuleGroup};
#[cfg(feature = "ec2")]
use crate::schema::vpc::types::{
    InternetGateway, NatGateway, NetworkAcl, RouteTable, TransitGateway, VpcEndpoint, VpcFlowLog,
};
#[cfg(feature = "ec2")]
use crate::schema::ec2::types::{
    Image, Instance, LaunchTemplate, LaunchTemplateVersion, SecurityGroup, Snapshot, Subnet,
    Volume, Vpc,
};
#[cfg(feature = "apprunner")]
use crate::schema::app_runner::types::{
    AppRunnerConnection, AppRunnerObservabilityConfiguration, AppRunnerService,
    AppRunnerVpcConnector,
};
#[cfg(feature = "eks")]
use crate::schema::eks::types::{EksAddon, EksCluster, EksFargateProfile, EksNodegroup};
#[cfg(feature = "rds")]
use crate::schema::rds::types::{DbCluster, DbInstance, DbParameterGroup, DbSnapshot, DbSubnetGroup};
#[cfg(feature = "ssm")]
use crate::schema::ssm::types::{ManagedInstance, Parameter, ParameterMeta, SsmDocument};
#[cfg(feature = "ecs")]
use crate::schema::ecs::types::{Cluster as EcsCluster, Service as EcsService, Task as EcsTask};
#[cfg(feature = "iam")]
use crate::schema::iam::types::{
    IamAccessKey, IamAttachedPolicy, IamGroup, IamInlinePolicy, IamMfaDevice, IamPolicy, IamRole,
    IamUser,
};

#[derive(SimpleObject)]
#[cfg_attr(feature = "kinesis", graphql(concrete(name = "DataStreamPage", params(DataStream))))]
#[cfg_attr(feature = "kinesis", graphql(concrete(name = "ShardPage", params(Shard))))]
#[cfg_attr(feature = "eventbridge", graphql(concrete(name = "EbEventBusPage", params(EbEventBus))))]
#[cfg_attr(feature = "eventbridge", graphql(concrete(name = "EbRulePage", params(EbRule))))]
#[cfg_attr(feature = "eventbridge", graphql(concrete(name = "EbTargetPage", params(EbTarget))))]
#[cfg_attr(feature = "costexplorer", graphql(concrete(name = "CostAndUsageResultPage", params(CostAndUsageResult))))]
#[cfg_attr(feature = "mq", graphql(concrete(name = "MqBrokerPage", params(MqBroker))))]
#[cfg_attr(feature = "mq", graphql(concrete(name = "MqConfigurationPage", params(MqConfiguration))))]
#[cfg_attr(feature = "pinpoint", graphql(concrete(name = "PinpointAppPage", params(PinpointApp))))]
#[cfg_attr(feature = "pinpoint", graphql(concrete(name = "PinpointCampaignPage", params(PinpointCampaign))))]
#[cfg_attr(feature = "pinpoint", graphql(concrete(name = "PinpointSegmentPage", params(PinpointSegment))))]
#[cfg_attr(feature = "polly", graphql(concrete(name = "PollyVoicePage", params(PollyVoice))))]
#[cfg_attr(feature = "polly", graphql(concrete(name = "PollyLexiconPage", params(PollyLexicon))))]
#[cfg_attr(feature = "polly", graphql(concrete(name = "PollySpeechSynthesisTaskPage", params(PollySpeechSynthesisTask))))]
#[cfg_attr(feature = "licensemanager", graphql(concrete(name = "LicenseConfigurationPage", params(LicenseConfiguration))))]
#[cfg_attr(feature = "licensemanager", graphql(concrete(name = "LicensePage", params(License))))]
#[cfg_attr(feature = "licensemanager", graphql(concrete(name = "LicenseGrantPage", params(LicenseGrant))))]
#[cfg_attr(feature = "elasticbeanstalk", graphql(concrete(name = "BeanstalkEnvironmentPage", params(BeanstalkEnvironment))))]
#[cfg_attr(feature = "elasticbeanstalk", graphql(concrete(name = "BeanstalkApplicationVersionPage", params(BeanstalkApplicationVersion))))]
#[cfg_attr(feature = "apigatewayv2", graphql(concrete(name = "ApiV2Page", params(ApiV2))))]
#[cfg_attr(feature = "apigatewayv2", graphql(concrete(name = "ApiV2StagePage", params(ApiV2Stage))))]
#[cfg_attr(feature = "apigatewayv2", graphql(concrete(name = "ApiV2RoutePage", params(ApiV2Route))))]
#[cfg_attr(feature = "apigatewayv2", graphql(concrete(name = "ApiV2DomainNamePage", params(ApiV2DomainName))))]
#[cfg_attr(feature = "apigatewayv2", graphql(concrete(name = "ApiV2VpcLinkPage", params(ApiV2VpcLink))))]
#[cfg_attr(feature = "storagegateway", graphql(concrete(name = "StorageGatewayGatewayPage", params(StorageGatewayGateway))))]
#[cfg_attr(feature = "storagegateway", graphql(concrete(name = "StorageGatewayVolumePage", params(StorageGatewayVolume))))]
#[cfg_attr(feature = "storagegateway", graphql(concrete(name = "StorageGatewayFileSharePage", params(StorageGatewayFileShare))))]
#[cfg_attr(feature = "autoscaling", graphql(concrete(name = "AutoScalingGroupPage", params(AutoScalingGroup))))]
#[cfg_attr(feature = "autoscaling", graphql(concrete(name = "ScalingActivityPage", params(ScalingActivity))))]
#[cfg_attr(feature = "servicequotas", graphql(concrete(name = "ServiceQuotaPage", params(ServiceQuota))))]
#[cfg_attr(any(feature = "servicequotas", feature = "fms", feature = "sqs", feature = "dynamodb", feature = "kms", feature = "ecs"), graphql(concrete(name = "StringPage", params(String))))]
#[cfg_attr(feature = "health", graphql(concrete(name = "HealthEventPage", params(HealthEvent))))]
#[cfg_attr(feature = "appsync", graphql(concrete(name = "AppSyncApiPage", params(AppSyncApi))))]
#[cfg_attr(feature = "appsync", graphql(concrete(name = "AppSyncDataSourcePage", params(AppSyncDataSource))))]
#[cfg_attr(feature = "transfer", graphql(concrete(name = "TransferServerPage", params(TransferServer))))]
#[cfg_attr(feature = "transfer", graphql(concrete(name = "TransferUserPage", params(TransferUser))))]
#[cfg_attr(feature = "directconnect", graphql(concrete(name = "DxConnectionPage", params(DxConnection))))]
#[cfg_attr(feature = "directconnect", graphql(concrete(name = "DxVirtualInterfacePage", params(DxVirtualInterface))))]
#[cfg_attr(feature = "redshiftserverless", graphql(concrete(name = "RedshiftServerlessNamespacePage", params(RedshiftServerlessNamespace))))]
#[cfg_attr(feature = "redshiftserverless", graphql(concrete(name = "RedshiftServerlessWorkgroupPage", params(RedshiftServerlessWorkgroup))))]
#[cfg_attr(feature = "neptune", graphql(concrete(name = "NeptuneClusterPage", params(NeptuneCluster))))]
#[cfg_attr(feature = "neptune", graphql(concrete(name = "NeptuneInstancePage", params(NeptuneInstance))))]
#[cfg_attr(feature = "inspector2", graphql(concrete(name = "InspectorFindingPage", params(InspectorFinding))))]
#[cfg_attr(feature = "inspector2", graphql(concrete(name = "InspectorCoveragePage", params(InspectorCoverage))))]
#[cfg_attr(feature = "docdb", graphql(concrete(name = "DocDbClusterPage", params(DocDbCluster))))]
#[cfg_attr(feature = "docdb", graphql(concrete(name = "DocDbInstancePage", params(DocDbInstance))))]
#[cfg_attr(feature = "securityhub", graphql(concrete(name = "SecurityHubFindingPage", params(SecurityHubFinding))))]
#[cfg_attr(feature = "detective", graphql(concrete(name = "DetectiveGraphPage", params(DetectiveGraph))))]
#[cfg_attr(feature = "detective", graphql(concrete(name = "DetectiveMemberPage", params(DetectiveMember))))]
#[cfg_attr(feature = "detective", graphql(concrete(name = "DetectiveDatasourcePackagePage", params(DetectiveDatasourcePackage))))]
#[cfg_attr(feature = "firehose", graphql(concrete(name = "FirehoseDeliveryStreamPage", params(FirehoseDeliveryStream))))]
#[cfg_attr(feature = "controltower", graphql(concrete(name = "ControlTowerLandingZonePage", params(ControlTowerLandingZone))))]
#[cfg_attr(feature = "controltower", graphql(concrete(name = "EnabledControlPage", params(EnabledControl))))]
#[cfg_attr(feature = "kafka", graphql(concrete(name = "MskClusterPage", params(MskCluster))))]
#[cfg_attr(feature = "kafka", graphql(concrete(name = "BrokerNodePage", params(BrokerNode))))]
#[cfg_attr(feature = "appconfig", graphql(concrete(name = "AppConfigApplicationPage", params(AppConfigApplication))))]
#[cfg_attr(feature = "appconfig", graphql(concrete(name = "AppConfigEnvironmentPage", params(AppConfigEnvironment))))]
#[cfg_attr(feature = "appconfig", graphql(concrete(name = "AppConfigProfilePage", params(AppConfigProfile))))]
#[cfg_attr(feature = "memorydb", graphql(concrete(name = "MemoryDbClusterPage", params(MemoryDbCluster))))]
#[cfg_attr(feature = "memorydb", graphql(concrete(name = "MemoryDbSubnetGroupPage", params(MemoryDbSubnetGroup))))]
#[cfg_attr(feature = "globalaccelerator", graphql(concrete(name = "AcceleratorPage", params(Accelerator))))]
#[cfg_attr(feature = "globalaccelerator", graphql(concrete(name = "GaListenerPage", params(GaListener))))]
#[cfg_attr(feature = "globalaccelerator", graphql(concrete(name = "GaEndpointGroupPage", params(GaEndpointGroup))))]
#[cfg_attr(feature = "cognitoidentityprovider", graphql(concrete(name = "UserPoolPage", params(UserPool))))]
#[cfg_attr(feature = "cognitoidentityprovider", graphql(concrete(name = "UserPoolClientPage", params(UserPoolClient))))]
#[cfg_attr(feature = "backup", graphql(concrete(name = "BackupVaultPage", params(BackupVault))))]
#[cfg_attr(feature = "backup", graphql(concrete(name = "BackupPlanPage", params(BackupPlan))))]
#[cfg_attr(feature = "backup", graphql(concrete(name = "RecoveryPointPage", params(RecoveryPoint))))]
#[cfg_attr(feature = "organizations", graphql(concrete(name = "OrgAccountPage", params(OrgAccount))))]
#[cfg_attr(feature = "organizations", graphql(concrete(name = "OrganizationalUnitPage", params(OrganizationalUnit))))]
#[cfg_attr(feature = "organizations", graphql(concrete(name = "OrgPolicyPage", params(OrgPolicy))))]
#[cfg_attr(feature = "codebuild", graphql(concrete(name = "BuildProjectPage", params(BuildProject))))]
#[cfg_attr(feature = "codebuild", graphql(concrete(name = "BuildPage", params(Build))))]
#[cfg_attr(feature = "codepipeline", graphql(concrete(name = "PipelinePage", params(Pipeline))))]
#[cfg_attr(feature = "codepipeline", graphql(concrete(name = "PipelineExecutionPage", params(PipelineExecution))))]
#[cfg_attr(feature = "macie2", graphql(concrete(name = "MacieFindingPage", params(MacieFinding))))]
#[cfg_attr(feature = "macie2", graphql(concrete(name = "MacieBucketSummaryPage", params(MacieBucketSummary))))]
#[cfg_attr(feature = "acm", graphql(concrete(name = "AcmCertificatePage", params(AcmCertificate))))]
#[cfg_attr(feature = "sns", graphql(concrete(name = "SnsTopicPage", params(SnsTopic))))]
#[cfg_attr(feature = "sns", graphql(concrete(name = "SnsSubscriptionPage", params(SnsSubscription))))]
#[cfg_attr(feature = "batch", graphql(concrete(name = "BatchJobQueuePage", params(BatchJobQueue))))]
#[cfg_attr(feature = "batch", graphql(concrete(name = "BatchComputeEnvironmentPage", params(BatchComputeEnvironment))))]
#[cfg_attr(feature = "batch", graphql(concrete(name = "BatchJobDefinitionPage", params(BatchJobDefinition))))]
#[cfg_attr(feature = "acmpca", graphql(concrete(name = "PrivateCaPage", params(PrivateCa))))]
#[cfg_attr(feature = "redshift", graphql(concrete(name = "RedshiftClusterPage", params(RedshiftCluster))))]
#[cfg_attr(feature = "redshift", graphql(concrete(name = "RedshiftSnapshotPage", params(RedshiftSnapshot))))]
#[cfg_attr(feature = "cloudtrail", graphql(concrete(name = "CloudTrailEventPage", params(CloudTrailEvent))))]
#[cfg_attr(feature = "fms", graphql(concrete(name = "FmsPolicyPage", params(FmsPolicy))))]
#[cfg_attr(feature = "fms", graphql(concrete(name = "FmsPolicyComplianceStatusPage", params(FmsPolicyComplianceStatus))))]
#[cfg_attr(feature = "emr", graphql(concrete(name = "EmrClusterPage", params(EmrCluster))))]
#[cfg_attr(feature = "emr", graphql(concrete(name = "EmrStepPage", params(EmrStep))))]
#[cfg_attr(feature = "qldb", graphql(concrete(name = "QldbLedgerPage", params(QldbLedger))))]
#[cfg_attr(feature = "qldb", graphql(concrete(name = "QldbJournalExportPage", params(QldbJournalExport))))]
#[cfg_attr(feature = "ram", graphql(concrete(name = "RamResourceSharePage", params(RamResourceShare))))]
#[cfg_attr(feature = "ram", graphql(concrete(name = "RamResourcePage", params(RamResource))))]
#[cfg_attr(feature = "ram", graphql(concrete(name = "RamPrincipalPage", params(RamPrincipal))))]
#[cfg_attr(feature = "ssoadmin", graphql(concrete(name = "SsoInstancePage", params(SsoInstance))))]
#[cfg_attr(feature = "ssoadmin", graphql(concrete(name = "SsoPermissionSetPage", params(SsoPermissionSet))))]
#[cfg_attr(feature = "ssoadmin", graphql(concrete(name = "SsoAccountAssignmentPage", params(SsoAccountAssignment))))]
#[cfg_attr(feature = "timestream", graphql(concrete(name = "TimestreamDatabasePage", params(TimestreamDatabase))))]
#[cfg_attr(feature = "timestream", graphql(concrete(name = "TimestreamTablePage", params(TimestreamTable))))]
#[cfg_attr(feature = "sagemaker", graphql(concrete(name = "SageMakerEndpointPage", params(SageMakerEndpoint))))]
#[cfg_attr(feature = "codedeploy", graphql(concrete(name = "DeployApplicationPage", params(DeployApplication))))]
#[cfg_attr(feature = "codedeploy", graphql(concrete(name = "DeploymentGroupPage", params(DeploymentGroup))))]
#[cfg_attr(feature = "codedeploy", graphql(concrete(name = "DeploymentPage", params(Deployment))))]
#[cfg_attr(feature = "sagemaker", graphql(concrete(name = "SageMakerTrainingJobPage", params(SageMakerTrainingJob))))]
#[cfg_attr(feature = "sagemaker", graphql(concrete(name = "SageMakerModelPage", params(SageMakerModel))))]
#[cfg_attr(feature = "cloudwatch", graphql(concrete(name = "LogGroupPage", params(LogGroup))))]
#[cfg_attr(feature = "cloudwatch", graphql(concrete(name = "LogStreamPage", params(LogStream))))]
#[cfg_attr(feature = "cloudwatch", graphql(concrete(name = "MetricFilterPage", params(MetricFilter))))]
#[cfg_attr(feature = "cloudwatch", graphql(concrete(name = "LogEventPage", params(LogEvent))))]
#[cfg_attr(feature = "cloudwatch", graphql(concrete(name = "MetricPage", params(Metric))))]
#[cfg_attr(feature = "cloudwatch", graphql(concrete(name = "MetricResultPage", params(MetricResult))))]
#[cfg_attr(feature = "cloudwatch", graphql(concrete(name = "AlarmPage", params(Alarm))))]
#[cfg_attr(feature = "efs", graphql(concrete(name = "EfsFileSystemPage", params(EfsFileSystem))))]
#[cfg_attr(feature = "efs", graphql(concrete(name = "EfsMountTargetPage", params(EfsMountTarget))))]
#[cfg_attr(feature = "efs", graphql(concrete(name = "EfsAccessPointPage", params(EfsAccessPoint))))]
#[cfg_attr(feature = "route53", graphql(concrete(name = "R53HostedZonePage", params(R53HostedZone))))]
#[cfg_attr(feature = "route53", graphql(concrete(name = "R53ResourceRecordSetPage", params(R53ResourceRecordSet))))]
#[cfg_attr(feature = "route53", graphql(concrete(name = "R53HealthCheckPage", params(R53HealthCheck))))]
#[cfg_attr(feature = "config", graphql(concrete(name = "ConfigRulePage", params(ConfigRule))))]
#[cfg_attr(feature = "config", graphql(concrete(name = "ComplianceSummaryPage", params(ComplianceSummary))))]
#[cfg_attr(feature = "config", graphql(concrete(name = "ComplianceByResourcePage", params(ComplianceByResource))))]
#[cfg_attr(feature = "elasticache", graphql(concrete(name = "ElastiCacheClusterPage", params(ElastiCacheCluster))))]
#[cfg_attr(feature = "elasticache", graphql(concrete(name = "ElastiCacheReplicationGroupPage", params(ElastiCacheReplicationGroup))))]
#[cfg_attr(feature = "elasticache", graphql(concrete(name = "ElastiCacheSubnetGroupPage", params(ElastiCacheSubnetGroup))))]
#[cfg_attr(feature = "cloudfront", graphql(concrete(name = "CfDistributionPage", params(CfDistribution))))]
#[cfg_attr(feature = "secretsmanager", graphql(concrete(name = "SecretPage", params(Secret))))]
#[cfg_attr(feature = "lightsail", graphql(concrete(name = "LightsailInstancePage", params(LightsailInstance))))]
#[cfg_attr(feature = "lightsail", graphql(concrete(name = "LightsailDatabasePage", params(LightsailDatabase))))]
#[cfg_attr(feature = "lightsail", graphql(concrete(name = "LightsailLoadBalancerPage", params(LightsailLoadBalancer))))]
#[cfg_attr(feature = "lightsail", graphql(concrete(name = "LightsailStaticIpPage", params(LightsailStaticIp))))]
#[cfg_attr(feature = "apigateway", graphql(concrete(name = "ApigwRestApiPage", params(ApigwRestApi))))]
#[cfg_attr(feature = "apigateway", graphql(concrete(name = "ApigwResourcePage", params(ApigwResource))))]
#[cfg_attr(feature = "apigateway", graphql(concrete(name = "ApigwDeploymentPage", params(ApigwDeployment))))]
#[cfg_attr(feature = "xray", graphql(concrete(name = "XRayGroupPage", params(XRayGroup))))]
#[cfg_attr(feature = "xray", graphql(concrete(name = "XRaySamplingRulePage", params(XRaySamplingRule))))]
#[cfg_attr(feature = "glue", graphql(concrete(name = "GlueDatabasePage", params(GlueDatabase))))]
#[cfg_attr(feature = "glue", graphql(concrete(name = "GlueTablePage", params(GlueTable))))]
#[cfg_attr(feature = "glue", graphql(concrete(name = "GlueCrawlerPage", params(GlueCrawler))))]
#[cfg_attr(feature = "glue", graphql(concrete(name = "GlueJobPage", params(GlueJob))))]
#[cfg_attr(feature = "dms", graphql(concrete(name = "DmsReplicationInstancePage", params(DmsReplicationInstance))))]
#[cfg_attr(feature = "dms", graphql(concrete(name = "DmsEndpointPage", params(DmsEndpoint))))]
#[cfg_attr(feature = "dms", graphql(concrete(name = "DmsReplicationTaskPage", params(DmsReplicationTask))))]
#[cfg_attr(feature = "wafv2", graphql(concrete(name = "WebAclPage", params(WebAcl))))]
#[cfg_attr(feature = "wafv2", graphql(concrete(name = "WafIpSetPage", params(WafIpSet))))]
#[cfg_attr(feature = "wafv2", graphql(concrete(name = "WafRuleGroupPage", params(WafRuleGroup))))]
#[cfg_attr(feature = "elbv2", graphql(concrete(name = "LoadBalancerPage", params(LoadBalancer))))]
#[cfg_attr(feature = "elbv2", graphql(concrete(name = "TargetGroupPage", params(TargetGroup))))]
#[cfg_attr(feature = "elbv2", graphql(concrete(name = "ListenerPage", params(Listener))))]
#[cfg_attr(feature = "elbv2", graphql(concrete(name = "ListenerRulePage", params(ListenerRule))))]
#[cfg_attr(feature = "kms", graphql(concrete(name = "KmsKeyPage", params(KmsKey))))]
#[cfg_attr(feature = "kms", graphql(concrete(name = "KmsAliasPage", params(KmsAlias))))]
#[cfg_attr(feature = "guardduty", graphql(concrete(name = "DetectorPage", params(Detector))))]
#[cfg_attr(feature = "guardduty", graphql(concrete(name = "FindingPage", params(Finding))))]
#[cfg_attr(feature = "budgets", graphql(concrete(name = "BudgetPage", params(Budget))))]
#[cfg_attr(feature = "budgets", graphql(concrete(name = "BudgetNotificationPage", params(BudgetNotification))))]
#[cfg_attr(feature = "athena", graphql(concrete(name = "AthenaWorkgroupPage", params(AthenaWorkgroup))))]
#[cfg_attr(feature = "athena", graphql(concrete(name = "AthenaNamedQueryPage", params(AthenaNamedQuery))))]
#[cfg_attr(feature = "athena", graphql(concrete(name = "AthenaQueryExecutionPage", params(AthenaQueryExecution))))]
#[cfg_attr(feature = "auditmanager", graphql(concrete(name = "AuditManagerAssessmentPage", params(AuditManagerAssessment))))]
#[cfg_attr(feature = "auditmanager", graphql(concrete(name = "AuditManagerFrameworkPage", params(AuditManagerFramework))))]
#[cfg_attr(feature = "auditmanager", graphql(concrete(name = "AuditManagerControlPage", params(AuditManagerControl))))]
#[cfg_attr(feature = "comprehend", graphql(concrete(name = "ComprehendEntityRecognizerPage", params(ComprehendEntityRecognizer))))]
#[cfg_attr(feature = "comprehend", graphql(concrete(name = "ComprehendDocumentClassifierPage", params(ComprehendDocumentClassifier))))]
#[cfg_attr(feature = "comprehend", graphql(concrete(name = "ComprehendEndpointPage", params(ComprehendEndpoint))))]
#[cfg_attr(feature = "shield", graphql(concrete(name = "ShieldProtectionPage", params(ShieldProtection))))]
#[cfg_attr(feature = "shield", graphql(concrete(name = "ProtectionGroupPage", params(ProtectionGroup))))]
#[cfg_attr(feature = "shield", graphql(concrete(name = "AttackSummaryPage", params(AttackSummary))))]
#[cfg_attr(feature = "codeartifact", graphql(concrete(name = "CodeArtifactDomainPage", params(CodeArtifactDomain))))]
#[cfg_attr(feature = "codeartifact", graphql(concrete(name = "CodeArtifactRepositoryPage", params(CodeArtifactRepository))))]
#[cfg_attr(feature = "codeartifact", graphql(concrete(name = "CodeArtifactPackagePage", params(CodeArtifactPackage))))]
#[cfg_attr(feature = "rekognition", graphql(concrete(name = "RekognitionCollectionPage", params(RekognitionCollection))))]
#[cfg_attr(feature = "rekognition", graphql(concrete(name = "RekognitionProjectPage", params(RekognitionProject))))]
#[cfg_attr(feature = "rekognition", graphql(concrete(name = "RekognitionStreamProcessorPage", params(RekognitionStreamProcessor))))]
#[cfg_attr(feature = "keyspaces", graphql(concrete(name = "KeyspacesKeyspacePage", params(KeyspacesKeyspace))))]
#[cfg_attr(feature = "keyspaces", graphql(concrete(name = "KeyspacesTablePage", params(KeyspacesTable))))]
#[cfg_attr(feature = "fsx", graphql(concrete(name = "FsxFileSystemPage", params(FsxFileSystem))))]
#[cfg_attr(feature = "fsx", graphql(concrete(name = "FsxBackupPage", params(FsxBackup))))]
#[cfg_attr(feature = "fsx", graphql(concrete(name = "FsxStorageVirtualMachinePage", params(FsxStorageVirtualMachine))))]
#[cfg_attr(feature = "transcribe", graphql(concrete(name = "TranscriptionJobPage", params(TranscriptionJob))))]
#[cfg_attr(feature = "transcribe", graphql(concrete(name = "TranscribeVocabularyPage", params(TranscribeVocabulary))))]
#[cfg_attr(feature = "transcribe", graphql(concrete(name = "TranscribeLanguageModelPage", params(TranscribeLanguageModel))))]
#[cfg_attr(feature = "cloudformation", graphql(concrete(name = "CfnStackPage", params(CfnStack))))]
#[cfg_attr(feature = "cloudformation", graphql(concrete(name = "CfnStackResourcePage", params(CfnStackResource))))]
#[cfg_attr(feature = "cloudformation", graphql(concrete(name = "CfnExportPage", params(CfnExport))))]
#[cfg_attr(feature = "s3", graphql(concrete(name = "S3BucketPage", params(S3Bucket))))]
#[cfg_attr(feature = "sfn", graphql(concrete(name = "StateMachinePage", params(StateMachine))))]
#[cfg_attr(feature = "sfn", graphql(concrete(name = "ExecutionPage", params(Execution))))]
#[cfg_attr(feature = "translate", graphql(concrete(name = "TranslateTerminologyPage", params(TranslateTerminology))))]
#[cfg_attr(feature = "translate", graphql(concrete(name = "TranslateParallelDataPage", params(TranslateParallelData))))]
#[cfg_attr(feature = "translate", graphql(concrete(name = "TranslateTextTranslationJobPage", params(TranslateTextTranslationJob))))]
#[cfg_attr(feature = "workspaces", graphql(concrete(name = "WorkspacePage", params(Workspace))))]
#[cfg_attr(feature = "workspaces", graphql(concrete(name = "WorkspaceDirectoryPage", params(WorkspaceDirectory))))]
#[cfg_attr(feature = "workspaces", graphql(concrete(name = "WorkspaceBundlePage", params(WorkspaceBundle))))]
#[cfg_attr(feature = "quicksight", graphql(concrete(name = "QuickSightUserPage", params(QuickSightUser))))]
#[cfg_attr(feature = "quicksight", graphql(concrete(name = "QuickSightDashboardPage", params(QuickSightDashboard))))]
#[cfg_attr(feature = "quicksight", graphql(concrete(name = "QuickSightDataSetPage", params(QuickSightDataSet))))]
#[cfg_attr(feature = "quicksight", graphql(concrete(name = "QuickSightDataSourcePage", params(QuickSightDataSource))))]
#[cfg_attr(feature = "datasync", graphql(concrete(name = "DataSyncAgentPage", params(DataSyncAgent))))]
#[cfg_attr(feature = "datasync", graphql(concrete(name = "DataSyncLocationPage", params(DataSyncLocation))))]
#[cfg_attr(feature = "datasync", graphql(concrete(name = "DataSyncTaskPage", params(DataSyncTask))))]
#[cfg_attr(feature = "datasync", graphql(concrete(name = "DataSyncTaskExecutionPage", params(DataSyncTaskExecution))))]
#[cfg_attr(feature = "lakeformation", graphql(concrete(name = "LakeFormationResourcePage", params(LakeFormationResource))))]
#[cfg_attr(feature = "lakeformation", graphql(concrete(name = "LakeFormationPermissionPage", params(LakeFormationPermission))))]
#[cfg_attr(feature = "lambda", graphql(concrete(name = "LambdaFunctionPage", params(LambdaFunction))))]
#[cfg_attr(feature = "lambda", graphql(concrete(name = "LambdaAliasPage", params(LambdaAlias))))]
#[cfg_attr(feature = "lambda", graphql(concrete(name = "LambdaEventSourceMappingPage", params(LambdaEventSourceMapping))))]
#[cfg_attr(feature = "lambda", graphql(concrete(name = "LambdaLayerPage", params(LambdaLayer))))]
#[cfg_attr(feature = "codecommit", graphql(concrete(name = "CodeCommitRepositoryPage", params(CodeCommitRepository))))]
#[cfg_attr(feature = "codecommit", graphql(concrete(name = "CodeCommitBranchPage", params(CodeCommitBranch))))]
#[cfg_attr(feature = "codecommit", graphql(concrete(name = "CodeCommitPullRequestPage", params(CodeCommitPullRequest))))]
#[cfg_attr(feature = "iot", graphql(concrete(name = "IotThingPage", params(IotThing))))]
#[cfg_attr(feature = "iot", graphql(concrete(name = "IotThingGroupPage", params(IotThingGroup))))]
#[cfg_attr(feature = "iot", graphql(concrete(name = "IotPolicyPage", params(IotPolicy))))]
#[cfg_attr(feature = "iot", graphql(concrete(name = "IotCertificatePage", params(IotCertificate))))]
#[cfg_attr(feature = "iot", graphql(concrete(name = "IotTopicRulePage", params(IotTopicRule))))]
#[cfg_attr(feature = "sesv2", graphql(concrete(name = "SesIdentityPage", params(SesIdentity))))]
#[cfg_attr(feature = "sesv2", graphql(concrete(name = "SesConfigurationSetPage", params(SesConfigurationSet))))]
#[cfg_attr(feature = "sesv2", graphql(concrete(name = "SesEmailTemplatePage", params(SesEmailTemplate))))]
#[cfg_attr(feature = "sesv2", graphql(concrete(name = "SesSuppressedDestinationPage", params(SesSuppressedDestination))))]
#[cfg_attr(feature = "connect", graphql(concrete(name = "ConnectInstancePage", params(ConnectInstance))))]
#[cfg_attr(feature = "connect", graphql(concrete(name = "ConnectQueuePage", params(ConnectQueue))))]
#[cfg_attr(feature = "connect", graphql(concrete(name = "ConnectContactFlowPage", params(ConnectContactFlow))))]
#[cfg_attr(feature = "connect", graphql(concrete(name = "ConnectUserPage", params(ConnectUser))))]
#[cfg_attr(feature = "bedrock", graphql(concrete(name = "BedrockCustomModelPage", params(BedrockCustomModel))))]
#[cfg_attr(feature = "bedrock", graphql(concrete(name = "BedrockGuardrailPage", params(BedrockGuardrail))))]
#[cfg_attr(feature = "ecr", graphql(concrete(name = "EcrRepositoryPage", params(EcrRepository))))]
#[cfg_attr(feature = "ecr", graphql(concrete(name = "EcrImagePage", params(EcrImage))))]
#[cfg_attr(feature = "networkfirewall", graphql(concrete(name = "FirewallPage", params(Firewall))))]
#[cfg_attr(feature = "networkfirewall", graphql(concrete(name = "FirewallPolicyPage", params(FirewallPolicy))))]
#[cfg_attr(feature = "networkfirewall", graphql(concrete(name = "RuleGroupPage", params(RuleGroup))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "RouteTablePage", params(RouteTable))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "NetworkAclPage", params(NetworkAcl))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "InternetGatewayPage", params(InternetGateway))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "NatGatewayPage", params(NatGateway))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "VpcEndpointPage", params(VpcEndpoint))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "TransitGatewayPage", params(TransitGateway))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "VpcFlowLogPage", params(VpcFlowLog))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "InstancePage", params(Instance))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "SecurityGroupPage", params(SecurityGroup))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "VpcPage", params(Vpc))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "SubnetPage", params(Subnet))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "VolumePage", params(Volume))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "ImagePage", params(Image))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "LaunchTemplatePage", params(LaunchTemplate))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "LaunchTemplateVersionPage", params(LaunchTemplateVersion))))]
#[cfg_attr(feature = "ec2", graphql(concrete(name = "SnapshotPage", params(Snapshot))))]
#[cfg_attr(feature = "apprunner", graphql(concrete(name = "AppRunnerServicePage", params(AppRunnerService))))]
#[cfg_attr(feature = "apprunner", graphql(concrete(name = "AppRunnerVpcConnectorPage", params(AppRunnerVpcConnector))))]
#[cfg_attr(feature = "apprunner", graphql(concrete(name = "AppRunnerConnectionPage", params(AppRunnerConnection))))]
#[cfg_attr(feature = "apprunner", graphql(concrete(name = "AppRunnerObservabilityConfigurationPage", params(AppRunnerObservabilityConfiguration))))]
#[cfg_attr(feature = "eks", graphql(concrete(name = "EksClusterPage", params(EksCluster))))]
#[cfg_attr(feature = "eks", graphql(concrete(name = "EksNodegroupPage", params(EksNodegroup))))]
#[cfg_attr(feature = "eks", graphql(concrete(name = "EksFargateProfilePage", params(EksFargateProfile))))]
#[cfg_attr(feature = "eks", graphql(concrete(name = "EksAddonPage", params(EksAddon))))]
#[cfg_attr(feature = "rds", graphql(concrete(name = "DbInstancePage", params(DbInstance))))]
#[cfg_attr(feature = "rds", graphql(concrete(name = "DbClusterPage", params(DbCluster))))]
#[cfg_attr(feature = "rds", graphql(concrete(name = "DbSnapshotPage", params(DbSnapshot))))]
#[cfg_attr(feature = "rds", graphql(concrete(name = "DbParameterGroupPage", params(DbParameterGroup))))]
#[cfg_attr(feature = "rds", graphql(concrete(name = "DbSubnetGroupPage", params(DbSubnetGroup))))]
#[cfg_attr(feature = "ssm", graphql(concrete(name = "ManagedInstancePage", params(ManagedInstance))))]
#[cfg_attr(feature = "ssm", graphql(concrete(name = "SsmParameterPage", params(Parameter))))]
#[cfg_attr(feature = "ssm", graphql(concrete(name = "ParameterMetaPage", params(ParameterMeta))))]
#[cfg_attr(feature = "ssm", graphql(concrete(name = "SsmDocumentPage", params(SsmDocument))))]
#[cfg_attr(feature = "ecs", graphql(concrete(name = "EcsClusterPage", params(EcsCluster))))]
#[cfg_attr(feature = "ecs", graphql(concrete(name = "EcsServicePage", params(EcsService))))]
#[cfg_attr(feature = "ecs", graphql(concrete(name = "EcsTaskPage", params(EcsTask))))]
#[cfg_attr(feature = "iam", graphql(concrete(name = "IamRolePage", params(IamRole))))]
#[cfg_attr(feature = "iam", graphql(concrete(name = "IamPolicyPage", params(IamPolicy))))]
#[cfg_attr(feature = "iam", graphql(concrete(name = "IamUserPage", params(IamUser))))]
#[cfg_attr(feature = "iam", graphql(concrete(name = "IamGroupPage", params(IamGroup))))]
#[cfg_attr(feature = "iam", graphql(concrete(name = "IamAttachedPolicyPage", params(IamAttachedPolicy))))]
#[cfg_attr(feature = "iam", graphql(concrete(name = "IamAccessKeyPage", params(IamAccessKey))))]
#[cfg_attr(feature = "iam", graphql(concrete(name = "IamMfaDevicePage", params(IamMfaDevice))))]
#[cfg_attr(feature = "iam", graphql(concrete(name = "IamInlinePolicyPage", params(IamInlinePolicy))))]
pub struct Page<T: OutputType> {
    pub items: Vec<T>,
    /// Opaque continuation token; pass back as `nextToken` to resume.
    pub next_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_holds_items_and_token() {
        let page = Page {
            items: vec!["a".to_string(), "b".to_string()],
            next_token: Some("tok".to_string()),
        };
        assert_eq!(page.items, vec!["a", "b"]);
        assert_eq!(page.next_token.as_deref(), Some("tok"));
    }

    #[test]
    fn page_with_no_more_pages_has_no_token() {
        let page = Page {
            items: vec!["only".to_string()],
            next_token: None,
        };
        assert_eq!(page.next_token, None);
    }
}
