//! seed-recitations — register recitation variants and import Tanzil ayah texts.
//!
//! Run after `seed-structure` (requires surahs table to exist).
//!
//! ## What it does
//!
//! 1. Registers the recitation catalogue (hafs, khalaf) in `recitations`.
//! 2. Seeds default tajweed colour maps into `tajweed_rule_colors`.
//! 3. Parses one or more Tanzil-format XML files and loads ayah texts
//!    into `recitation_texts`.
//!
//! ## Usage
//!
//! ```bash
//! # Derive Hafs texts from the existing ayahs table (fastest, no network)
//! seed-recitations --from-db
//!
//! # Import Hafs from a local Tanzil XML file
//! seed-recitations --hafs data/import/quran-uthmani.xml
//!
//! # Download Hafs from Tanzil and import
//! seed-recitations --download-hafs
//!
//! # Import Khalaf from a local Tanzil XML file (downloaded from tanzil.net/download/)
//! seed-recitations --from-db --khalaf data/import/quran-khalaf.xml
//!
//! # Re-run safely — all inserts are INSERT OR IGNORE (idempotent).
//! # Use --reset to wipe and re-import from scratch.
//! ```
//!
//! ## Obtaining the Tanzil Khalaf XML
//!
//! 1. Visit <https://tanzil.net/download/>
//! 2. Select "Khalaf" (خلف عن حمزة) from the recitation menu.
//! 3. Choose "XML" format, enable "Include tashkeel", and download.
//! 4. Save the file to `data/import/quran-khalaf.xml`.

mod tanzil {
    //! Tanzil XML parser.
    //!
    //! Tanzil distributes Quranic text in a simple XML schema:
    //!
    //! ```xml
    //! <?xml version="1.0" encoding="UTF-8"?>
    //! <!DOCTYPE quran SYSTEM "quran.dtd">
    //! <quran>
    //!   <sura index="1" name="Al-Faatiha">
    //!     <aya index="1" text="بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ" />
    //!     ...
    //!   </sura>
    //! </quran>
    //! ```
    //!
    //! Returns a flat `Vec<(surah, ayah, text)>` ordered as they appear in the file.

    use anyhow::{Context, Result};
    use quick_xml::{events::Event, Reader};

    /// Parse a Tanzil XML file and return `(surah, ayah, text)` triples.
    pub fn parse(path: &str) -> Result<Vec<(i32, i32, String)>> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read Tanzil XML file '{}'", path))?;

        parse_str(&raw)
            .with_context(|| format!("XML parse error in '{}'", path))
    }

    /// Parse Tanzil XML from a string (also used for downloaded content).
    pub fn parse_str(content: &str) -> Result<Vec<(i32, i32, String)>> {
        // Strip DOCTYPE declaration — quick-xml does not implement a full DTD
        // processor and may emit an error on <!DOCTYPE ...>.
        let content: String = content
            .lines()
            .filter(|l| !l.trim_start().starts_with("<!DOCTYPE"))
            .collect::<Vec<_>>()
            .join("\n");

        let mut reader = Reader::from_str(&content);
        let mut result: Vec<(i32, i32, String)> = Vec::with_capacity(6_300);
        let mut current_sura: i32 = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"sura" => {
                    current_sura = attr_i32(e, b"index")?;
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"aya" => {
                    let ayah = attr_i32(e, b"index")?;
                    let text = attr_str(e, b"text")?;
                    result.push((current_sura, ayah, text));
                }
                Ok(Event::Eof) => break,
                Err(e) => anyhow::bail!("XML error at position {}: {}", reader.buffer_position(), e),
                _ => {}
            }
        }

        anyhow::ensure!(!result.is_empty(), "parsed 0 ayahs — check the XML file format");
        Ok(result)
    }

    fn attr_i32(e: &quick_xml::events::BytesStart, name: &[u8]) -> Result<i32> {
        for attr in e.attributes() {
            let attr = attr.context("malformed XML attribute")?;
            if attr.key.as_ref() == name {
                let v = std::str::from_utf8(attr.value.as_ref())
                    .context("attribute value is not valid UTF-8")?;
                return v.parse::<i32>()
                    .with_context(|| format!("attribute '{}' is not an integer: '{}'",
                        std::str::from_utf8(name).unwrap_or("?"), v));
            }
        }
        anyhow::bail!(
            "required attribute '{}' missing on <{}>",
            std::str::from_utf8(name).unwrap_or("?"),
            std::str::from_utf8(e.name().as_ref()).unwrap_or("?")
        )
    }

    fn attr_str(e: &quick_xml::events::BytesStart, name: &[u8]) -> Result<String> {
        for attr in e.attributes() {
            let attr = attr.context("malformed XML attribute")?;
            if attr.key.as_ref() == name {
                let unescaped = attr.unescape_value()
                    .context("failed to unescape XML attribute value")?;
                return Ok(unescaped.into_owned());
            }
        }
        anyhow::bail!(
            "required attribute '{}' missing on <{}>",
            std::str::from_utf8(name).unwrap_or("?"),
            std::str::from_utf8(e.name().as_ref()).unwrap_or("?")
        )
    }
}

