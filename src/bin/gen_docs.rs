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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use async_graphql::parser::parse_schema;
use async_graphql::parser::types::{
    BaseType, FieldDefinition, ObjectType as SdlObjectType, Type as SdlType, TypeKind,
    TypeSystemDefinition,
};
use async_graphql::{
    EmptyMutation, EmptySubscription, ObjectType, OutputType, Schema, SubscriptionType,
};

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

fn sdl_for<Q>() -> (String, String, Schema<Q, EmptyMutation, EmptySubscription>)
where
    Q: ObjectType + Default + Send + Sync + 'static,
{
    let schema = Schema::build(Q::default(), EmptyMutation, EmptySubscription).finish();
    let sdl = clean_sdl(&schema.sdl());
    (Q::type_name().into_owned(), sdl, schema)
}

fn sdl_for_with_mutation<Q, M>() -> (String, String, Schema<Q, M, EmptySubscription>)
where
    Q: ObjectType + Default + Send + Sync + 'static,
    M: ObjectType + Default + Send + Sync + 'static,
{
    let schema = Schema::build(Q::default(), M::default(), EmptySubscription).finish();
    let sdl = clean_sdl(&schema.sdl());
    (Q::type_name().into_owned(), sdl, schema)
}

/// Executes every synthesized/curated example against the service's own
/// (credential-free) schema and records real failures. A resolver that runs
/// far enough to call `ctx.data::<...Client>()?` and die there produces an
/// error whose `path` is non-empty (it failed *while resolving* a field) —
/// that's expected and not a failure. An empty `path` means the query never
/// started resolving (parse/validation error, e.g. an unknown field), which
/// is a real authoring mistake in the example.
async fn validate_examples<Q, M, S>(
    schema: &Schema<Q, M, S>,
    examples: &[String],
    label: &str,
    failures: &mut Vec<String>,
) where
    Q: ObjectType + Send + Sync + 'static,
    M: ObjectType + Send + Sync + 'static,
    S: SubscriptionType + Send + Sync + 'static,
{
    for example in examples {
        let resp = schema.execute(example.as_str()).await;
        for err in resp.errors.iter().filter(|e| e.path.is_empty()) {
            failures.push(format!("{label}: `{example}` -> {}", err.message));
        }
    }
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
    let after = if text[block_end..].starts_with('\n') {
        block_end + 1
    } else {
        block_end
    };
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
        let name: String = trimmed
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            fields.insert(name);
        }
    }
    fields
}

/// Indexes a service's SDL by type name so synthesis can resolve a query
/// field's return type to its own field list. The typed registry would be
/// nicer, but `Schema::registry()` is pub(crate) — parsing our own emitted
/// SDL back is the supported route and adds no dependency. The SDL fed in is
/// already `clean_sdl`-processed (no `schema { }` block, no
/// `EmptyMutation`/`EmptySubscription` stubs), so every definition here is a
/// real service type.
fn index_types(sdl: &str) -> BTreeMap<String, TypeKind> {
    let Ok(doc) = parse_schema(sdl) else {
        return BTreeMap::new();
    };
    doc.definitions
        .into_iter()
        .filter_map(|def| match def {
            TypeSystemDefinition::Type(positioned) => {
                let type_def = positioned.node;
                Some((type_def.name.node.to_string(), type_def.kind))
            }
            _ => None,
        })
        .collect()
}

/// Strips `List`/nullable wrappers down to the named type at the core of a
/// field or argument type (e.g. `[String!]!` -> `String`).
fn base_type_name(ty: &SdlType) -> String {
    match &ty.base {
        BaseType::Named(name) => name.to_string(),
        BaseType::List(inner) => base_type_name(inner),
    }
}

/// Ranks a field name by how identifier-like it looks, for leaf-selection
/// preference; lower is preferred. Ties keep declaration order (stable sort).
fn identifier_rank(name: &str) -> u8 {
    if matches!(name, "name" | "id" | "arn" | "status" | "state") {
        0
    } else if name.ends_with("Name") || name.ends_with("Id") {
        1
    } else {
        2
    }
}

/// Selects up to 4 scalar/enum leaf fields from an object type, preferring
/// identifier-ish names, falling back to declaration order. Fields whose
/// resolved type is itself an object (i.e. nested, not a leaf) are skipped
/// entirely.
fn leaf_field_names(obj: &SdlObjectType, index: &BTreeMap<String, TypeKind>) -> Vec<String> {
    let mut candidates: Vec<(usize, u8, &FieldDefinition)> = obj
        .fields
        .iter()
        .map(|f| &f.node)
        .filter(|field| {
            let type_name = base_type_name(&field.ty.node);
            !matches!(index.get(&type_name), Some(TypeKind::Object(_)))
        })
        .enumerate()
        .map(|(i, field)| (i, identifier_rank(field.name.node.as_str()), field))
        .collect();
    candidates.sort_by_key(|(i, rank, _)| (*rank, *i));
    candidates
        .into_iter()
        .take(4)
        .map(|(_, _, field)| field.name.node.to_string())
        .collect()
}

