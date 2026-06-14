use anyhow::Result;
use quran_db::{connect, run_migrations};
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "quran_api=debug,info".into()),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:qaf.db".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    info!("connecting to database: {}", database_url);
    let pool = connect(&database_url).await?;
    run_migrations(&pool).await?;

    let app = quran_api::build_router(pool);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("quran-api listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
