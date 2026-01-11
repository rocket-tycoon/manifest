use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use rocket_manifest::{api, db, mcp};

#[derive(Parser)]
#[command(name = "rocket-manifest")]
#[command(about = "Living feature documentation for AI-assisted development")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the RocketManifest server
    Serve {
        /// Port for HTTP API
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Run as daemon
        #[arg(short, long)]
        daemon: bool,
    },
    /// Start MCP server via stdio (for Claude Code integration)
    Mcp,
    /// Check server status
    Status,
    /// Stop the daemon
    Stop,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "rocket_manifest=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Serve { port, daemon: _ }) => {
            tracing::info!("Starting RocketManifest server on port {}", port);

            let db = db::Database::open_default()?;
            db.migrate()?;

            let app = api::create_router(db);

            let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
            tracing::info!(
                "RocketManifest server listening on http://127.0.0.1:{}",
                port
            );

            axum::serve(listener, app).await?;
        }
        Some(Commands::Mcp) => {
            let db = db::Database::open_default()?;
            db.migrate()?;

            mcp::run_stdio_server(db).await?;
        }
        Some(Commands::Status) => {
            println!("Checking RocketManifest server status...");
            // TODO: Check if server is running
        }
        Some(Commands::Stop) => {
            println!("Stopping RocketManifest server...");
            // TODO: Stop daemon
        }
        None => {
            // Default: start server
            tracing::info!("Starting RocketManifest server on port 3000");

            let db = db::Database::open_default()?;
            db.migrate()?;

            let app = api::create_router(db);

            let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
            tracing::info!("RocketManifest server listening on http://127.0.0.1:3000");

            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}