/// Produces a placeholder literal for a required argument, based on its
/// resolved type and name. Enum arguments use their first declared value
/// (unquoted); everything else falls back to a type-appropriate guess.
fn placeholder_value(arg_name: &str, type_name: &str, index: &BTreeMap<String, TypeKind>) -> String {
    if let Some(TypeKind::Enum(enum_type)) = index.get(type_name) {
        if let Some(first) = enum_type.values.first() {
            return first.node.value.node.to_string();
        }
    }
    match type_name {
        "Int" | "Float" => "1".to_string(),
        "Boolean" => "true".to_string(),
        _ => {
            let lower = arg_name.to_lowercase();
            if lower.ends_with("arn") {
                format!("\"arn:aws:service:us-east-1:123456789012:resource/{arg_name}\"")
            } else if lower.ends_with("id") {
                "\"example-id\"".to_string()
            } else if lower.ends_with("name") {
                "\"my-db\"".to_string()
            } else {
                "\"example\"".to_string()
            }
        }
    }
}

/// Renders a field's selection set: `Page<T>` return types unwrap to
/// `{ items { <leaves> } nextToken }`; everything else selects its leaf
/// fields directly. Degrades gracefully (empty braces) if the payload type
/// has no scalar/enum leaves to select, rather than panicking.
fn selection_set(type_name: &str, index: &BTreeMap<String, TypeKind>) -> Option<String> {
    let Some(TypeKind::Object(obj)) = index.get(type_name) else {
        // Scalar/enum leaf return type: no selection set needed.
        return None;
    };

    let is_page = type_name.ends_with("Page")
        && obj.fields.iter().any(|f| f.node.name.node == "items")
        && obj.fields.iter().any(|f| f.node.name.node == "nextToken");

    if is_page {
        let items_field = obj
            .fields
            .iter()
            .find(|f| f.node.name.node == "items")
            .expect("checked above");
        let item_type = base_type_name(&items_field.node.ty.node);
        let leaves = match index.get(&item_type) {
            Some(TypeKind::Object(item_obj)) => leaf_field_names(item_obj, index),
            _ => Vec::new(),
        };
        return Some(format!("{{ items {{ {} }} nextToken }}", leaves.join(" ")));
    }

    let leaves = leaf_field_names(obj, index);
    Some(format!("{{ {} }}", leaves.join(" ")))
}

/// Synthesizes example queries for the first 3 fields of a service's query
/// type: page-unwraps paginated return types, selects a handful of leaf
/// fields, and fills in placeholders for required arguments (plus
/// `limit: 5` when the field accepts one).
fn synthesize_examples(sdl: &str, query_type_name: &str) -> Vec<String> {
    let index = index_types(sdl);
    let Some(TypeKind::Object(query_obj)) = index.get(query_type_name) else {
        return Vec::new();
    };

    query_obj
        .fields
        .iter()
        .take(3)
        .map(|f| synthesize_field_query(&f.node, &index))
        .collect()
}

fn synthesize_field_query(field: &FieldDefinition, index: &BTreeMap<String, TypeKind>) -> String {
    let field_name = field.name.node.as_str();

    let mut args = Vec::new();
    for arg in &field.arguments {
        let arg_node = &arg.node;
        let arg_name = arg_node.name.node.as_str();
        let type_name = base_type_name(&arg_node.ty.node);
        let is_required = !arg_node.ty.node.nullable && arg_node.default_value.is_none();
        if is_required {
            args.push(format!(
                "{arg_name}: {}",
                placeholder_value(arg_name, &type_name, index)
            ));
        } else if arg_name == "limit" && type_name == "Int" {
            args.push("limit: 5".to_string());
        }
    }

    let return_type_name = base_type_name(&field.ty.node);
    let selection = selection_set(&return_type_name, index);

    let args_str = if args.is_empty() {
        String::new()
    } else {
        format!("({})", args.join(", "))
    };
    match selection {
        Some(sel) => format!("{{ {field_name}{args_str} {sel} }}"),
        None => format!("{{ {field_name}{args_str} }}"),
    }
}

/// One example query for a service page: an optional bolded caption (from a
/// curated file's `#` comment line) plus the query itself. Synthesized
/// examples always have `caption: None`.
#[allow(dead_code)]
struct Example {
    caption: Option<String>,
    query: String,
}

/// Splits a curated `docs/examples/<slug>.graphql` file's contents into
/// examples. A `#`-prefixed line starts a new example and becomes its
/// caption; every following non-blank, non-comment line is appended to that
/// example's query (multi-line queries are joined with spaces). A blank line
/// ends the current example. A file whose first line is a query with no
/// preceding comment yields an example with `caption: None`.
#[allow(dead_code)]
fn parse_curated_examples(contents: &str) -> Vec<Example> {
    let mut examples = Vec::new();
    let mut caption: Option<String> = None;
    let mut query_lines: Vec<&str> = Vec::new();

    fn flush(caption: &mut Option<String>, query_lines: &mut Vec<&str>, examples: &mut Vec<Example>) {
        if !query_lines.is_empty() {
            examples.push(Example {
                caption: caption.take(),
                query: query_lines.join(" "),
            });
            query_lines.clear();
        }
    }

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut caption, &mut query_lines, &mut examples);
        } else if let Some(rest) = trimmed.strip_prefix('#') {
            flush(&mut caption, &mut query_lines, &mut examples);
            caption = Some(rest.trim().to_string());
        } else {
            query_lines.push(trimmed);
        }
    }
    flush(&mut caption, &mut query_lines, &mut examples);

    examples
}

