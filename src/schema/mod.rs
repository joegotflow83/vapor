//! Schema modules for vapor project

pub mod aws;
pub mod common;
pub mod pagination;
pub mod time;
#[cfg(test)]
pub(crate) mod test_util;
#[cfg(feature = "ec2")]
pub mod ec2;
#[cfg(feature = "s3")]
pub mod s3;
#[cfg(feature = "lambda")]
pub mod lambda;
#[cfg(feature = "ssm")]
pub mod ssm;
#[cfg(feature = "ec2")]
pub mod vpc;
#[cfg(feature = "ecs")]
pub mod ecs;
#[cfg(feature = "eks")]
pub mod eks;
#[cfg(feature = "ecr")]
pub mod ecr;
#[cfg(feature = "batch")]
pub mod batch;
#[cfg(feature = "elbv2")]
pub mod elbv2;
#[cfg(feature = "autoscaling")]
pub mod asg;
#[cfg(feature = "dynamodb")]
pub mod dynamodb;
#[cfg(feature = "rds")]
pub mod rds;
#[cfg(feature = "efs")]
pub mod efs;
#[cfg(feature = "elasticache")]
pub mod elasticache;
#[cfg(feature = "redshift")]
pub mod redshift;
#[cfg(feature = "redshiftserverless")]
pub mod redshift_serverless;
#[cfg(feature = "memorydb")]
pub mod memorydb;
#[cfg(feature = "neptune")]
pub mod neptune;
#[cfg(feature = "docdb")]
pub mod documentdb;
#[cfg(feature = "athena")]
pub mod athena;
#[cfg(feature = "glue")]
pub mod glue;
#[cfg(feature = "emr")]
pub mod emr;
#[cfg(feature = "kinesis")]
pub mod kinesis;
#[cfg(feature = "firehose")]
pub mod firehose;
#[cfg(feature = "kafka")]
pub mod msk;
#[cfg(feature = "route53")]
pub mod route53;
#[cfg(feature = "cloudfront")]
pub mod cloudfront;
#[cfg(feature = "apigateway")]
pub mod apigateway;
#[cfg(feature = "apigatewayv2")]
pub mod apigatewayv2;
#[cfg(feature = "globalaccelerator")]
pub mod global_accelerator;
#[cfg(feature = "directconnect")]
pub mod direct_connect;
#[cfg(feature = "networkfirewall")]
pub mod network_firewall;
#[cfg(feature = "iam")]
pub mod iam;
#[cfg(feature = "kms")]
pub mod kms;
#[cfg(feature = "secretsmanager")]
pub mod secrets_manager;
#[cfg(feature = "acm")]
pub mod acm;
#[cfg(feature = "cognitoidentityprovider")]
pub mod cognito;
#[cfg(feature = "guardduty")]
pub mod guardduty;
#[cfg(feature = "inspector2")]
pub mod inspector;
#[cfg(feature = "securityhub")]
pub mod security_hub;
#[cfg(feature = "macie2")]
pub mod macie;
#[cfg(feature = "shield")]
pub mod shield;
#[cfg(feature = "wafv2")]
pub mod wafv2;
#[cfg(feature = "sts")]
pub mod sts;
#[cfg(feature = "cloudwatch")]
pub mod cloudwatch;
#[cfg(feature = "cloudtrail")]
pub mod cloudtrail;
#[cfg(feature = "config")]
pub mod config_svc;
#[cfg(feature = "cloudformation")]
pub mod cloudformation;
#[cfg(feature = "codepipeline")]
pub mod codepipeline;
#[cfg(feature = "codebuild")]
pub mod codebuild;
#[cfg(feature = "codedeploy")]
pub mod codedeploy;
#[cfg(feature = "sfn")]
pub mod step_functions;
#[cfg(feature = "eventbridge")]
pub mod eventbridge;
#[cfg(feature = "sns")]
pub mod sns;
#[cfg(feature = "sqs")]
pub mod sqs;
#[cfg(feature = "servicequotas")]
pub mod service_quotas;
#[cfg(feature = "health")]
pub mod health;
#[cfg(feature = "organizations")]
pub mod organizations;
#[cfg(feature = "appconfig")]
pub mod appconfig;
#[cfg(feature = "appsync")]
pub mod appsync;
#[cfg(feature = "costexplorer")]
pub mod cost_explorer;
#[cfg(feature = "sagemaker")]
pub mod sagemaker;
#[cfg(feature = "transfer")]
pub mod transfer;
#[cfg(feature = "opensearch")]
pub mod opensearch;
#[cfg(feature = "backup")]
pub mod backup;
#[cfg(feature = "ssoadmin")]
pub mod sso_admin;
#[cfg(feature = "acmpca")]
pub mod acm_pca;
#[cfg(feature = "ram")]
pub mod ram;
#[cfg(feature = "controltower")]
pub mod control_tower;
#[cfg(feature = "fms")]
pub mod fms;
#[cfg(feature = "auditmanager")]
pub mod audit_manager;
#[cfg(feature = "detective")]
pub mod detective;
#[cfg(feature = "sesv2")]
pub mod ses;
#[cfg(feature = "elasticbeanstalk")]
pub mod elastic_beanstalk;
#[cfg(feature = "apprunner")]
pub mod app_runner;
#[cfg(feature = "fsx")]
pub mod fsx;
#[cfg(feature = "mq")]
pub mod mq;
#[cfg(feature = "dms")]
pub mod dms;
#[cfg(feature = "workspaces")]
pub mod workspaces;
#[cfg(feature = "storagegateway")]
pub mod storage_gateway;
#[cfg(feature = "datasync")]
pub mod datasync;
#[cfg(feature = "lightsail")]
pub mod lightsail;
#[cfg(feature = "qldb")]
pub mod qldb;
#[cfg(feature = "keyspaces")]
pub mod keyspaces;
#[cfg(feature = "bedrock")]
pub mod bedrock;
#[cfg(feature = "xray")]
pub mod xray;
#[cfg(feature = "timestream")]
pub mod timestream;
#[cfg(feature = "lakeformation")]
pub mod lake_formation;
#[cfg(feature = "quicksight")]
pub mod quicksight;
#[cfg(feature = "comprehend")]
pub mod comprehend;
#[cfg(feature = "rekognition")]
pub mod rekognition;
#[cfg(feature = "transcribe")]
pub mod transcribe;
#[cfg(feature = "translate")]
pub mod translate;
#[cfg(feature = "polly")]
pub mod polly;
#[cfg(feature = "codeartifact")]
pub mod codeartifact;
#[cfg(feature = "codecommit")]
pub mod codecommit;
#[cfg(feature = "iot")]
pub mod iot;
#[cfg(feature = "licensemanager")]
pub mod license_manager;
#[cfg(feature = "budgets")]
pub mod budgets;
#[cfg(feature = "connect")]
pub mod connect;
#[cfg(feature = "pinpoint")]
pub mod pinpoint;
pub mod root;

