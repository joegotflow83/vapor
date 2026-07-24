//! Schema modules for vapor project

#[cfg(feature = "acm")]
pub mod acm;
#[cfg(feature = "acmpca")]
pub mod acm_pca;
#[cfg(feature = "apigateway")]
pub mod apigateway;
#[cfg(feature = "apigatewayv2")]
pub mod apigatewayv2;
#[cfg(feature = "apprunner")]
pub mod app_runner;
#[cfg(feature = "appconfig")]
pub mod appconfig;
#[cfg(feature = "appsync")]
pub mod appsync;
#[cfg(feature = "autoscaling")]
pub mod asg;
#[cfg(feature = "athena")]
pub mod athena;
#[cfg(feature = "auditmanager")]
pub mod audit_manager;
pub mod aws;
#[cfg(feature = "backup")]
pub mod backup;
#[cfg(feature = "batch")]
pub mod batch;
#[cfg(feature = "bedrock")]
pub mod bedrock;
#[cfg(feature = "budgets")]
pub mod budgets;
#[cfg(feature = "cloudformation")]
pub mod cloudformation;
#[cfg(feature = "cloudfront")]
pub mod cloudfront;
#[cfg(feature = "cloudtrail")]
pub mod cloudtrail;
#[cfg(feature = "cloudwatch")]
pub mod cloudwatch;
#[cfg(feature = "codeartifact")]
pub mod codeartifact;
#[cfg(feature = "codebuild")]
pub mod codebuild;
#[cfg(feature = "codecommit")]
pub mod codecommit;
#[cfg(feature = "codedeploy")]
pub mod codedeploy;
#[cfg(feature = "codepipeline")]
pub mod codepipeline;
#[cfg(feature = "cognitoidentityprovider")]
pub mod cognito;
pub mod common;
#[cfg(feature = "comprehend")]
pub mod comprehend;
#[cfg(feature = "config")]
pub mod config_svc;
#[cfg(feature = "connect")]
pub mod connect;
#[cfg(feature = "controltower")]
pub mod control_tower;
#[cfg(feature = "costexplorer")]
pub mod cost_explorer;
#[cfg(feature = "datasync")]
pub mod datasync;
#[cfg(feature = "detective")]
pub mod detective;
#[cfg(feature = "directconnect")]
pub mod direct_connect;
#[cfg(feature = "dms")]
pub mod dms;
#[cfg(feature = "docdb")]
pub mod documentdb;
#[cfg(feature = "dynamodb")]
pub mod dynamodb;
#[cfg(feature = "ec2")]
pub mod ec2;
#[cfg(feature = "ecr")]
pub mod ecr;
#[cfg(feature = "ecs")]
pub mod ecs;
#[cfg(feature = "efs")]
pub mod efs;
#[cfg(feature = "eks")]
pub mod eks;
#[cfg(feature = "elasticbeanstalk")]
pub mod elastic_beanstalk;
#[cfg(feature = "elasticache")]
pub mod elasticache;
#[cfg(feature = "elbv2")]
pub mod elbv2;
#[cfg(feature = "emr")]
pub mod emr;
#[cfg(feature = "eventbridge")]
pub mod eventbridge;
#[cfg(feature = "firehose")]
pub mod firehose;
#[cfg(feature = "fms")]
pub mod fms;
#[cfg(feature = "fsx")]
pub mod fsx;
#[cfg(feature = "globalaccelerator")]
pub mod global_accelerator;
#[cfg(feature = "glue")]
pub mod glue;
#[cfg(feature = "guardduty")]
pub mod guardduty;
#[cfg(feature = "health")]
pub mod health;
#[cfg(feature = "iam")]
pub mod iam;
#[cfg(feature = "inspector2")]
pub mod inspector;
#[cfg(feature = "iot")]
pub mod iot;
#[cfg(feature = "keyspaces")]
pub mod keyspaces;
#[cfg(feature = "kinesis")]
pub mod kinesis;
#[cfg(feature = "kms")]
pub mod kms;
#[cfg(feature = "lakeformation")]
pub mod lake_formation;
#[cfg(feature = "lambda")]
pub mod lambda;
#[cfg(feature = "licensemanager")]
pub mod license_manager;
#[cfg(feature = "lightsail")]
pub mod lightsail;
#[cfg(feature = "macie2")]
pub mod macie;
#[cfg(feature = "memorydb")]
pub mod memorydb;
#[cfg(feature = "mq")]
pub mod mq;
#[cfg(feature = "kafka")]
pub mod msk;
#[cfg(feature = "neptune")]
pub mod neptune;
#[cfg(feature = "networkfirewall")]
pub mod network_firewall;
#[cfg(feature = "opensearch")]
pub mod opensearch;
#[cfg(feature = "organizations")]
pub mod organizations;
pub mod pagination;
#[cfg(feature = "pinpoint")]
pub mod pinpoint;
#[cfg(feature = "polly")]
pub mod polly;
#[cfg(feature = "qldb")]
pub mod qldb;
#[cfg(feature = "quicksight")]
pub mod quicksight;
#[cfg(feature = "ram")]
pub mod ram;
#[cfg(feature = "rds")]
pub mod rds;
#[cfg(feature = "redshift")]
pub mod redshift;
#[cfg(feature = "redshiftserverless")]
pub mod redshift_serverless;
#[cfg(feature = "rekognition")]
pub mod rekognition;
pub mod root;
#[cfg(feature = "route53")]
pub mod route53;
#[cfg(feature = "s3")]
pub mod s3;
#[cfg(feature = "sagemaker")]
pub mod sagemaker;
#[cfg(feature = "secretsmanager")]
pub mod secrets_manager;
#[cfg(feature = "securityhub")]
pub mod security_hub;
#[cfg(feature = "servicequotas")]
pub mod service_quotas;
#[cfg(feature = "sesv2")]
pub mod ses;
#[cfg(feature = "shield")]
pub mod shield;
#[cfg(feature = "sns")]
pub mod sns;
#[cfg(feature = "sqs")]
pub mod sqs;
#[cfg(feature = "ssm")]
pub mod ssm;
#[cfg(feature = "ssoadmin")]
pub mod sso_admin;
#[cfg(feature = "sfn")]
pub mod step_functions;
#[cfg(feature = "storagegateway")]
pub mod storage_gateway;
#[cfg(feature = "sts")]
pub mod sts;
#[cfg(test)]
pub(crate) mod test_util;
pub mod time;
#[cfg(feature = "timestream")]
pub mod timestream;
#[cfg(feature = "transcribe")]
pub mod transcribe;
#[cfg(feature = "transfer")]
pub mod transfer;
#[cfg(feature = "translate")]
pub mod translate;
#[cfg(feature = "ec2")]
pub mod vpc;
#[cfg(feature = "wafv2")]
pub mod wafv2;
#[cfg(feature = "workspaces")]
pub mod workspaces;
#[cfg(feature = "xray")]
pub mod xray;

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
        let body_end = manifest[body_start..]
            .find(']')
            .expect("`docs` array should close")
            + body_start;
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
