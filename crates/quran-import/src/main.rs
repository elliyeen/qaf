//! quran-import — load the full Quranic Arabic Corpus into qaf.db
//!
//! Usage:
//!   quran-import --qac path/to/quran-morphology.txt
//!   quran-import --qac path/to/quran-morphology.txt --db sqlite:/path/to/qaf.db
//!   quran-import --qac path/to/quran-morphology.txt --reset   # clear tables first
//!   quran-import --qac path/to/quran-morphology.txt --no-progress
//!
//! See data/import/README.md for instructions on downloading the QAC file.

mod importer;
mod qac;
mod transliterate;

use anyhow::Result;
use clap::Parser;
use quran_db::{connect, run_migrations};
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "quran-import",
    about = "Import the full Quranic Arabic Corpus into qaf.db",
    long_about = None
)]
struct Args {
    /// Path to the QAC morphology file (quran-morphology.txt).
    /// Download: https://github.com/mustafa0x/quran-morphology
    #[arg(short, long, value_name = "FILE")]
    qac: String,

    /// SQLite database URL.  Defaults to DATABASE_URL env var or sqlite:qaf.db.
    #[arg(short, long, value_name = "URL")]
    db: Option<String>,

    /// Source label written into morphology.source (for provenance tracking).
    #[arg(long, default_value = "mustafa0x/quran-morphology")]
    source: String,

    /// Delete all existing words / morphology / ontology rows before importing.
    /// Safe to omit for subsequent runs — inserts are idempotent (INSERT OR IGNORE).
    #[arg(long)]
    reset: bool,

    /// Suppress the progress bar (useful for CI / log-only output).
    #[arg(long)]
    no_progress: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "quran_import=info,warn".into()),
        )
        .init();

    let args = Args::parse();

    let database_url = args
        .db
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:qaf.db".into());

    info!("connecting to {}", database_url);
    let pool = connect(&database_url).await?;
    run_migrations(&pool).await?;

    let stats = importer::import(
        &pool,
        &args.qac,
        args.reset,
        &args.source,
        !args.no_progress,
    )
    .await?;

    info!(
        "import complete — words: {}/{} inserted | morphology: {} | ontology roots: {}",
        stats.words_inserted, stats.words_processed,
        stats.morphology_inserted,
        stats.ontology_inserted,
    );

    println!(
        "\n  Words processed : {}\n  Words inserted  : {}\n  Morphology rows : {}\n  Ontology roots  : {}\n",
        stats.words_processed,
        stats.words_inserted,
        stats.morphology_inserted,
        stats.ontology_inserted,
    );

    Ok(())
}
