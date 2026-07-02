//! Generates the mdBook service pages under `docs/src/` from the live GraphQL
//! schema, rather than from hand-maintained prose (see README.md's old
//! `## GraphQL Schema` section for the manual version this replaces).
//!
//! Must be run with `--all-features` — it references every service's Query
//! type directly, mirroring `src/schema/aws/registry.rs`'s `QueryRoot` tuple.
//! Run via `cargo run --all-features --bin gen-docs`.
//!
//! Output (`docs/src/services/*.md`, `docs/src/SUMMARY.md`) is generated,
//! not committed — see `.gitignore`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use async_graphql::{EmptyMutation, EmptySubscription, ObjectType, OutputType, Schema};

struct ServicePage {
    slug: &'static str,
    title: &'static str,
    feature: &'static str,
    note: Option<&'static str>,
    /// The GraphQL type name of this service's query root (the `#[Object]`
    /// struct's own name, e.g. `AcmQuery` — async-graphql does *not*
    /// rename merged-in roots to the spec's conventional `Query`). Needed
    /// to find the right `type X { ... }` block in `sdl` for the drift
    /// check below.
    query_type_name: String,
    sdl: String,
}

fn sdl_for<Q>() -> (String, String)
where
    Q: ObjectType + Default + Send + Sync + 'static,
{
    let sdl = clean_sdl(&Schema::build(Q::default(), EmptyMutation, EmptySubscription).finish().sdl());
    (Q::type_name().into_owned(), sdl)
}

fn sdl_for_with_mutation<Q, M>() -> (String, String)
where
    Q: ObjectType + Default + Send + Sync + 'static,
    M: ObjectType + Default + Send + Sync + 'static,
{
    let sdl = clean_sdl(&Schema::build(Q::default(), M::default(), EmptySubscription).finish().sdl());
    (Q::type_name().into_owned(), sdl)
}

/// Strips the `schema { ... }` root-operation block and any empty
/// `EmptyMutation`/`EmptySubscription` type stubs that async-graphql may
/// emit, so each page shows only the types that matter for the service.
fn clean_sdl(sdl: &str) -> String {
    let mut out = sdl.to_string();
    for marker in ["schema {", "type EmptyMutation", "type EmptySubscription"] {
        out = strip_block(&out, marker);
    }
    out.trim().to_string()
}

fn strip_block(text: &str, start_marker: &str) -> String {
    let Some(marker_pos) = text.find(start_marker) else {
        return text.to_string();
    };
    let Some(brace_offset) = text[marker_pos..].find('{') else {
        return text.to_string();
    };
    let brace_start = marker_pos + brace_offset;

    let mut depth = 0i32;
    let mut end = None;
    for (i, c) in text[brace_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(brace_start + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(block_end) = end else {
        return text.to_string();
    };

    // Also eat one trailing newline so we don't leave a blank line behind.
    let after = if text[block_end..].starts_with('\n') { block_end + 1 } else { block_end };
    format!("{}{}", &text[..marker_pos], &text[after..])
}

/// Extracts the field names declared directly on `type <type_name> { ... }`,
/// skipping `"""..."""` descriptions. Used only for the drift check below.
fn query_field_names(sdl: &str, type_name: &str) -> BTreeSet<String> {
    let marker = format!("type {type_name} ");
    let Some(marker_pos) = sdl.find(&marker) else {
        return BTreeSet::new();
    };
    let Some(brace_offset) = sdl[marker_pos..].find('{') else {
        return BTreeSet::new();
    };
    let brace_start = marker_pos + brace_offset;

    let mut depth = 0i32;
    let mut body_end = sdl.len();
    let mut body_start = brace_start + 1;
    for (i, c) in sdl[brace_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    body_end = brace_start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    if body_start > body_end {
        body_start = body_end;
    }
    let body = &sdl[body_start..body_end];

    let mut fields = BTreeSet::new();
    let mut in_description = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"\"\"") {
            // Toggle unless it's a one-line `"""desc"""`.
            if trimmed.len() > 3 && trimmed.ends_with("\"\"\"") && trimmed != "\"\"\"" {
                continue;
            }
            in_description = !in_description;
            continue;
        }
        if in_description || trimmed.is_empty() {
            continue;
        }
        let name: String = trimmed.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            fields.insert(name);
        }
    }
    fields
}

