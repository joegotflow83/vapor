#![recursion_limit = "1024"]

mod aws;
mod error;
mod readonly;
mod schema;
mod server;
mod services;
mod telemetry;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Compact,
}

#[derive(Parser)]
#[command(
    name = "vapor",
    about = "GraphQL interface over AWS APIs",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Query {
        query: String,
        #[arg(long)]
        region: Option<String>,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// Execute a GraphQL query, refusing anything that is not a read.
    ///
    /// Mutations and subscriptions are rejected before AWS credentials are
    /// touched, so this subcommand is safe for an agent to run unattended.
    Read {
        query: String,
        #[arg(long)]
        region: Option<String>,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// List the AWS services compiled into this binary.
    Services,
    Serve {
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        region: Option<String>,
        /// Address to bind the server to. Defaults to 127.0.0.1 (loopback
        /// only). Pass 0.0.0.0 to accept connections from other hosts —
        /// this requires --auth-token (or VAPOR_AUTH_TOKEN) since the
        /// server exposes AWS-mutating operations.
        #[arg(long)]
        bind: Option<String>,
        /// Bearer token required on every request (Authorization: Bearer
        /// <token>). Optional on loopback binds, mandatory otherwise. Can
        /// also be set via VAPOR_AUTH_TOKEN.
        #[arg(long, env = "VAPOR_AUTH_TOKEN", hide_env_values = true)]
        auth_token: Option<String>,
    },
    /// Print the version of this binary.
    Version,
}

fn render(value: &serde_json::Value, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(value).unwrap(),
        OutputFormat::Compact => serde_json::to_string(value).unwrap(),
    }
}

/// Run a GraphQL document against AWS and print the response envelope. Exits
/// non-zero if the response carried any errors.
async fn execute(query: &str, region: Option<&str>, format: &OutputFormat) {
    let config = aws::config::load_aws_config(region).await;
    let schema = schema::root::build_schema(&config);
    let result = schema.execute(query).await;
    let envelope = serde_json::to_value(&result).unwrap();
    println!("{}", render(&envelope, format));

    if !result.errors.is_empty() {
        for error in &result.errors {
            eprintln!("Error: {}", error.message);
        }
        std::process::exit(1);
    }
}

/// Report a refused write in the same envelope shape a GraphQL error would
/// take, so callers can parse one response format regardless of outcome.
fn refuse(message: &str, format: &OutputFormat) -> ! {
    let envelope = serde_json::json!({
        "data": null,
        "errors": [{
            "message": message,
            "extensions": { "code": readonly::READ_ONLY_VIOLATION },
        }],
    });
    println!("{}", render(&envelope, format));
    eprintln!("Error: {message}");
    std::process::exit(1);
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Query {
            query,
            region,
            format,
        } => {
            execute(&query, region.as_deref(), &format).await;
        }
        Commands::Read {
            query,
            region,
            format,
        } => {
            if let Err(message) = readonly::ensure_read_only(&query) {
                refuse(&message, &format);
            }
            execute(&query, region.as_deref(), &format).await;
        }
        Commands::Services => {
            println!("vapor {}", env!("CARGO_PKG_VERSION"));
            println!(
                "services ({}): {}",
                services::ENABLED_SERVICES.len(),
                services::ENABLED_SERVICES.join(" ")
            );
        }
        Commands::Serve {
            port,
            region,
            bind,
            auth_token,
        } => {
            let port = port.unwrap_or(4000);
            let bind = bind.unwrap_or_else(|| "127.0.0.1".to_string());
            let config = aws::config::load_aws_config(region.as_deref()).await;
            let schema = schema::root::build_schema(&config);
            server::run_server(schema, port, &bind, auth_token).await;
        }
        Commands::Version => {
            println!("vapor {}", env!("CARGO_PKG_VERSION"));
        }
    }
}