#[cfg(test)]
mod docs_feature_sync {
    //! Guards against a service being added here but forgotten in Cargo.toml's
    //! `docs` feature. gen-docs has `required-features = ["docs"]`, so a missing
    //! entry doesn't fail the build — it silently drops the service's page from
    //! the published docs. This test turns that silent gap into a loud failure.
    //!
    //! Reads both files as text (via `include_str!`, so it's independent of the
    //! working directory and of which features are enabled) rather than relying
    //! on `cfg!`, which can only see features that happen to be on.

    /// Every `#[cfg(feature = "...")]` gate in this file. These gate the service
    /// schema modules, which are exactly what gen-docs renders.
    fn schema_features() -> Vec<String> {
        let src = include_str!("mod.rs");
        let mut out = Vec::new();
        for line in src.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("#[cfg(feature = \"") {
                if let Some(name) = rest.split('"').next() {
                    out.push(name.to_string());
                }
            }
        }
        out
    }

    /// The feature names listed in Cargo.toml's `docs = [ ... ]` array.
    fn docs_features() -> Vec<String> {
        let manifest = include_str!("../../Cargo.toml");
        let start = manifest
            .find("\ndocs = [")
            .expect("Cargo.toml should define a `docs` feature");
        let body_start = manifest[start..].find('[').unwrap() + start + 1;
        let body_end = manifest[body_start..].find(']').expect("`docs` array should close") + body_start;
        let body = &manifest[body_start..body_end];

        let mut out = Vec::new();
        let mut rest = body;
        while let Some(open) = rest.find('"') {
            rest = &rest[open + 1..];
            let close = rest.find('"').expect("unterminated string in `docs` array");
            out.push(rest[..close].to_string());
            rest = &rest[close + 1..];
        }
        out
    }

    #[test]
    fn every_schema_feature_is_in_docs() {
        let docs = docs_features();
        // Sanity check that our parsing actually found the array.
        assert!(
            docs.len() > 50,
            "parsed only {} entries from the `docs` feature — parsing likely broke",
            docs.len()
        );

        let missing: Vec<String> = schema_features()
            .into_iter()
            .filter(|f| !docs.contains(f))
            .collect();

        assert!(
            missing.is_empty(),
            "these features gate schema modules in src/schema/mod.rs but are missing from \
             Cargo.toml's `docs` feature, so gen-docs would silently omit their pages: {missing:?}. \
             Add them to the `docs` array."
        );
    }
}
