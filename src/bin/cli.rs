use anyhow::Result;
use clap::{Parser, Subcommand};
use rustfmt_sender::{consts::RUSTFMT_SCHEMA_URL, rustfmtSender, BackendConfig};
use std::path::PathBuf;
use tracing::{error, info, Level};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "rustfmt-sender")]
#[command(about = "Send rustfmt-schema file data to a backend server")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Backend server URL
    #[arg(long, default_value = RUSTFMT_SCHEMA_URL)]
    backend_url: String,

    /// Authentication token
    #[arg(long)]
    auth_token: Option<String>,

    /// Request timeout in seconds
    #[arg(long, default_value = "30")]
    timeout: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a single rustfmt-schema file
    Send {
        /// Path to the rustfmt-schema file
        file: PathBuf,
    },
    /// Send multiple rustfmt-schema files
    SendMultiple {
        /// Paths to rustfmt-schema files
        files: Vec<PathBuf>,
    },
    /// Send current process rustfmt-schema variables
    SendCurrent,
    /// Test connection to backend server
    Test,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    // Create backend configuration
    let config = BackendConfig {
        url: cli.backend_url,
        auth_token: cli.auth_token,
        timeout_seconds: Some(cli.timeout),
    };

    // Create rustfmtSender instance
    let sender = rustfmtSender::new(config)?;

    // Execute command
    match cli.command {
        Commands::Send { file } => {
            info!("Sending rustfmt-schema file: {:?}", file);
            match sender.read_and_send(&file).await {
                Ok(()) => {
                    info!("Successfully sent rustfmt-schema data from {:?}", file);
                }
                Err(e) => {
                    error!("Failed to send rustfmt-schema data: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::SendMultiple { files } => {
            info!("Sending {} rustfmt-schema files", files.len());
            if let Err(e) = sender.read_and_send_multiple(&files).await {
                error!("Failed to send some rustfmt-schema files: {}", e);
                std::process::exit(1);
            }
            info!("Successfully processed all rustfmt-schema files");
        }
        Commands::SendCurrent => {
            info!("Sending current process rustfmt-schema variables");
            let rustfmt_data = rustfmt_sender::create_rustfmt_data_from_current();
            if let Err(e) = sender.send_rustfmt_data(&rustfmt_data).await {
                error!("Failed to send current rustfmt-schema data: {}", e);
                std::process::exit(1);
            }
            info!("Successfully sent current rustfmt-schema data");
        }
        Commands::Test => {
            info!("Testing connection to backend server");
            let test_data = rustfmt_sender::rustfmtData {
                source_file: "test".to_string(),
                variables: std::collections::HashMap::new(),
                timestamp: chrono::Utc::now(),
                metadata: None,
            };
            
            match sender.send_rustfmt_data(&test_data).await {
                Ok(()) => {
                    info!("Connection test successful");
                }
                Err(e) => {
                    error!("Connection test failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
