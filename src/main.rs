#![recursion_limit = "1024"]

mod aws;
mod error;
mod schema;
mod server;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Compact,
}

#[derive(Parser)]
#[command(name = "vapor", about = "GraphQL interface over AWS APIs")]
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
    Serve {
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        region: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Query { query, region, format } => {
            let config = aws::config::load_aws_config(region.as_deref()).await;
            let schema = schema::root::build_schema(&config);
            let result = schema.execute(&query).await;

            let output = match format {
                OutputFormat::Json => serde_json::to_string_pretty(&result).unwrap(),
                OutputFormat::Compact => serde_json::to_string(&result).unwrap(),
            };
            println!("{output}");

            if !result.errors.is_empty() {
                for error in &result.errors {
                    eprintln!("Error: {}", error.message);
                }
                std::process::exit(1);
            }
        }
        Commands::Serve { port, region } => {
            let port = port.unwrap_or(4000);
            let config = aws::config::load_aws_config(region.as_deref()).await;
            let schema = schema::root::build_schema(&config);
            server::run_server(schema, port).await;
        }
    }
}