/// Reads and parses `<dir>/<slug>.graphql`, if present. Split out from
/// `load_curated_examples` so tests can point at a scratch directory instead
/// of the real `docs/examples/`.
#[allow(dead_code)]
fn load_curated_examples_from(dir: &Path, slug: &str) -> Option<Vec<Example>> {
    let contents = fs::read_to_string(dir.join(format!("{slug}.graphql"))).ok()?;
    Some(parse_curated_examples(&contents))
}

/// Reads `docs/examples/<slug>.graphql`, if present. Presence of this file
/// fully replaces synthesis for that service — see Decision 4 in
/// `specs/example-docs.md`.
#[allow(dead_code)]
fn load_curated_examples(slug: &str) -> Option<Vec<Example>> {
    load_curated_examples_from(Path::new("docs/examples"), slug)
}

/// Resolves the examples to render for a service: a curated file, if one was
/// loaded, replaces synthesis entirely; otherwise the synthesized queries are
/// used as-is (uncaptioned).
#[allow(dead_code)]
fn resolve_examples(curated: Option<Vec<Example>>, synthesized: Vec<String>) -> Vec<Example> {
    curated.unwrap_or_else(|| {
        synthesized
            .into_iter()
            .map(|query| Example {
                caption: None,
                query,
            })
            .collect()
    })
}

fn render_page(page: &ServicePage) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", page.title));
    out.push_str(&format!(
        "Cargo feature: `{}` (`cargo build --features {}`)\n\n",
        page.feature, page.feature
    ));
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

