use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

/// Initialize a SQLite connection pool.
/// `database_url` e.g. "sqlite:qaf.db" or "sqlite::memory:"
pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Run embedded migrations from the `migrations/` directory at the workspace root.
/// Call once at startup before serving any requests.
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}