// ─── Recitation catalogue ─────────────────────────────────────────────────────
// (name_slug, rawi_ar, qari_ar, description)

struct RecitationMeta {
    name: &'static str,
    rawi: &'static str,
    qari: &'static str,
    description: &'static str,
}

const RECITATION_CATALOG: &[RecitationMeta] = &[
    RecitationMeta {
        name: "hafs",
        rawi: "حفص بن سليمان الأسدي الكوفي",
        qari: "عاصم بن أبي النجود الكوفي",
        description: "رواية حفص عن عاصم — الرواية الأكثر انتشاراً في العالم الإسلامي.",
    },
    RecitationMeta {
        name: "khalaf",
        rawi: "خلف بن هشام البزار البغدادي",
        qari: "حمزة بن حبيب الزيات الكوفي",
        description: "رواية خلف عن حمزة — إحدى روايتَي قراءة حمزة من القراءات العشر.",
    },
];

// ─── Tajweed colour maps ──────────────────────────────────────────────────────
// Standard Madinah Mushaf colour conventions, used as defaults for all riwayat.
// Rules with no colour (izhar, izhar_shafawi) are intentionally omitted —
// the renderer leaves those spans unstyled.

const STANDARD_COLORS: &[(&str, &str)] = &[
    ("ghunnah",             "#06A94D"),  // green
    ("idgham_ghunnah",      "#06A94D"),  // green
    ("idgham_bila_ghunnah", "#0D5C26"),  // dark green
    ("idgham_shafawi",      "#06A94D"),  // green
    ("ikhfa",               "#3CAA6A"),  // medium green
    ("ikhfa_shafawi",       "#3CAA6A"),  // medium green
    ("iqlab",               "#C8A800"),  // gold
    ("madd_tabii",          "#1F6CB0"),  // blue
    ("madd_muttasil",       "#0D3B7A"),  // dark blue
    ("madd_munfasil",       "#2E6DA8"),  // medium blue
    ("madd_lazim",          "#0D3B7A"),  // dark blue
    ("madd_arid",           "#1F6CB0"),  // blue
    ("madd_lin",            "#4A7FB5"),  // blue-gray
    ("madd_badal",          "#5B9BD5"),  // light blue
    ("qalqalah",            "#8B0000"),  // dark red
    ("sakt",                "#808080"),  // gray (Hafs 4 positions + Khalaf 4 positions)
];

// Additional colours for Khalaf — rules that occur in Khalaf but not (or rarely) in Hafs.
const KHALAF_EXTRA_COLORS: &[(&str, &str)] = &[
    ("imalah",  "#C05400"),  // burnt orange
    ("tashil",  "#7B4FA6"),  // purple
    ("naql",    "#C06818"),  // orange-brown
    ("ishmam",  "#008080"),  // teal
];

// ─── CLI ─────────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use quran_db::{connect, run_migrations};
use sqlx::SqlitePool;
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "seed-recitations",
    about = "Register recitations and import Tanzil XML ayah texts into recitation_texts"
)]
struct Args {
    /// Derive Hafs texts from the ayahs.text_uthmani column already in the DB.
    /// Fastest option — no file or network required.
    /// Cannot be combined with --hafs or --download-hafs.
    #[arg(long, conflicts_with_all = ["hafs", "download_hafs"])]
    from_db: bool,

    /// Path to a local Tanzil Hafs (Uthmani) XML file.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["from_db", "download_hafs"])]
    hafs: Option<String>,

    /// Download Hafs (Uthmani) text directly from Tanzil.
    /// Requires an internet connection.
    #[arg(long, conflicts_with_all = ["hafs", "from_db"])]
    download_hafs: bool,

    /// Path to a local Tanzil Khalaf XML file.
    /// Download from: https://tanzil.net/download/ → select "Khalaf" → XML format.
    #[arg(long, value_name = "FILE")]
    khalaf: Option<String>,

    /// SQLite database URL. Defaults to DATABASE_URL env var or sqlite:qaf.db.
    #[arg(long, value_name = "URL")]
    db: Option<String>,

    /// Delete all existing recitation_texts and tajweed_rule_colors before importing.
    /// The recitations catalogue rows are always upserted (not deleted).
    #[arg(long)]
    reset: bool,

