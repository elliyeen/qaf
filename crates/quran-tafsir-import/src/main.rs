//! quran-tafsir-import — populate the `reflections` table from quran.com tafsirs.
//!
//! Usage
//! ─────
//!   # List available tafsirs and their IDs
//!   quran-tafsir-import --list
//!
//!   # Import Ibn Kathir (id 169, English) into the default DB
//!   quran-tafsir-import --tafsir-id 169
//!
//!   # Import into an explicit DB file, wiping previous rows for this tafsir first
//!   quran-tafsir-import --tafsir-id 169 \
//!     --db "sqlite:/abs/path/qaf.db?mode=rwc" \
//!     --reset
//!
//! Idempotency
//! ───────────
//! Without --reset, already-imported verses are silently skipped
//! (INSERT OR IGNORE keyed on surah + ayah + source slug).

mod client;
mod importer;

use anyhow::Result;
use clap::Parser;
use client::QuranClient;
use quran_db::{connect, run_migrations};
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "quran-tafsir-import",
    about = "Import tafsir from quran.com into the qaf reflections table",
    long_about = None
)]
struct Args {
    /// Print available tafsirs and exit (no DB needed).
    #[arg(long)]
    list: bool,

    /// Tafsir resource ID to import (see --list for IDs).
    /// Required unless --list is passed.
    #[arg(long, value_name = "ID")]
    tafsir_id: Option<u32>,

    /// SQLite database URL.
    /// Defaults to DATABASE_URL env var, then sqlite:qaf.db.
    /// Use an absolute path with ?mode=rwc to avoid CANTOPEN errors:
    ///   sqlite:/Users/you/qaf/qaf.db?mode=rwc
    #[arg(long, value_name = "URL")]
    db: Option<String>,

    /// Delete all existing reflections for this tafsir before importing.
    /// Safe to omit — re-runs are idempotent without it.
    #[arg(long)]
    reset: bool,

    /// Suppress the progress bar (useful in CI / log-only contexts).
    #[arg(long)]
    no_progress: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "quran_tafsir_import=info,warn".into()),
        )
        .init();

    let args = Args::parse();
    let client = QuranClient::new()?;

    // ── --list ────────────────────────────────────────────────────────────
    if args.list {
        let tafsirs = client.list_tafsirs().await?;
        println!("\n{:<6}  {:<12}  {:<30}  {}", "ID", "Lang", "Name", "Author");
        println!("{}", "─".repeat(72));
        for t in &tafsirs {
            println!(
                "{:<6}  {:<12}  {:<30}  {}",
                t.id,
                t.language_name,
                t.name,
                t.author_name
            );
        }
        println!("\n{} tafsirs available.", tafsirs.len());
        return Ok(());
    }

    // ── Require --tafsir-id when not --list ───────────────────────────────
    let tafsir_id = args.tafsir_id.ok_or_else(|| {
        anyhow::anyhow!("--tafsir-id is required (or use --list to see available tafsirs)")
    })?;

    // ── Connect to DB ─────────────────────────────────────────────────────
    let database_url = args
        .db
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:qaf.db".into());

    info!("connecting to {}", database_url);
    let pool = connect(&database_url).await?;
    run_migrations(&pool).await?;

    // ── Run import ────────────────────────────────────────────────────────
    let stats = importer::import(
        &pool,
        &client,
        tafsir_id,
        args.reset,
        !args.no_progress,
    )
    .await?;

    info!(
        "import complete — chapters: {} | verses fetched: {} | rows inserted: {} | skipped: {}",
        stats.chapters_processed,
        stats.verses_fetched,
        stats.rows_inserted,
        stats.rows_skipped,
    );

    println!(
        "\n  Chapters processed : {}\n  Verses fetched     : {}\n  Rows inserted      : {}\n  Rows skipped       : {}\n",
        stats.chapters_processed,
        stats.verses_fetched,
        stats.rows_inserted,
        stats.rows_skipped,
    );

    Ok(())
}
