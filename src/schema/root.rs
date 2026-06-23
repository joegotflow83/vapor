//! Schema construction: compose the roots and inject AWS clients into context.
//!
//! Clients are registered with `.data(...)` so resolvers can retrieve them via
//! `ctx.data::<T>()`. Each registration is gated by the same Cargo feature that
//! enables the corresponding service in the roots.

use async_graphql::{EmptySubscription, Schema};
use aws_config::SdkConfig;

use crate::schema::aws::registry::{MutationRoot, QueryRoot};

/// Build the GraphQL schema, registering an AWS client for each enabled
/// service so resolvers can pull them from the request context.
pub fn build_schema(config: &SdkConfig) -> Schema<QueryRoot, MutationRoot, EmptySubscription> {
    #[allow(unused_mut)]
    let mut builder =
        Schema::build(QueryRoot::default(), MutationRoot::default(), EmptySubscription);

    #[cfg(feature = "ec2")]
    {
        builder = builder.data(crate::aws::ec2::Ec2Client::new(config));
    }

    #[cfg(feature = "s3")]
    {
        builder = builder.data(crate::aws::s3::S3Client::new(config));
    }

    #[cfg(feature = "lambda")]
    {
        builder = builder.data(crate::aws::lambda::LambdaClient::new(config));
    }

    #[cfg(feature = "ssm")]
    {
        builder = builder.data(crate::aws::ssm::SsmClient::new(config));
    }

    #[cfg(feature = "ecs")]
    {
        builder = builder.data(crate::aws::ecs::EcsClient::new(config));
    }

    #[cfg(feature = "eks")]
    {
        builder = builder.data(crate::aws::eks::EksClient::new(config));
    }

    #[cfg(feature = "ecr")]
    {
        builder = builder.data(crate::aws::ecr::EcrClient::new(config));
    }

    #[cfg(feature = "batch")]
    {
        builder = builder.data(crate::aws::batch::BatchClient::new(config));
    }

    #[cfg(feature = "elbv2")]
    {
        builder = builder.data(crate::aws::elbv2::Elbv2Client::new(config));
    }

    #[cfg(feature = "autoscaling")]
    {
        builder = builder.data(crate::aws::autoscaling::AutoscalingClient::new(config));
    }

    #[cfg(feature = "dynamodb")]
    {
        builder = builder.data(crate::aws::dynamodb::DynamodbClient::new(config));
    }

    #[cfg(feature = "rds")]
    {
        builder = builder.data(crate::aws::rds::RdsClient::new(config));
    }

    #[cfg(feature = "efs")]
    {
        builder = builder.data(crate::aws::efs::EfsClient::new(config));
    }

    #[cfg(feature = "elasticache")]
    {
        builder = builder.data(crate::aws::elasticache::ElastiCacheClient::new(config));
    }

    #[cfg(feature = "redshift")]
    {
        builder = builder.data(crate::aws::redshift::RedshiftClient::new(config));
    }

    #[cfg(feature = "redshiftserverless")]
    {
        builder =
            builder.data(crate::aws::redshift_serverless::RedshiftServerlessClient::new(config));
    }

    #[cfg(feature = "memorydb")]
    {
        builder = builder.data(crate::aws::memorydb::MemoryDbClient::new(config));
    }

    #[cfg(feature = "neptune")]
    {
        builder = builder.data(crate::aws::neptune::NeptuneClient::new(config));
    }

    #[cfg(feature = "docdb")]
    {
        builder = builder.data(crate::aws::documentdb::DocumentDbClient::new(config));
    }

    #[cfg(feature = "athena")]
    {
        builder = builder.data(crate::aws::athena::AthenaClient::new(config));
    }

    #[cfg(feature = "glue")]
    {
        builder = builder.data(crate::aws::glue::GlueClient::new(config));
    }

    #[cfg(feature = "emr")]
    {
        builder = builder.data(crate::aws::emr::EmrClient::new(config));
    }

    #[cfg(feature = "kinesis")]
    {
        builder = builder.data(crate::aws::kinesis::KinesisClient::new(config));
    }

    #[cfg(feature = "firehose")]
    {
        builder = builder.data(crate::aws::firehose::FirehoseClient::new(config));
    }

    #[cfg(feature = "kafka")]
    {
        builder = builder.data(crate::aws::msk::MskClient::new(config));
    }

    #[cfg(feature = "route53")]
    {
        builder = builder.data(crate::aws::route53::Route53Client::new(config));
    }

    #[cfg(feature = "cloudfront")]
    {
        builder = builder.data(crate::aws::cloudfront::CloudFrontClient::new(config));
    }

    #[cfg(feature = "apigateway")]
    {
        builder = builder.data(crate::aws::apigateway::ApiGatewayClient::new(config));
    }

    #[cfg(feature = "apigatewayv2")]
    {
        builder = builder.data(crate::aws::apigatewayv2::ApiGatewayV2Client::new(config));
    }

    #[cfg(feature = "globalaccelerator")]
    {
        builder = builder.data(crate::aws::global_accelerator::GlobalAcceleratorClient::new(config));
    }

    #[cfg(feature = "directconnect")]
    {
        builder = builder.data(crate::aws::direct_connect::DirectConnectClient::new(config));
    }

    #[cfg(feature = "networkfirewall")]
    {
        builder = builder.data(crate::aws::network_firewall::NetworkFirewallClient::new(config));
    }

    #[cfg(feature = "iam")]
    {
        builder = builder.data(crate::aws::iam::IamClient::new(config));
    }

    #[cfg(feature = "kms")]
    {
        builder = builder.data(crate::aws::kms::KmsClient::new(config));
    }

    #[cfg(feature = "secretsmanager")]
    {
        builder = builder.data(crate::aws::secrets_manager::SecretsManagerClient::new(config));
    }

    #[cfg(feature = "acm")]
    {
        builder = builder.data(crate::aws::acm::AcmClient::new(config));
    }

    #[cfg(feature = "cognitoidentityprovider")]
    {
        builder = builder.data(crate::aws::cognito::CognitoClient::new(config));
    }

    #[cfg(feature = "guardduty")]
    {
        builder = builder.data(crate::aws::guardduty::GuardDutyClient::new(config));
    }

    #[cfg(feature = "inspector2")]
    {
        builder = builder.data(crate::aws::inspector::InspectorClient::new(config));
    }

    #[cfg(feature = "securityhub")]
    {
        builder = builder.data(crate::aws::security_hub::SecurityHubClient::new(config));
    }

    #[cfg(feature = "macie2")]
    {
        builder = builder.data(crate::aws::macie::MacieClient::new(config));
    }

    #[cfg(feature = "shield")]
    {
        builder = builder.data(crate::aws::shield::ShieldClient::new(config));
    }

    #[cfg(feature = "wafv2")]
    {
        builder = builder.data(crate::aws::wafv2::WafV2Client::new(config));
    }

    #[cfg(feature = "sts")]
    {
        builder = builder.data(crate::aws::sts::StsClient::new(config));
    }

    #[cfg(feature = "cloudwatch")]
    {
        builder = builder.data(crate::aws::cloudwatch::CloudWatchClient::new(config));
    }

    #[cfg(feature = "cloudwatchlogs")]
    {
        builder = builder.data(crate::aws::cloudwatch_logs::CloudWatchLogsClient::new(config));
    }

    #[cfg(feature = "cloudtrail")]
    {
        builder = builder.data(crate::aws::cloudtrail::CloudTrailClient::new(config));
    }

    #[cfg(feature = "config")]
    {
        builder = builder.data(crate::aws::config_svc::AwsConfigClient::new(config));
    }

    #[cfg(feature = "cloudformation")]
    {
        builder = builder.data(crate::aws::cloudformation::CloudFormationClient::new(config));
    }

    #[cfg(feature = "codepipeline")]
    {
        builder = builder.data(crate::aws::codepipeline::CodePipelineClient::new(config));
    }

    #[cfg(feature = "codebuild")]
    {
        builder = builder.data(crate::aws::codebuild::CodeBuildClient::new(config));
    }

    #[cfg(feature = "codedeploy")]
    {
        builder = builder.data(crate::aws::codedeploy::CodeDeployClient::new(config));
    }

    #[cfg(feature = "sfn")]
    {
        builder = builder.data(crate::aws::step_functions::StepFunctionsClient::new(config));
    }

    #[cfg(feature = "eventbridge")]
    {
        builder = builder.data(crate::aws::eventbridge::EventBridgeClient::new(config));
    }

    #[cfg(feature = "sns")]
    {
        builder = builder.data(crate::aws::sns::SnsClient::new(config));
    }

    #[cfg(feature = "sqs")]
    {
        builder = builder.data(crate::aws::sqs::SqsClient::new(config));
    }

    #[cfg(feature = "servicequotas")]
    {
        builder = builder.data(crate::aws::service_quotas::ServiceQuotasClient::new(config));
    }

    #[cfg(feature = "health")]
    {
        builder = builder.data(crate::aws::health::HealthClient::new(config));
    }

    #[cfg(feature = "organizations")]
    {
        builder = builder.data(crate::aws::organizations::OrganizationsClient::new(config));
    }

    #[cfg(feature = "appconfig")]
    {
        builder = builder.data(crate::aws::appconfig::AppConfigClient::new(config));
    }

    #[cfg(feature = "appsync")]
    {
        builder = builder.data(crate::aws::appsync::AppSyncClient::new(config));
    }

    #[cfg(feature = "costexplorer")]
    {
        builder = builder.data(crate::aws::cost_explorer::CostExplorerClient::new(config));
    }

    #[cfg(feature = "sagemaker")]
    {
        builder = builder.data(crate::aws::sagemaker::SageMakerClient::new(config));
    }

    #[cfg(feature = "transfer")]
    {
        builder = builder.data(crate::aws::transfer::TransferClient::new(config));
    }

    #[cfg(feature = "opensearch")]
    {
        builder = builder.data(crate::aws::opensearch::OpenSearchClient::new(config));
    }

    #[cfg(feature = "backup")]
    {
        builder = builder.data(crate::aws::backup::BackupClient::new(config));
    }

    builder.finish()
}