    /// Suppress the progress bar.
    #[arg(long)]
    no_progress: bool,
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "seed_recitations=info,warn".into()),
        )
        .init();

    let args = Args::parse();

    if !args.from_db && !args.download_hafs && args.hafs.is_none() && args.khalaf.is_none() {
        anyhow::bail!(
            "nothing to do — pass --from-db, --hafs FILE, --download-hafs, and/or --khalaf FILE.\n\
             Run with --help for usage."
        );
    }

    let database_url = args
        .db
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:qaf.db".into());

    info!("connecting to {}", database_url);
    let pool = connect(&database_url).await?;
    run_migrations(&pool).await?;

    if args.reset {
        tracing::warn!("--reset: truncating recitation_texts and tajweed_rule_colors");
        sqlx::query("DELETE FROM tajweed_spans").execute(&pool).await?;
        sqlx::query("DELETE FROM recitation_texts").execute(&pool).await?;
        sqlx::query("DELETE FROM tajweed_rule_colors").execute(&pool).await?;
    }

    // Step 1 — seed recitation catalogue.
    seed_catalogue(&pool).await?;

    // Step 2 — seed colour maps.
    seed_colors(&pool).await?;

    // Step 3 — import texts.
    if args.from_db {
        info!("deriving Hafs texts from ayahs.text_uthmani…");
        let n = import_hafs_from_db(&pool, !args.no_progress).await?;
        println!("  Hafs ayahs inserted : {}", n);
    } else if args.download_hafs {
        info!("downloading Hafs (Uthmani) from Tanzil…");
        let content = download_tanzil_hafs().await?;
        let ayahs = tanzil::parse_str(&content)
            .context("failed to parse downloaded Hafs XML")?;
        info!("  parsed {} ayahs from Tanzil Hafs", ayahs.len());
        let n = import_texts(&pool, "hafs", ayahs, "tanzil.net", !args.no_progress).await?;
        println!("  Hafs ayahs inserted : {}", n);
    } else if let Some(ref path) = args.hafs {
        let ayahs = tanzil::parse(path)?;
        info!("  parsed {} ayahs from '{}'", ayahs.len(), path);
        let n = import_texts(&pool, "hafs", ayahs, path, !args.no_progress).await?;
        println!("  Hafs ayahs inserted : {}", n);
    }

    if let Some(ref path) = args.khalaf {
        let ayahs = tanzil::parse(path)?;
        info!("  parsed {} ayahs from '{}'", ayahs.len(), path);
        let n = import_texts(&pool, "khalaf", ayahs, path, !args.no_progress).await?;
        println!("  Khalaf ayahs inserted : {}", n);
    }

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn seed_catalogue(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    for r in RECITATION_CATALOG {
        sqlx::query(
            "INSERT OR IGNORE INTO recitations (name, rawi, qari, description)
             VALUES (?, ?, ?, ?)",
        )
        .bind(r.name)
        .bind(r.rawi)
        .bind(r.qari)
        .bind(r.description)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("insert recitation '{}'", r.name))?;
    }
    tx.commit().await?;
    info!("recitation catalogue ready ({} entries)", RECITATION_CATALOG.len());
    Ok(())
}

async fn seed_colors(pool: &SqlitePool) -> Result<()> {
    // Fetch recitation ids by name.
    let hafs_id: i64 =
        sqlx::query_scalar("SELECT id FROM recitations WHERE name = 'hafs'")
            .fetch_one(pool)
            .await
            .context("recitation 'hafs' not found — run seed_catalogue first")?;

    let khalaf_id: i64 =
        sqlx::query_scalar("SELECT id FROM recitations WHERE name = 'khalaf'")
            .fetch_one(pool)
            .await
            .context("recitation 'khalaf' not found — run seed_catalogue first")?;

    let mut tx = pool.begin().await?;

    for &(rule, color) in STANDARD_COLORS {
        for &rid in &[hafs_id, khalaf_id] {
            sqlx::query(
                "INSERT OR IGNORE INTO tajweed_rule_colors (recitation_id, rule, color_hex)
                 VALUES (?, ?, ?)",
            )
            .bind(rid)
            .bind(rule)
            .bind(color)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("insert color rule '{}' for recitation {}", rule, rid))?;
        }
    }

    for &(rule, color) in KHALAF_EXTRA_COLORS {
        sqlx::query(
            "INSERT OR IGNORE INTO tajweed_rule_colors (recitation_id, rule, color_hex)
             VALUES (?, ?, ?)",
        )
        .bind(khalaf_id)
        .bind(rule)
        .bind(color)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("insert Khalaf color rule '{}'", rule))?;
    }

    tx.commit().await?;
    info!("tajweed colour map seeded");
    Ok(())
}

