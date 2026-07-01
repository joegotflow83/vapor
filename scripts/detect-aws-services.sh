#!/usr/bin/env bash
set -euo pipefail

# Detect which AWS services are actually in use in the current account/profile
# and print a `cargo build` command enabling only the matching vapor feature
# flags. Useful when the prebuilt GitHub release (curated "release" feature
# group, ~26 common services — see Cargo.toml) doesn't cover something you
# rely on, so you can build a binary tailored to your account instead.
#
# Detection is via the Resource Groups Tagging API, which covers most
# taggable resources across services in one call per region. This is
# best-effort: untagged resources, and a handful of global/free services
# (bare IAM policies, STS, etc.) may not surface here. Cross-check against
# FEATURE_FLAGS.md if a service you use doesn't show up.
#
# Usage:
#   ./scripts/detect-aws-services.sh                 # scan the current default region
#   ./scripts/detect-aws-services.sh --all-regions    # scan every enabled region (slower)
#   AWS_PROFILE=my-profile ./scripts/detect-aws-services.sh

command -v aws >/dev/null 2>&1 || { echo "error: aws CLI is required" >&2; exit 1; }

all_regions=false
if [[ "${1:-}" == "--all-regions" ]]; then
  all_regions=true
fi

if $all_regions; then
  regions=$(aws ec2 describe-regions --query 'Regions[].RegionName' --output text)
else
  regions=$(aws configure get region 2>/dev/null || true)
  regions=${regions:-us-east-1}
fi

echo "Scanning region(s): $regions" >&2

arns_file=$(mktemp)
features_file=$(mktemp)
unmapped_file=$(mktemp)
trap 'rm -f "$arns_file" "$features_file" "$unmapped_file"' EXIT

for region in $regions; do
  echo "  - $region" >&2
  aws resourcegroupstaggingapi get-resources \
    --region "$region" \
    --query 'ResourceTagMappingList[].ResourceARN' \
    --output text 2>/dev/null | tr '\t' '\n' >> "$arns_file" || true
done

# Map an ARN service namespace (arn:partition:SERVICE:region:account:resource)
# to a vapor Cargo feature flag. Echoes nothing for unrecognized namespaces.
map_service() {
  case "$1" in
    ec2) echo ec2 ;;
    s3) echo s3 ;;
    lambda) echo lambda ;;
    iam) echo iam ;;
    rds) echo rds ;;
    dynamodb) echo dynamodb ;;
    cloudwatch) echo cloudwatch ;;
    logs) echo cloudwatchlogs ;;
    sqs) echo sqs ;;
    sns) echo sns ;;
    kms) echo kms ;;
    secretsmanager) echo secretsmanager ;;
    ecs) echo ecs ;;
    ecr) echo ecr ;;
    eks) echo eks ;;
    elasticloadbalancing) echo elbv2 ;;
    autoscaling) echo autoscaling ;;
    route53) echo route53 ;;
    cloudfront) echo cloudfront ;;
    cloudformation) echo cloudformation ;;
    sts) echo sts ;;
    apigateway) echo apigateway ;;
    events) echo eventbridge ;;
    ssm) echo ssm ;;
    kinesis) echo kinesis ;;
    elasticache) echo elasticache ;;
    states) echo sfn ;;
    es) echo opensearch ;;
    redshift) echo redshift ;;
    redshift-serverless) echo redshiftserverless ;;
    glue) echo glue ;;
    athena) echo athena ;;
    firehose) echo firehose ;;
    kafka) echo kafka ;;
    codepipeline) echo codepipeline ;;
    codebuild) echo codebuild ;;
    codedeploy) echo codedeploy ;;
    codecommit) echo codecommit ;;
    codeartifact) echo codeartifact ;;
    guardduty) echo guardduty ;;
    config) echo config ;;
    securityhub) echo securityhub ;;
    inspector2) echo inspector2 ;;
    wafv2) echo wafv2 ;;
    elasticfilesystem) echo efs ;;
    organizations) echo organizations ;;
    cognito-idp) echo cognitoidentityprovider ;;
    appsync) echo appsync ;;
    backup) echo backup ;;
    servicequotas) echo servicequotas ;;
    health) echo health ;;
    elasticmapreduce) echo emr ;;
    sagemaker) echo sagemaker ;;
    batch) echo batch ;;
    transfer) echo transfer ;;
    appconfig) echo appconfig ;;
    globalaccelerator) echo globalaccelerator ;;
    directconnect) echo directconnect ;;
    macie2) echo macie2 ;;
    network-firewall) echo networkfirewall ;;
    shield) echo shield ;;
    sso) echo ssoadmin ;;
    acm-pca) echo acmpca ;;
    ram) echo ram ;;
    controltower) echo controltower ;;
    fms) echo fms ;;
    auditmanager) echo auditmanager ;;
    detective) echo detective ;;
    ses) echo sesv2 ;;
    elasticbeanstalk) echo elasticbeanstalk ;;
    apprunner) echo apprunner ;;
    fsx) echo fsx ;;
    mq) echo mq ;;
    dms) echo dms ;;
    workspaces) echo workspaces ;;
    storagegateway) echo storagegateway ;;
    datasync) echo datasync ;;
    lightsail) echo lightsail ;;
    qldb) echo qldb ;;
    cassandra) echo keyspaces ;;
    bedrock) echo bedrock ;;
    xray) echo xray ;;
    timestream) echo timestream ;;
    lakeformation) echo lakeformation ;;
    quicksight) echo quicksight ;;
    comprehend) echo comprehend ;;
    rekognition) echo rekognition ;;
    transcribe) echo transcribe ;;
    translate) echo translate ;;
    polly) echo polly ;;
    iot) echo iot ;;
    license-manager) echo licensemanager ;;
    budgets) echo budgets ;;
    connect) echo connect ;;
    mobiletargeting) echo pinpoint ;;
    acm) echo acm ;;
    neptune-db) echo neptune ;;
    docdb) echo docdb ;;
    *) echo "" ;;
  esac
}

services=$(cut -d: -f3 "$arns_file" | sort -u)

while IFS= read -r svc; do
  [[ -z "$svc" ]] && continue
  feature=$(map_service "$svc")
  if [[ -n "$feature" ]]; then
    echo "$feature" >> "$features_file"
  else
    echo "$svc" >> "$unmapped_file"
  fi
done <<< "$services"

feature_list=$(sort -u "$features_file" | paste -sd' ' -)

if [[ -z "$feature_list" ]]; then
  echo "No taggable resources matched a known vapor feature. Nothing to suggest." >&2
  exit 0
fi

echo
echo "Detected services -> feature flags: $feature_list"
if [[ -s "$unmapped_file" ]]; then
  echo "Unrecognized ARN service namespaces (cross-check FEATURE_FLAGS.md manually): $(paste -sd' ' - < "$unmapped_file")" >&2
fi
echo
echo "Suggested build command:"
echo "  cargo build --release --no-default-features --features \"$feature_list\""
