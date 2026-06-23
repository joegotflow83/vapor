//! Root GraphQL objects, composed from per-service modules.
//!
//! Each AWS service contributes its own `#[Object]` query/mutation type
//! (e.g. `crate::schema::ec2::queries::Ec2Query`). They are merged here with
//! `#[derive(MergedObject)]`, gated by the same Cargo feature that enables the
//! service's client. Adding a service end-to-end is then:
//!   1. declare its `crate::schema::<svc>` module,
//!   2. add one `#[cfg(feature = "<svc>")]` field to each root below,
//!   3. register its client in `crate::schema::root::build_schema`.

use async_graphql::{MergedObject, Object};

/// Always-present base query so the root is non-empty even when every
/// service feature is disabled.
#[derive(Default)]
pub struct BaseQuery;

#[Object]
impl BaseQuery {
    /// Liveness / identification field.
    async fn placeholder(&self) -> &'static str {
        "vapor"
    }
}

/// Always-present base mutation (keeps the root non-empty).
#[derive(Default)]
pub struct BaseMutation;

#[Object]
impl BaseMutation {
    async fn _placeholder(&self) -> bool {
        false
    }
}

/// Composed query root — one field per enabled service.
#[derive(MergedObject, Default)]
pub struct QueryRoot(
    BaseQuery,
    #[cfg(feature = "ec2")] crate::schema::ec2::queries::Ec2Query,
    #[cfg(feature = "ec2")] crate::schema::vpc::queries::VpcQuery,
    #[cfg(feature = "s3")] crate::schema::s3::queries::S3Query,
    #[cfg(feature = "lambda")] crate::schema::lambda::queries::LambdaQuery,
    #[cfg(feature = "ssm")] crate::schema::ssm::queries::SsmQuery,
    #[cfg(feature = "ecs")] crate::schema::ecs::queries::EcsQuery,
    #[cfg(feature = "eks")] crate::schema::eks::queries::EksQuery,
    #[cfg(feature = "ecr")] crate::schema::ecr::queries::EcrQuery,
    #[cfg(feature = "batch")] crate::schema::batch::queries::BatchQuery,
    #[cfg(feature = "elbv2")] crate::schema::elbv2::queries::Elbv2Query,
    #[cfg(feature = "autoscaling")] crate::schema::asg::queries::AsgQuery,
    #[cfg(feature = "dynamodb")] crate::schema::dynamodb::queries::DynamodbQuery,
    #[cfg(feature = "rds")] crate::schema::rds::queries::RdsQuery,
    #[cfg(feature = "efs")] crate::schema::efs::queries::EfsQuery,
    #[cfg(feature = "elasticache")] crate::schema::elasticache::queries::ElastiCacheQuery,
    #[cfg(feature = "redshift")] crate::schema::redshift::queries::RedshiftQuery,
    #[cfg(feature = "redshiftserverless")]
    crate::schema::redshift_serverless::queries::RedshiftServerlessQuery,
    #[cfg(feature = "memorydb")] crate::schema::memorydb::queries::MemoryDbQuery,
    #[cfg(feature = "neptune")] crate::schema::neptune::queries::NeptuneQuery,
    #[cfg(feature = "docdb")] crate::schema::documentdb::queries::DocumentDbQuery,
    #[cfg(feature = "athena")] crate::schema::athena::queries::AthenaQuery,
    #[cfg(feature = "glue")] crate::schema::glue::queries::GlueQuery,
    #[cfg(feature = "emr")] crate::schema::emr::queries::EmrQuery,
    #[cfg(feature = "kinesis")] crate::schema::kinesis::queries::KinesisQuery,
    #[cfg(feature = "firehose")] crate::schema::firehose::queries::FirehoseQuery,
    #[cfg(feature = "kafka")] crate::schema::msk::queries::MskQuery,
    #[cfg(feature = "route53")] crate::schema::route53::queries::Route53Query,
    #[cfg(feature = "cloudfront")] crate::schema::cloudfront::queries::CloudFrontQuery,
    #[cfg(feature = "apigateway")] crate::schema::apigateway::queries::ApiGatewayQuery,
    #[cfg(feature = "apigatewayv2")] crate::schema::apigatewayv2::queries::ApiGatewayV2Query,
    #[cfg(feature = "globalaccelerator")]
    crate::schema::global_accelerator::queries::GlobalAcceleratorQuery,
    #[cfg(feature = "directconnect")]
    crate::schema::direct_connect::queries::DirectConnectQuery,
    #[cfg(feature = "networkfirewall")]
    crate::schema::network_firewall::queries::NetworkFirewallQuery,
    #[cfg(feature = "iam")] crate::schema::iam::queries::IamQuery,
    #[cfg(feature = "kms")] crate::schema::kms::queries::KmsQuery,
    #[cfg(feature = "secretsmanager")]
    crate::schema::secrets_manager::queries::SecretsManagerQuery,
    #[cfg(feature = "acm")] crate::schema::acm::queries::AcmQuery,
    #[cfg(feature = "cognitoidentityprovider")] crate::schema::cognito::queries::CognitoQuery,
    #[cfg(feature = "guardduty")] crate::schema::guardduty::queries::GuardDutyQuery,
    #[cfg(feature = "inspector2")] crate::schema::inspector::queries::InspectorQuery,
    #[cfg(feature = "securityhub")] crate::schema::security_hub::queries::SecurityHubQuery,
    #[cfg(feature = "macie2")] crate::schema::macie::queries::MacieQuery,
    #[cfg(feature = "shield")] crate::schema::shield::queries::ShieldQuery,
    #[cfg(feature = "wafv2")] crate::schema::wafv2::queries::Wafv2Query,
    #[cfg(feature = "sts")] crate::schema::sts::queries::StsQuery,
    #[cfg(feature = "cloudwatch")] crate::schema::cloudwatch::queries::CloudWatchQuery,
    #[cfg(feature = "cloudtrail")] crate::schema::cloudtrail::queries::CloudTrailQuery,
    #[cfg(feature = "config")] crate::schema::config_svc::queries::AwsConfigQuery,
    #[cfg(feature = "cloudformation")]
    crate::schema::cloudformation::queries::CloudFormationQuery,
    #[cfg(feature = "codepipeline")] crate::schema::codepipeline::queries::CodePipelineQuery,
    #[cfg(feature = "codebuild")] crate::schema::codebuild::queries::CodeBuildQuery,
    #[cfg(feature = "codedeploy")] crate::schema::codedeploy::queries::CodeDeployQuery,
    #[cfg(feature = "sfn")] crate::schema::step_functions::queries::StepFunctionsQuery,
    #[cfg(feature = "eventbridge")] crate::schema::eventbridge::queries::EventBridgeQuery,
    #[cfg(feature = "sns")] crate::schema::sns::queries::SnsQuery,
    #[cfg(feature = "sqs")] crate::schema::sqs::queries::SqsQuery,
    #[cfg(feature = "servicequotas")] crate::schema::service_quotas::queries::ServiceQuotasQuery,
    #[cfg(feature = "health")] crate::schema::health::queries::HealthQuery,
    #[cfg(feature = "organizations")] crate::schema::organizations::queries::OrganizationsQuery,
    #[cfg(feature = "appconfig")] crate::schema::appconfig::queries::AppConfigQuery,
    #[cfg(feature = "appsync")] crate::schema::appsync::queries::AppSyncQuery,
    #[cfg(feature = "costexplorer")] crate::schema::cost_explorer::queries::CostExplorerQuery,
    #[cfg(feature = "sagemaker")] crate::schema::sagemaker::queries::SageMakerQuery,
    #[cfg(feature = "transfer")] crate::schema::transfer::queries::TransferQuery,
    #[cfg(feature = "opensearch")] crate::schema::opensearch::queries::OpenSearchQuery,
    #[cfg(feature = "backup")] crate::schema::backup::queries::BackupQuery,
);

/// Composed mutation root — one field per enabled service.
#[derive(MergedObject, Default)]
pub struct MutationRoot(
    BaseMutation,
    #[cfg(feature = "ec2")] crate::schema::ec2::mutations::Ec2Mutation,
);