/// Download Hafs Uthmani XML from Tanzil.
async fn download_tanzil_hafs() -> Result<String> {
    // Tanzil's canonical Hafs Uthmani XML with full tashkeel.
    const URL: &str =
        "https://tanzil.net/pub/quran/quran-uthmani.xml";

    let content = reqwest::get(URL)
        .await
        .with_context(|| format!("GET {} failed", URL))?
        .error_for_status()
        .with_context(|| format!("Tanzil returned an error for {}", URL))?
        .text()
        .await
        .context("failed to read Tanzil response body")?;

    Ok(content)
}

/// Import a list of `(surah, ayah, text)` triples into `recitation_texts`
/// for the named recitation. Returns the number of rows inserted.
async fn import_texts(
    pool: &SqlitePool,
    recitation_name: &str,
    ayahs: Vec<(i32, i32, String)>,
    source: &str,
    show_progress: bool,
) -> Result<u64> {
    let recitation_id: i64 =
        sqlx::query_scalar("SELECT id FROM recitations WHERE name = ?")
            .bind(recitation_name)
            .fetch_one(pool)
            .await
            .with_context(|| format!("recitation '{}' not found in catalogue", recitation_name))?;

    let total = ayahs.len() as u64;
    let pb = if show_progress {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        pb.set_message(format!("importing {}", recitation_name));
        Some(pb)
    } else {
        None
    };

    let mut inserted: u64 = 0;
    let mut tx = pool.begin().await?;

    for (i, (surah, ayah, text)) in ayahs.into_iter().enumerate() {
        let r = sqlx::query(
            "INSERT OR IGNORE INTO recitation_texts
             (recitation_id, surah_id, ayah_number, text, source)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(recitation_id)
        .bind(surah)
        .bind(ayah)
        .bind(&text)
        .bind(source)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("insert recitation_text {}:{} ({})", surah, ayah, recitation_name))?;

        inserted += r.rows_affected();

        if let Some(ref pb) = pb {
            pb.inc(1);
        }

        // Flush every 1000 ayahs.
        if (i + 1) % 1000 == 0 {
            tx.commit().await?;
            tx = pool.begin().await?;
        }
    }

    tx.commit().await?;

    if let Some(pb) = pb {
        pb.finish_with_message("done");
    }

    Ok(inserted)
}

/// Derive Hafs texts from `ayahs.text_uthmani` already in the database.
///
/// This avoids any external dependency — the words table already contains the
/// full Hafs corpus and `seed_ayahs` (run in Step 1) joined them into ayah text.
/// Suitable as the primary Hafs source when a Tanzil XML file is unavailable.
async fn import_hafs_from_db(pool: &SqlitePool, show_progress: bool) -> Result<u64> {
    let recitation_id: i64 =
        sqlx::query_scalar("SELECT id FROM recitations WHERE name = 'hafs'")
            .fetch_one(pool)
            .await
            .context("recitation 'hafs' not found — run seed_catalogue first")?;

    // Fetch all ayahs that have text (text_uthmani is nullable until seeded).
    let rows: Vec<(i32, i32, String)> = sqlx::query_as(
        "SELECT surah_id, ayah_number, text_uthmani
         FROM ayahs
         WHERE text_uthmani IS NOT NULL
         ORDER BY surah_id, ayah_number",
    )
    .fetch_all(pool)
    .await
    .context("fetch ayahs for Hafs import")?;

    let total = rows.len() as u64;
    info!("{} ayahs with text_uthmani found", total);

    let pb = if show_progress {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        pb.set_message("importing hafs (from db)");
        Some(pb)
    } else {
        None
    };

    let mut inserted: u64 = 0;
    let mut tx = pool.begin().await?;

    for (i, (surah_id, ayah_number, text)) in rows.into_iter().enumerate() {
        let r = sqlx::query(
            "INSERT OR IGNORE INTO recitation_texts
             (recitation_id, surah_id, ayah_number, text, source)
             VALUES (?, ?, ?, ?, 'quranic-corpus/seed')",
        )
        .bind(recitation_id)
        .bind(surah_id)
        .bind(ayah_number)
        .bind(&text)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("insert hafs recitation_text {}:{}", surah_id, ayah_number))?;

        inserted += r.rows_affected();

        if let Some(ref pb) = pb {
            pb.inc(1);
        }

        if (i + 1) % 1000 == 0 {
            tx.commit().await?;
            tx = pool.begin().await?;
        }
    }

    tx.commit().await?;

    if let Some(pb) = pb {
        pb.finish_with_message("done");
    }

    Ok(inserted)
}