#[tokio::main]
async fn main() {
    let mut pages: Vec<ServicePage> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    macro_rules! page {
        ($pages:ident, $slug:expr, $title:expr, $feature:expr, $note:expr, $query:ty) => {{
            let (query_type_name, sdl, schema) = sdl_for::<$query>();
            let examples = synthesize_examples(&sdl, &query_type_name);
            validate_examples(&schema, &examples, $slug, &mut failures).await;
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
    let (ec2_query_type_name, ec2_sdl, ec2_schema) = sdl_for_with_mutation::<
        vapor::schema::ec2::queries::Ec2Query,
        vapor::schema::ec2::mutations::Ec2Mutation,
    >();
    let ec2_examples = synthesize_examples(&ec2_sdl, &ec2_query_type_name);
    validate_examples(&ec2_schema, &ec2_examples, "ec2", &mut failures).await;
    pages.push(ServicePage {
        slug: "ec2",
        title: "EC2",
        feature: "ec2",
        note: None,
        query_type_name: ec2_query_type_name,
        sdl: ec2_sdl,
    });

    page!(
        pages,
        "vpc",
        "VPC",
        "ec2",
        None,
        vapor::schema::vpc::queries::VpcQuery
    );
    page!(
        pages,
        "s3",
        "S3",
        "s3",
        None,
        vapor::schema::s3::queries::S3Query
    );
    page!(
        pages,
        "lambda",
        "Lambda",
        "lambda",
        None,
        vapor::schema::lambda::queries::LambdaQuery
    );
    page!(
        pages,
        "ssm",
        "Systems Manager",
        "ssm",
        None,
        vapor::schema::ssm::queries::SsmQuery
    );
    page!(
        pages,
        "ecs",
        "ECS",
        "ecs",
        None,
        vapor::schema::ecs::queries::EcsQuery
    );
    page!(
        pages,
        "eks",
        "EKS",
        "eks",
        None,
        vapor::schema::eks::queries::EksQuery
    );
    page!(
        pages,
        "ecr",
        "ECR",
        "ecr",
        None,
        vapor::schema::ecr::queries::EcrQuery
    );
    page!(
        pages,
        "batch",
        "Batch",
        "batch",
        None,
        vapor::schema::batch::queries::BatchQuery
    );
    page!(
        pages,
        "elbv2",
        "Elastic Load Balancing v2",
        "elbv2",
        None,
        vapor::schema::elbv2::queries::Elbv2Query
    );
    page!(
        pages,
        "asg",
        "Auto Scaling",
        "autoscaling",
        None,
        vapor::schema::asg::queries::AsgQuery
    );
    page!(
        pages,
        "dynamodb",
        "DynamoDB",
        "dynamodb",
        None,
        vapor::schema::dynamodb::queries::DynamodbQuery
    );
    page!(
        pages,
        "rds",
        "RDS",
        "rds",
        None,
        vapor::schema::rds::queries::RdsQuery
    );
    page!(
        pages,
        "efs",
        "EFS",
        "efs",
        None,
        vapor::schema::efs::queries::EfsQuery
    );
    page!(
        pages,
        "elasticache",
        "ElastiCache",
        "elasticache",
        None,
        vapor::schema::elasticache::queries::ElastiCacheQuery
    );
    page!(
        pages,
        "redshift",
        "Redshift",
        "redshift",
        None,
        vapor::schema::redshift::queries::RedshiftQuery
    );
    page!(
        pages,
        "redshift_serverless",
        "Redshift Serverless",
        "redshiftserverless",
        None,
        vapor::schema::redshift_serverless::queries::RedshiftServerlessQuery
    );
    page!(
        pages,
        "memorydb",
        "MemoryDB",
        "memorydb",
        None,
        vapor::schema::memorydb::queries::MemoryDbQuery
    );
    page!(
        pages,
        "neptune",
        "Neptune",
        "neptune",
        None,
        vapor::schema::neptune::queries::NeptuneQuery
    );
    page!(
        pages,
        "documentdb",
        "DocumentDB",
        "docdb",
        None,
        vapor::schema::documentdb::queries::DocumentDbQuery
    );
    page!(
        pages,
        "athena",
        "Athena",
        "athena",
        None,
        vapor::schema::athena::queries::AthenaQuery
    );
    page!(
        pages,
        "glue",
        "Glue",
        "glue",
        None,
        vapor::schema::glue::queries::GlueQuery
    );
    page!(
        pages,
        "emr",
        "EMR",
        "emr",
        None,
        vapor::schema::emr::queries::EmrQuery
    );
    page!(
        pages,
        "kinesis",
        "Kinesis Data Streams",
        "kinesis",
        None,
        vapor::schema::kinesis::queries::KinesisQuery
    );
    page!(
        pages,
        "firehose",
        "Kinesis Data Firehose",
        "firehose",
        None,
        vapor::schema::firehose::queries::FirehoseQuery
    );
    page!(
        pages,
        "msk",
        "MSK (Managed Streaming for Kafka)",
        "kafka",
        Some("Pulls in the `ec2` feature since broker AZ enrichment needs `describe_subnets`."),
        vapor::schema::msk::queries::MskQuery
    );
    page!(
        pages,
        "route53",
        "Route 53",
        "route53",
        None,
        vapor::schema::route53::queries::Route53Query
    );
    page!(
        pages,
        "cloudfront",
        "CloudFront",
        "cloudfront",
        None,
        vapor::schema::cloudfront::queries::CloudFrontQuery
    );
    page!(
        pages,
        "apigateway",
        "API Gateway (REST)",
        "apigateway",
        None,
        vapor::schema::apigateway::queries::ApiGatewayQuery
    );
    page!(
        pages,
        "apigatewayv2",
        "API Gateway v2 (HTTP/WebSocket)",
        "apigatewayv2",
        None,
        vapor::schema::apigatewayv2::queries::ApiGatewayV2Query
    );
    page!(
        pages,
        "global_accelerator",
        "Global Accelerator",
        "globalaccelerator",
        None,
        vapor::schema::global_accelerator::queries::GlobalAcceleratorQuery
    );
    page!(
        pages,
        "direct_connect",
        "Direct Connect",
        "directconnect",
        None,
        vapor::schema::direct_connect::queries::DirectConnectQuery
    );
    page!(
        pages,
        "network_firewall",
        "Network Firewall",
        "networkfirewall",
        None,
        vapor::schema::network_firewall::queries::NetworkFirewallQuery
    );
    page!(
        pages,
        "iam",
        "IAM",
        "iam",
        None,
        vapor::schema::iam::queries::IamQuery
    );
    page!(
        pages,
        "kms",
        "KMS",
        "kms",
        None,
        vapor::schema::kms::queries::KmsQuery
    );
    page!(
        pages,
        "secrets_manager",
        "Secrets Manager",
        "secretsmanager",
        None,
        vapor::schema::secrets_manager::queries::SecretsManagerQuery
    );
    page!(
        pages,
        "acm",
        "Certificate Manager",
        "acm",
        None,
        vapor::schema::acm::queries::AcmQuery
    );
    page!(
        pages,
        "cognito",
        "Cognito Identity Provider",
        "cognitoidentityprovider",
        None,
        vapor::schema::cognito::queries::CognitoQuery
    );
    page!(
        pages,
        "guardduty",
        "GuardDuty",
        "guardduty",
        None,
        vapor::schema::guardduty::queries::GuardDutyQuery
    );
    page!(
        pages,
        "inspector",
        "Inspector v2",
        "inspector2",
        None,
        vapor::schema::inspector::queries::InspectorQuery
    );
    page!(
        pages,
        "security_hub",
        "Security Hub",
        "securityhub",
        None,
        vapor::schema::security_hub::queries::SecurityHubQuery
    );
    page!(
        pages,
        "macie",
        "Macie v2",
        "macie2",
        None,
        vapor::schema::macie::queries::MacieQuery
    );
    page!(
        pages,
        "shield",
        "Shield",
        "shield",
        None,
        vapor::schema::shield::queries::ShieldQuery
    );
    page!(
        pages,
        "wafv2",
        "WAF v2",
        "wafv2",
        None,
        vapor::schema::wafv2::queries::Wafv2Query
    );
    page!(
        pages,
        "sts",
        "STS",
        "sts",
        None,
        vapor::schema::sts::queries::StsQuery
    );
    page!(
        pages,
        "cloudwatch",
        "CloudWatch (includes Logs)",
        "cloudwatch",
        None,
        vapor::schema::cloudwatch::queries::CloudWatchQuery
    );
    page!(
        pages,
        "cloudtrail",
        "CloudTrail",
        "cloudtrail",
        None,
        vapor::schema::cloudtrail::queries::CloudTrailQuery
    );
    page!(
        pages,
        "config_svc",
        "AWS Config",
        "config",
        None,
        vapor::schema::config_svc::queries::AwsConfigQuery
    );
    page!(
        pages,
        "cloudformation",
        "CloudFormation",
        "cloudformation",
        None,
        vapor::schema::cloudformation::queries::CloudFormationQuery
    );
    page!(
        pages,
        "codepipeline",
        "CodePipeline",
        "codepipeline",
        None,
        vapor::schema::codepipeline::queries::CodePipelineQuery
    );
    page!(
        pages,
        "codebuild",
        "CodeBuild",
        "codebuild",
        None,
        vapor::schema::codebuild::queries::CodeBuildQuery
    );
    page!(
        pages,
        "codedeploy",
        "CodeDeploy",
        "codedeploy",
        None,
        vapor::schema::codedeploy::queries::CodeDeployQuery
    );
    page!(
        pages,
        "step_functions",
        "Step Functions",
        "sfn",
        None,
        vapor::schema::step_functions::queries::StepFunctionsQuery
    );
    page!(
        pages,
        "eventbridge",
        "EventBridge",
        "eventbridge",
        None,
        vapor::schema::eventbridge::queries::EventBridgeQuery
    );
    page!(
        pages,
        "sns",
        "SNS",
        "sns",
        None,
        vapor::schema::sns::queries::SnsQuery
    );
    page!(
        pages,
        "sqs",
        "SQS",
        "sqs",
        None,
        vapor::schema::sqs::queries::SqsQuery
    );
    page!(
        pages,
        "service_quotas",
        "Service Quotas",
        "servicequotas",
        None,
        vapor::schema::service_quotas::queries::ServiceQuotasQuery
    );
    page!(
        pages,
        "health",
        "Health",
        "health",
        None,
        vapor::schema::health::queries::HealthQuery
    );
    page!(
        pages,
        "organizations",
        "Organizations",
        "organizations",
        None,
        vapor::schema::organizations::queries::OrganizationsQuery
    );
    page!(
        pages,
        "appconfig",
        "AppConfig",
        "appconfig",
        None,
        vapor::schema::appconfig::queries::AppConfigQuery
    );
    page!(
        pages,
        "appsync",
        "AppSync",
        "appsync",
        None,
        vapor::schema::appsync::queries::AppSyncQuery
    );
    page!(
        pages,
        "cost_explorer",
        "Cost Explorer",
        "costexplorer",
        None,
        vapor::schema::cost_explorer::queries::CostExplorerQuery
    );
    page!(
        pages,
        "sagemaker",
        "SageMaker",
        "sagemaker",
        None,
        vapor::schema::sagemaker::queries::SageMakerQuery
    );
    page!(
        pages,
        "transfer",
        "Transfer Family",
        "transfer",
        None,
        vapor::schema::transfer::queries::TransferQuery
    );
    page!(
        pages,
        "opensearch",
        "OpenSearch",
        "opensearch",
        None,
        vapor::schema::opensearch::queries::OpenSearchQuery
    );
    page!(
        pages,
        "backup",
        "Backup",
        "backup",
        None,
        vapor::schema::backup::queries::BackupQuery
    );
    page!(
        pages,
        "sso_admin",
        "IAM Identity Center (SSO Admin)",
        "ssoadmin",
        None,
        vapor::schema::sso_admin::queries::SsoAdminQuery
    );
    page!(
        pages,
        "acm_pca",
        "ACM Private CA",
        "acmpca",
        None,
        vapor::schema::acm_pca::queries::AcmPcaQuery
    );
    page!(
        pages,
        "ram",
        "Resource Access Manager",
        "ram",
        None,
        vapor::schema::ram::queries::RamQuery
    );
    page!(
        pages,
        "control_tower",
        "Control Tower",
        "controltower",
        None,
        vapor::schema::control_tower::queries::ControlTowerQuery
    );
    page!(
        pages,
        "fms",
        "Firewall Manager",
        "fms",
        None,
        vapor::schema::fms::queries::FmsQuery
    );
    page!(
        pages,
        "audit_manager",
        "Audit Manager",
        "auditmanager",
        None,
        vapor::schema::audit_manager::queries::AuditManagerQuery
    );
    page!(
        pages,
        "detective",
        "Detective",
        "detective",
        None,
        vapor::schema::detective::queries::DetectiveQuery
    );
    page!(
        pages,
        "ses",
        "Simple Email Service v2",
        "sesv2",
        None,
        vapor::schema::ses::queries::SesQuery
    );
    page!(
        pages,
        "elastic_beanstalk",
        "Elastic Beanstalk",
        "elasticbeanstalk",
        None,
        vapor::schema::elastic_beanstalk::queries::ElasticBeanstalkQuery
    );
    page!(
        pages,
        "app_runner",
        "App Runner",
        "apprunner",
        None,
        vapor::schema::app_runner::queries::AppRunnerQuery
    );
    page!(
        pages,
        "fsx",
        "FSx",
        "fsx",
        None,
        vapor::schema::fsx::queries::FsxQuery
    );
    page!(
        pages,
        "mq",
        "Amazon MQ",
        "mq",
        None,
        vapor::schema::mq::queries::MqQuery
    );
    page!(
        pages,
        "dms",
        "Database Migration Service",
        "dms",
        None,
        vapor::schema::dms::queries::DmsQuery
    );
    page!(
        pages,
        "workspaces",
        "WorkSpaces",
        "workspaces",
        None,
        vapor::schema::workspaces::queries::WorkspacesQuery
    );
    page!(
        pages,
        "storage_gateway",
        "Storage Gateway",
        "storagegateway",
        None,
        vapor::schema::storage_gateway::queries::StorageGatewayQuery
    );
    page!(
        pages,
        "datasync",
        "DataSync",
        "datasync",
        None,
        vapor::schema::datasync::queries::DataSyncQuery
    );
    page!(
        pages,
        "lightsail",
        "Lightsail",
        "lightsail",
        None,
        vapor::schema::lightsail::queries::LightsailQuery
    );
    page!(
        pages,
        "qldb",
        "QLDB",
        "qldb",
        None,
        vapor::schema::qldb::queries::QldbQuery
    );
    page!(
        pages,
        "keyspaces",
        "Amazon Keyspaces",
        "keyspaces",
        None,
        vapor::schema::keyspaces::queries::KeyspacesQuery
    );
    page!(
        pages,
        "bedrock",
        "Bedrock",
        "bedrock",
        None,
        vapor::schema::bedrock::queries::BedrockQuery
    );
    page!(
        pages,
        "xray",
        "X-Ray",
        "xray",
        None,
        vapor::schema::xray::queries::XRayQuery
    );
    page!(
        pages,
        "timestream",
        "Timestream",
        "timestream",
        None,
        vapor::schema::timestream::queries::TimestreamQuery
    );
    page!(
        pages,
        "lake_formation",
        "Lake Formation",
        "lakeformation",
        None,
        vapor::schema::lake_formation::queries::LakeFormationQuery
    );
    page!(
        pages,
        "quicksight",
        "QuickSight",
        "quicksight",
        None,
        vapor::schema::quicksight::queries::QuickSightQuery
    );
    page!(
        pages,
        "comprehend",
        "Comprehend",
        "comprehend",
        None,
        vapor::schema::comprehend::queries::ComprehendQuery
    );
    page!(
        pages,
        "rekognition",
        "Rekognition",
        "rekognition",
        None,
        vapor::schema::rekognition::queries::RekognitionQuery
    );
    page!(
        pages,
        "transcribe",
        "Transcribe",
        "transcribe",
        None,
        vapor::schema::transcribe::queries::TranscribeQuery
    );
    page!(
        pages,
        "translate",
        "Translate",
        "translate",
        None,
        vapor::schema::translate::queries::TranslateQuery
    );
    page!(
        pages,
        "polly",
        "Polly",
        "polly",
        None,
        vapor::schema::polly::queries::PollyQuery
    );
    page!(
        pages,
        "codeartifact",
        "CodeArtifact",
        "codeartifact",
        None,
        vapor::schema::codeartifact::queries::CodeArtifactQuery
    );
    page!(
        pages,
        "codecommit",
        "CodeCommit",
        "codecommit",
        None,
        vapor::schema::codecommit::queries::CodeCommitQuery
    );
    page!(
        pages,
        "iot",
        "IoT Core",
        "iot",
        None,
        vapor::schema::iot::queries::IotQuery
    );
    page!(
        pages,
        "license_manager",
        "License Manager",
        "licensemanager",
        None,
        vapor::schema::license_manager::queries::LicenseManagerQuery
    );
    page!(
        pages,
        "budgets",
        "Budgets",
        "budgets",
        None,
        vapor::schema::budgets::queries::BudgetsQuery
    );
    page!(
        pages,
        "connect",
        "Connect",
        "connect",
        None,
        vapor::schema::connect::queries::ConnectQuery
    );
    page!(
        pages,
        "pinpoint",
        "Pinpoint",
        "pinpoint",
        None,
        vapor::schema::pinpoint::queries::PinpointQuery
    );

    assert_eq!(
        pages.len(),
        102,
        "expected 102 service pages (one per registry.rs QueryRoot entry)"
    );

    // Example validation gate: every synthesized example must be a
    // structurally valid query against its own service's schema. A resolver
    // dying on a missing (deliberately unregistered) AWS client is fine and
    // expected; a query that never starts resolving (e.g. references an
    // unknown field) is an authoring bug and fails the build.
    if !failures.is_empty() {
        eprintln!(
            "gen-docs: {} example query failure(s) across service schemas:",
            failures.len()
        );
        for failure in &failures {
            eprintln!("  {failure}");
        }
        std::process::exit(1);
    }

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
            eprintln!(
                "  fields documented on a service page but absent from the full schema: {extra:?}"
            );
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

    fs::write(docs_src.join("SUMMARY.md"), render_summary(&pages))
        .expect("write docs/src/SUMMARY.md");

    println!(
        "gen-docs: wrote {} service pages to docs/src/services/",
        pages.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_block_removes_balanced_braces_and_trailing_newline() {
        let input = "schema {\n  query: Query\n}\nkeep this\n";
        assert_eq!(strip_block(input, "schema {"), "keep this\n");
    }

    #[test]
    fn strip_block_handles_nested_braces() {
        let input = "type EmptyMutation {\n  nested: Nested {\n    x: Int\n  }\n}\nkeep\n";
        assert_eq!(strip_block(input, "type EmptyMutation"), "keep\n");
    }

    #[test]
    fn strip_block_returns_original_when_marker_missing() {
        assert_eq!(strip_block("hello world", "schema {"), "hello world");
    }

    #[test]
    fn strip_block_returns_original_when_no_brace_after_marker() {
        assert_eq!(strip_block("foo bar", "foo"), "foo bar");
    }

    #[test]
    fn clean_sdl_strips_schema_and_empty_stub_blocks() {
        let sdl = "schema {\n  query: Query\n  mutation: Mutation\n}\n\
                   type EmptyMutation {\n}\n\
                   type EmptySubscription {\n}\n\
                   type Query {\n  placeholder: Boolean!\n}\n";
        assert_eq!(clean_sdl(sdl), "type Query {\n  placeholder: Boolean!\n}");
    }

    #[test]
    fn query_field_names_skips_descriptions_and_keeps_fields() {
        let sdl = "type Query {\n\
                   \x20\x20\"\"\"\n  Multi-line description\n  \"\"\"\n\
                   \x20\x20fieldOne: String\n\
                   \x20\x20\"\"\"One-line description\"\"\"\n\
                   \x20\x20fieldTwo: Int\n\
                   \x20\x20fieldThree: Boolean!\n\
                   }\n";
        let fields = query_field_names(sdl, "Query");
        assert_eq!(
            fields,
            BTreeSet::from([
                "fieldOne".to_string(),
                "fieldTwo".to_string(),
                "fieldThree".to_string()
            ])
        );
    }

    #[test]
    fn query_field_names_returns_empty_when_type_missing() {
        let sdl = "type Foo {\n  a: Int\n}\n";
        assert_eq!(query_field_names(sdl, "Bar"), BTreeSet::new());
    }

    #[test]
    fn index_types_indexes_object_scalar_and_enum_definitions() {
        let sdl = "type Query {\n  widgets: [Widget!]!\n}\n\
                   type Widget {\n  name: String!\n  status: Status!\n}\n\
                   enum Status {\n  ACTIVE\n  INACTIVE\n}\n\
                   scalar DateTime\n";
        let types = index_types(sdl);
        assert_eq!(
            types.keys().cloned().collect::<Vec<_>>(),
            vec![
                "DateTime".to_string(),
                "Query".to_string(),
                "Status".to_string(),
                "Widget".to_string(),
            ]
        );
        assert!(matches!(types["Query"], TypeKind::Object(_)));
        assert!(matches!(types["Widget"], TypeKind::Object(_)));
        assert!(matches!(types["Status"], TypeKind::Enum(_)));
        assert!(matches!(types["DateTime"], TypeKind::Scalar));
    }

    #[test]
    fn index_types_returns_empty_map_on_parse_error() {
        assert_eq!(index_types("type { broken").len(), 0);
    }

    #[test]
    fn synthesize_examples_unwraps_paginated_field() {
        let sdl = "type Query {\n  s3Buckets: S3BucketPage!\n}\n\
                   type S3BucketPage {\n  items: [S3Bucket!]!\n  nextToken: String\n}\n\
                   type S3Bucket {\n  name: String!\n  creationDate: String!\n}\n";
        let examples = synthesize_examples(sdl, "Query");
        assert_eq!(
            examples,
            vec!["{ s3Buckets { items { name creationDate } nextToken } }".to_string()]
        );
    }

    #[test]
    fn synthesize_examples_selects_leaves_directly_for_non_paginated_field() {
        let sdl = "type Query {\n  stsCallerIdentity: StsIdentity!\n}\n\
                   type StsIdentity {\n  account: String!\n  arn: String!\n  userId: String!\n}\n";
        let examples = synthesize_examples(sdl, "Query");
        assert_eq!(
            examples,
            vec!["{ stsCallerIdentity { arn userId account } }".to_string()]
        );
    }

    #[test]
    fn synthesize_examples_fills_required_string_argument() {
        let sdl = "type Query {\n\
                   \x20\x20glueTables(databaseName: String!): GlueTablePage!\n\
                   }\n\
                   type GlueTablePage {\n  items: [GlueTable!]!\n  nextToken: String\n}\n\
                   type GlueTable {\n  name: String!\n}\n";
        let examples = synthesize_examples(sdl, "Query");
        assert_eq!(
            examples,
            vec![
                "{ glueTables(databaseName: \"my-db\") { items { name } nextToken } }".to_string()
            ]
        );
    }

    #[test]
    fn synthesize_examples_degrades_gracefully_with_no_scalar_leaves() {
        let sdl = "type Query {\n  widget: Widget!\n}\n\
                   type Widget {\n  nested: Nested!\n}\n\
                   type Nested {\n  value: String!\n}\n";
        let examples = synthesize_examples(sdl, "Query");
        assert_eq!(examples, vec!["{ widget {  } }".to_string()]);
    }

    #[test]
    fn synthesize_examples_caps_at_three_query_fields() {
        let sdl = "type Query {\n\
                   \x20\x20one: Widget!\n\
                   \x20\x20two: Widget!\n\
                   \x20\x20three: Widget!\n\
                   \x20\x20four: Widget!\n\
                   }\n\
                   type Widget {\n  name: String!\n}\n";
        assert_eq!(synthesize_examples(sdl, "Query").len(), 3);
    }

    #[test]
    fn parse_curated_examples_splits_caption_and_query() {
        let contents = "# List all widgets\n\
                         { widgets { name } }\n\
                         \n\
                         # Get one widget\n\
                         { widget(id: \"abc\") { name } }\n";
        let examples = parse_curated_examples(contents);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].caption.as_deref(), Some("List all widgets"));
        assert_eq!(examples[0].query, "{ widgets { name } }");
        assert_eq!(examples[1].caption.as_deref(), Some("Get one widget"));
        assert_eq!(examples[1].query, "{ widget(id: \"abc\") { name } }");
    }

    #[test]
    fn parse_curated_examples_handles_query_with_no_preceding_comment() {
        let contents = "{ widgets { name } }\n\
                         \n\
                         # Get one widget\n\
                         { widget(id: \"abc\") { name } }\n";
        let examples = parse_curated_examples(contents);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].caption, None);
        assert_eq!(examples[0].query, "{ widgets { name } }");
        assert_eq!(examples[1].caption.as_deref(), Some("Get one widget"));
    }

    #[test]
    fn parse_curated_examples_joins_multiline_query() {
        let contents = "# Nested selection\n\
                         { widget(id: \"abc\") {\n\
                         \x20\x20name\n\
                         \x20\x20status\n\
                         } }\n";
        let examples = parse_curated_examples(contents);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].query, "{ widget(id: \"abc\") { name status } }");
    }

    #[test]
    fn resolve_examples_prefers_curated_over_synthesized() {
        let curated = vec![Example {
            caption: Some("Curated".to_string()),
            query: "{ curated }".to_string(),
        }];
        let synthesized = vec!["{ synthesized }".to_string()];
        let resolved = resolve_examples(Some(curated), synthesized);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].caption.as_deref(), Some("Curated"));
        assert_eq!(resolved[0].query, "{ curated }");
    }

    #[test]
    fn resolve_examples_falls_back_to_synthesized_when_no_curated_file() {
        let synthesized = vec!["{ synthesized }".to_string()];
        let resolved = resolve_examples(None, synthesized);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].caption, None);
        assert_eq!(resolved[0].query, "{ synthesized }");
    }

    #[test]
    fn load_curated_examples_from_reads_file_when_present() {
        let dir = std::env::temp_dir().join(format!(
            "gen_docs_test_{}_{}",
            std::process::id(),
            "load_curated_examples_from_reads_file_when_present"
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("widget.graphql"), "# List\n{ widgets { name } }\n").unwrap();

        let examples = load_curated_examples_from(&dir, "widget").unwrap();
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].caption.as_deref(), Some("List"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_curated_examples_from_returns_none_when_absent() {
        let dir = std::env::temp_dir().join(format!(
            "gen_docs_test_{}_{}",
            std::process::id(),
            "load_curated_examples_from_returns_none_when_absent"
        ));
        fs::create_dir_all(&dir).unwrap();

        assert!(load_curated_examples_from(&dir, "missing-service").is_none());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn render_page_includes_note_when_present() {
        let page = ServicePage {
            slug: "s3",
            title: "S3",
            feature: "s3",
            note: Some("note text"),
            query_type_name: "S3Query".to_string(),
            sdl: "type S3Query {\n  buckets: [S3Bucket!]!\n}".to_string(),
        };
        assert_eq!(
            render_page(&page),
            "# S3\n\nCargo feature: `s3` (`cargo build --features s3`)\n\n\
             > note text\n\n```graphql\ntype S3Query {\n  buckets: [S3Bucket!]!\n}\n```\n"
        );
    }

    #[test]
    fn render_page_omits_note_line_when_none() {
        let page = ServicePage {
            slug: "x",
            title: "X",
            feature: "x",
            note: None,
            query_type_name: "XQuery".to_string(),
            sdl: "type XQuery {\n  a: Int\n}".to_string(),
        };
        assert_eq!(
            render_page(&page),
            "# X\n\nCargo feature: `x` (`cargo build --features x`)\n\n\
             ```graphql\ntype XQuery {\n  a: Int\n}\n```\n"
        );
    }

    #[test]
    fn render_summary_lists_pages_in_order() {
        let pages = vec![
            ServicePage {
                slug: "a",
                title: "Alpha",
                feature: "a",
                note: None,
                query_type_name: "AQuery".to_string(),
                sdl: String::new(),
            },
            ServicePage {
                slug: "b",
                title: "Beta",
                feature: "b",
                note: None,
                query_type_name: "BQuery".to_string(),
                sdl: String::new(),
            },
        ];
        assert_eq!(
            render_summary(&pages),
            "# Summary\n\n[Introduction](introduction.md)\n\n# AWS Services\n\n\
             - [Alpha](services/a.md)\n- [Beta](services/b.md)\n"
        );
    }

    struct HeuristicQuery;

    #[async_graphql::Object]
    impl HeuristicQuery {
        // Never registered via `.data(...)` below, so this always dies at
        // `ctx.data::<String>()?` — after the executor has already started
        // resolving the field, producing a non-empty error `path`.
        async fn needs_context(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<String> {
            Ok(ctx.data::<String>()?.clone())
        }
    }

    #[tokio::test]
    async fn unknown_field_error_has_empty_path() {
        let schema = Schema::build(HeuristicQuery, EmptyMutation, EmptySubscription).finish();
        let resp = schema.execute("{ noSuchField }").await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors.iter().all(|e| e.path.is_empty()));
    }

    #[tokio::test]
    async fn missing_context_data_error_has_non_empty_path() {
        let schema = Schema::build(HeuristicQuery, EmptyMutation, EmptySubscription).finish();
        let resp = schema.execute("{ needsContext }").await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors.iter().all(|e| !e.path.is_empty()));
    }
}
