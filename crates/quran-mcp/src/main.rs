mod schema;
mod server;

use anyhow::Result;
use quran_db::{connect, run_migrations};
use rmcp::{transport::stdio, ServiceExt};
use server::QuranServer;
use tracing::info;

/// Qaf MCP server — speaks MCP over stdio.
/// Configure in your MCP host (e.g. Claude Desktop) as:
///   command: ["quran-mcp"]
///   env: { DATABASE_URL: "sqlite:/path/to/qaf.db" }
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "quran_mcp=debug,warn".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:qaf.db".into());

    info!("quran-mcp: connecting to {}", database_url);
    let pool = connect(&database_url).await?;
    run_migrations(&pool).await?;

    info!("quran-mcp: serving over stdio");
    let service = QuranServer { pool }
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("serve error: {:?}", e))?;

    service.waiting().await?;
    Ok(())
}