fn render_page(page: &ServicePage) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", page.title));
    out.push_str(&format!("Cargo feature: `{}` (`cargo build --features {}`)\n\n", page.feature, page.feature));
    if let Some(note) = page.note {
        out.push_str(&format!("> {note}\n\n"));
    }
    out.push_str("```graphql\n");
    out.push_str(page.sdl.trim());
    out.push_str("\n```\n");
    out
}

fn render_summary(pages: &[ServicePage]) -> String {
    let mut out = String::new();
    out.push_str("# Summary\n\n");
    out.push_str("[Introduction](introduction.md)\n\n");
    out.push_str("# AWS Services\n\n");
    for page in pages {
        out.push_str(&format!("- [{}](services/{}.md)\n", page.title, page.slug));
    }
    out
}

fn main() {
    let mut pages: Vec<ServicePage> = Vec::new();

    macro_rules! page {
        ($pages:ident, $slug:expr, $title:expr, $feature:expr, $note:expr, $query:ty) => {{
            let (query_type_name, sdl) = sdl_for::<$query>();
            $pages.push(ServicePage {
                slug: $slug,
                title: $title,
                feature: $feature,
                note: $note,
                query_type_name,
                sdl,
            })
        }};
    }

    // EC2 is the only service with mutations (see registry.rs's MutationRoot),
    // so it's built with Ec2Mutation instead of EmptyMutation.
    let (ec2_query_type_name, ec2_sdl) = sdl_for_with_mutation::<
        vapor::schema::ec2::queries::Ec2Query,
        vapor::schema::ec2::mutations::Ec2Mutation,
    >();
    pages.push(ServicePage {
        slug: "ec2",
        title: "EC2",
        feature: "ec2",
        note: None,
        query_type_name: ec2_query_type_name,
        sdl: ec2_sdl,
    });

    page!(pages, "vpc", "VPC", "ec2", None, vapor::schema::vpc::queries::VpcQuery);
    page!(pages, "s3", "S3", "s3", None, vapor::schema::s3::queries::S3Query);
    page!(pages, "lambda", "Lambda", "lambda", None, vapor::schema::lambda::queries::LambdaQuery);
    page!(pages, "ssm", "Systems Manager", "ssm", None, vapor::schema::ssm::queries::SsmQuery);
    page!(pages, "ecs", "ECS", "ecs", None, vapor::schema::ecs::queries::EcsQuery);
    page!(pages, "eks", "EKS", "eks", None, vapor::schema::eks::queries::EksQuery);
    page!(pages, "ecr", "ECR", "ecr", None, vapor::schema::ecr::queries::EcrQuery);
    page!(pages, "batch", "Batch", "batch", None, vapor::schema::batch::queries::BatchQuery);
    page!(pages, "elbv2", "Elastic Load Balancing v2", "elbv2", None, vapor::schema::elbv2::queries::Elbv2Query);
    page!(pages, "asg", "Auto Scaling", "autoscaling", None, vapor::schema::asg::queries::AsgQuery);
    page!(pages, "dynamodb", "DynamoDB", "dynamodb", None, vapor::schema::dynamodb::queries::DynamodbQuery);
    page!(pages, "rds", "RDS", "rds", None, vapor::schema::rds::queries::RdsQuery);
    page!(pages, "efs", "EFS", "efs", None, vapor::schema::efs::queries::EfsQuery);
    page!(pages, "elasticache", "ElastiCache", "elasticache", None, vapor::schema::elasticache::queries::ElastiCacheQuery);
    page!(pages, "redshift", "Redshift", "redshift", None, vapor::schema::redshift::queries::RedshiftQuery);
    page!(pages, "redshift_serverless", "Redshift Serverless", "redshiftserverless", None, vapor::schema::redshift_serverless::queries::RedshiftServerlessQuery);
    page!(pages, "memorydb", "MemoryDB", "memorydb", None, vapor::schema::memorydb::queries::MemoryDbQuery);
    page!(pages, "neptune", "Neptune", "neptune", None, vapor::schema::neptune::queries::NeptuneQuery);
    page!(pages, "documentdb", "DocumentDB", "docdb", None, vapor::schema::documentdb::queries::DocumentDbQuery);
    page!(pages, "athena", "Athena", "athena", None, vapor::schema::athena::queries::AthenaQuery);
    page!(pages, "glue", "Glue", "glue", None, vapor::schema::glue::queries::GlueQuery);
    page!(pages, "emr", "EMR", "emr", None, vapor::schema::emr::queries::EmrQuery);
    page!(pages, "kinesis", "Kinesis Data Streams", "kinesis", None, vapor::schema::kinesis::queries::KinesisQuery);
    page!(pages, "firehose", "Kinesis Data Firehose", "firehose", None, vapor::schema::firehose::queries::FirehoseQuery);
    page!(pages, "msk", "MSK (Managed Streaming for Kafka)", "kafka", Some("Pulls in the `ec2` feature since broker AZ enrichment needs `describe_subnets`."), vapor::schema::msk::queries::MskQuery);
    page!(pages, "route53", "Route 53", "route53", None, vapor::schema::route53::queries::Route53Query);
    page!(pages, "cloudfront", "CloudFront", "cloudfront", None, vapor::schema::cloudfront::queries::CloudFrontQuery);
    page!(pages, "apigateway", "API Gateway (REST)", "apigateway", None, vapor::schema::apigateway::queries::ApiGatewayQuery);
    page!(pages, "apigatewayv2", "API Gateway v2 (HTTP/WebSocket)", "apigatewayv2", None, vapor::schema::apigatewayv2::queries::ApiGatewayV2Query);
    page!(pages, "global_accelerator", "Global Accelerator", "globalaccelerator", None, vapor::schema::global_accelerator::queries::GlobalAcceleratorQuery);
    page!(pages, "direct_connect", "Direct Connect", "directconnect", None, vapor::schema::direct_connect::queries::DirectConnectQuery);
    page!(pages, "network_firewall", "Network Firewall", "networkfirewall", None, vapor::schema::network_firewall::queries::NetworkFirewallQuery);
    page!(pages, "iam", "IAM", "iam", None, vapor::schema::iam::queries::IamQuery);
    page!(pages, "kms", "KMS", "kms", None, vapor::schema::kms::queries::KmsQuery);
    page!(pages, "secrets_manager", "Secrets Manager", "secretsmanager", None, vapor::schema::secrets_manager::queries::SecretsManagerQuery);
    page!(pages, "acm", "Certificate Manager", "acm", None, vapor::schema::acm::queries::AcmQuery);
    page!(pages, "cognito", "Cognito Identity Provider", "cognitoidentityprovider", None, vapor::schema::cognito::queries::CognitoQuery);
    page!(pages, "guardduty", "GuardDuty", "guardduty", None, vapor::schema::guardduty::queries::GuardDutyQuery);
    page!(pages, "inspector", "Inspector v2", "inspector2", None, vapor::schema::inspector::queries::InspectorQuery);
    page!(pages, "security_hub", "Security Hub", "securityhub", None, vapor::schema::security_hub::queries::SecurityHubQuery);
    page!(pages, "macie", "Macie v2", "macie2", None, vapor::schema::macie::queries::MacieQuery);
    page!(pages, "shield", "Shield", "shield", None, vapor::schema::shield::queries::ShieldQuery);
    page!(pages, "wafv2", "WAF v2", "wafv2", None, vapor::schema::wafv2::queries::Wafv2Query);
    page!(pages, "sts", "STS", "sts", None, vapor::schema::sts::queries::StsQuery);
    page!(pages, "cloudwatch", "CloudWatch (includes Logs)", "cloudwatch", None, vapor::schema::cloudwatch::queries::CloudWatchQuery);
    page!(pages, "cloudtrail", "CloudTrail", "cloudtrail", None, vapor::schema::cloudtrail::queries::CloudTrailQuery);
    page!(pages, "config_svc", "AWS Config", "config", None, vapor::schema::config_svc::queries::AwsConfigQuery);
    page!(pages, "cloudformation", "CloudFormation", "cloudformation", None, vapor::schema::cloudformation::queries::CloudFormationQuery);
    page!(pages, "codepipeline", "CodePipeline", "codepipeline", None, vapor::schema::codepipeline::queries::CodePipelineQuery);
    page!(pages, "codebuild", "CodeBuild", "codebuild", None, vapor::schema::codebuild::queries::CodeBuildQuery);
    page!(pages, "codedeploy", "CodeDeploy", "codedeploy", None, vapor::schema::codedeploy::queries::CodeDeployQuery);
    page!(pages, "step_functions", "Step Functions", "sfn", None, vapor::schema::step_functions::queries::StepFunctionsQuery);
    page!(pages, "eventbridge", "EventBridge", "eventbridge", None, vapor::schema::eventbridge::queries::EventBridgeQuery);
    page!(pages, "sns", "SNS", "sns", None, vapor::schema::sns::queries::SnsQuery);
    page!(pages, "sqs", "SQS", "sqs", None, vapor::schema::sqs::queries::SqsQuery);
    page!(pages, "service_quotas", "Service Quotas", "servicequotas", None, vapor::schema::service_quotas::queries::ServiceQuotasQuery);
    page!(pages, "health", "Health", "health", None, vapor::schema::health::queries::HealthQuery);
    page!(pages, "organizations", "Organizations", "organizations", None, vapor::schema::organizations::queries::OrganizationsQuery);
    page!(pages, "appconfig", "AppConfig", "appconfig", None, vapor::schema::appconfig::queries::AppConfigQuery);
    page!(pages, "appsync", "AppSync", "appsync", None, vapor::schema::appsync::queries::AppSyncQuery);
    page!(pages, "cost_explorer", "Cost Explorer", "costexplorer", None, vapor::schema::cost_explorer::queries::CostExplorerQuery);
    page!(pages, "sagemaker", "SageMaker", "sagemaker", None, vapor::schema::sagemaker::queries::SageMakerQuery);
    page!(pages, "transfer", "Transfer Family", "transfer", None, vapor::schema::transfer::queries::TransferQuery);
    page!(pages, "opensearch", "OpenSearch", "opensearch", None, vapor::schema::opensearch::queries::OpenSearchQuery);
    page!(pages, "backup", "Backup", "backup", None, vapor::schema::backup::queries::BackupQuery);
    page!(pages, "sso_admin", "IAM Identity Center (SSO Admin)", "ssoadmin", None, vapor::schema::sso_admin::queries::SsoAdminQuery);
    page!(pages, "acm_pca", "ACM Private CA", "acmpca", None, vapor::schema::acm_pca::queries::AcmPcaQuery);
    page!(pages, "ram", "Resource Access Manager", "ram", None, vapor::schema::ram::queries::RamQuery);
    page!(pages, "control_tower", "Control Tower", "controltower", None, vapor::schema::control_tower::queries::ControlTowerQuery);
    page!(pages, "fms", "Firewall Manager", "fms", None, vapor::schema::fms::queries::FmsQuery);
    page!(pages, "audit_manager", "Audit Manager", "auditmanager", None, vapor::schema::audit_manager::queries::AuditManagerQuery);
    page!(pages, "detective", "Detective", "detective", None, vapor::schema::detective::queries::DetectiveQuery);
    page!(pages, "ses", "Simple Email Service v2", "sesv2", None, vapor::schema::ses::queries::SesQuery);
    page!(pages, "elastic_beanstalk", "Elastic Beanstalk", "elasticbeanstalk", None, vapor::schema::elastic_beanstalk::queries::ElasticBeanstalkQuery);
    page!(pages, "app_runner", "App Runner", "apprunner", None, vapor::schema::app_runner::queries::AppRunnerQuery);
    page!(pages, "fsx", "FSx", "fsx", None, vapor::schema::fsx::queries::FsxQuery);
    page!(pages, "mq", "Amazon MQ", "mq", None, vapor::schema::mq::queries::MqQuery);
    page!(pages, "dms", "Database Migration Service", "dms", None, vapor::schema::dms::queries::DmsQuery);
    page!(pages, "workspaces", "WorkSpaces", "workspaces", None, vapor::schema::workspaces::queries::WorkspacesQuery);
    page!(pages, "storage_gateway", "Storage Gateway", "storagegateway", None, vapor::schema::storage_gateway::queries::StorageGatewayQuery);
    page!(pages, "datasync", "DataSync", "datasync", None, vapor::schema::datasync::queries::DataSyncQuery);
    page!(pages, "lightsail", "Lightsail", "lightsail", None, vapor::schema::lightsail::queries::LightsailQuery);
    page!(pages, "qldb", "QLDB", "qldb", None, vapor::schema::qldb::queries::QldbQuery);
    page!(pages, "keyspaces", "Amazon Keyspaces", "keyspaces", None, vapor::schema::keyspaces::queries::KeyspacesQuery);
    page!(pages, "bedrock", "Bedrock", "bedrock", None, vapor::schema::bedrock::queries::BedrockQuery);
    page!(pages, "xray", "X-Ray", "xray", None, vapor::schema::xray::queries::XRayQuery);
    page!(pages, "timestream", "Timestream", "timestream", None, vapor::schema::timestream::queries::TimestreamQuery);
    page!(pages, "lake_formation", "Lake Formation", "lakeformation", None, vapor::schema::lake_formation::queries::LakeFormationQuery);
    page!(pages, "quicksight", "QuickSight", "quicksight", None, vapor::schema::quicksight::queries::QuickSightQuery);
    page!(pages, "comprehend", "Comprehend", "comprehend", None, vapor::schema::comprehend::queries::ComprehendQuery);
    page!(pages, "rekognition", "Rekognition", "rekognition", None, vapor::schema::rekognition::queries::RekognitionQuery);
    page!(pages, "transcribe", "Transcribe", "transcribe", None, vapor::schema::transcribe::queries::TranscribeQuery);
    page!(pages, "translate", "Translate", "translate", None, vapor::schema::translate::queries::TranslateQuery);
    page!(pages, "polly", "Polly", "polly", None, vapor::schema::polly::queries::PollyQuery);
    page!(pages, "codeartifact", "CodeArtifact", "codeartifact", None, vapor::schema::codeartifact::queries::CodeArtifactQuery);
    page!(pages, "codecommit", "CodeCommit", "codecommit", None, vapor::schema::codecommit::queries::CodeCommitQuery);
    page!(pages, "iot", "IoT Core", "iot", None, vapor::schema::iot::queries::IotQuery);
    page!(pages, "license_manager", "License Manager", "licensemanager", None, vapor::schema::license_manager::queries::LicenseManagerQuery);
    page!(pages, "budgets", "Budgets", "budgets", None, vapor::schema::budgets::queries::BudgetsQuery);
    page!(pages, "connect", "Connect", "connect", None, vapor::schema::connect::queries::ConnectQuery);
    page!(pages, "pinpoint", "Pinpoint", "pinpoint", None, vapor::schema::pinpoint::queries::PinpointQuery);

    assert_eq!(pages.len(), 102, "expected 102 service pages (one per registry.rs QueryRoot entry)");

    // Drift check: every field in the fully-merged schema's `type Query` must
    // be accounted for by exactly one of the per-service pages above, and
    // vice versa. If registry.rs gains/loses a service without this table
    // being updated to match, fail loudly instead of silently publishing
    // incomplete docs.
    let full_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .build();
    let full_schema = vapor::schema::root::build_schema(&full_config);
    let full_query_type_name = vapor::schema::aws::registry::QueryRoot::type_name();
    let mut full_fields = query_field_names(&full_schema.sdl(), &full_query_type_name);
    // `BaseQuery::placeholder` is a liveness field always present on the
    // root, not tied to any AWS service — it has no page of its own.
    full_fields.remove("placeholder");

    let mut documented_fields = BTreeSet::new();
    for page in &pages {
        documented_fields.extend(query_field_names(&page.sdl, &page.query_type_name));
    }

    let missing: Vec<_> = full_fields.difference(&documented_fields).collect();
    let extra: Vec<_> = documented_fields.difference(&full_fields).collect();
    if !missing.is_empty() || !extra.is_empty() {
        eprintln!("gen-docs: per-service page table is out of sync with registry.rs");
        if !missing.is_empty() {
            eprintln!("  fields in the full schema but missing from a service page: {missing:?}");
        }
        if !extra.is_empty() {
            eprintln!("  fields documented on a service page but absent from the full schema: {extra:?}");
        }
        std::process::exit(1);
    }

    let docs_src = Path::new("docs/src");
    let services_dir = docs_src.join("services");
    fs::create_dir_all(&services_dir).expect("create docs/src/services");

    for page in &pages {
        let path = services_dir.join(format!("{}.md", page.slug));
        fs::write(&path, render_page(page)).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
    }

    fs::write(docs_src.join("SUMMARY.md"), render_summary(&pages)).expect("write docs/src/SUMMARY.md");

    println!("gen-docs: wrote {} service pages to docs/src/services/", pages.len());
}
