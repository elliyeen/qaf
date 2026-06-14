//! Fetch tafsir from quran.com and insert it into the `reflections` table.
//!
//! Strategy
//! ────────
//! 1. Resolve the tafsir resource (name, author, slug, language).
//! 2. For each of the 114 surahs, call `client.chapter_tafsir()`.
//! 3. Parse the `verse_key` ("surah:ayah") into (surah, ayah) integers.
//! 4. INSERT OR IGNORE into `reflections` keyed on (surah, ayah, source)
//!    where `source` is the tafsir slug (e.g. "en-tafsir-ibn-kathir").
//! 5. Commit in batches of 50 chapters (~7 × 50 = 350 verses per commit max).
//!
//! Idempotency
//! ───────────
//! Re-running without `--reset` is safe: (surah, ayah, source) collisions
//! are silently skipped via INSERT OR IGNORE.
//! With `--reset`, existing rows for this tafsir slug are deleted first.

use crate::client::{QuranClient, TafsirResource};
use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::SqlitePool;
use tracing::{debug, warn};

#[derive(Debug, Default)]
pub struct ImportStats {
    pub chapters_processed: u32,
    pub verses_fetched: u64,
    pub rows_inserted: u64,
    pub rows_skipped: u64,
}

pub async fn import(
    pool: &SqlitePool,
    client: &QuranClient,
    tafsir_id: u32,
    reset: bool,
    show_progress: bool,
) -> Result<ImportStats> {
    // ── Resolve tafsir metadata ────────────────────────────────────────────
    let resources = client
        .list_tafsirs()
        .await
        .context("could not fetch tafsir list from quran.com")?;

    let resource: &TafsirResource = resources
        .iter()
        .find(|r| r.id == tafsir_id)
        .with_context(|| format!("tafsir id {tafsir_id} not found in quran.com resource list"))?;

    let author = &resource.author_name;
    let source = &resource.slug;          // stable, human-readable identifier
    let lang   = &resource.language_name; // "english", "arabic", etc.

    tracing::info!(
        "importing tafsir {} — \"{}\" by {} ({})",
        tafsir_id, resource.name, author, lang
    );

    // ── Optional reset ─────────────────────────────────────────────────────
    if reset {
        warn!("--reset: deleting reflections with source = '{}'", source);
        sqlx::query("DELETE FROM reflections WHERE source = ?")
            .bind(source)
            .execute(pool)
            .await
            .context("reset DELETE failed")?;
    }

    // ── Progress bar ───────────────────────────────────────────────────────
    let pb = if show_progress {
        let pb = ProgressBar::new(114);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] surah {pos}/114 ({eta})",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let mut stats = ImportStats::default();

    // ── Main loop: one chapter at a time ───────────────────────────────────
    let mut tx = pool.begin().await?;
    let mut batch_chapters: u32 = 0;

    for chapter in 1u32..=114 {
        let verses = client
            .chapter_tafsir(tafsir_id, chapter)
            .await
            .with_context(|| format!("failed fetching tafsir for surah {chapter}"))?;

        stats.verses_fetched += verses.len() as u64;

        for verse in &verses {
            // Parse "surah:ayah" → (i32, i32)
            let (surah, ayah) = parse_verse_key(&verse.verse_key)
                .with_context(|| format!("bad verse_key: {}", verse.verse_key))?;

            let body = match &verse.text {
                Some(t) if !t.trim().is_empty() => t,
                _ => {
                    debug!("skipping empty tafsir for {}", verse.verse_key);
                    stats.rows_skipped += 1;
                    continue;
                }
            };

            let result = sqlx::query(
                "INSERT OR IGNORE INTO reflections
                 (surah, ayah, body, author, source, lang)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(surah)
            .bind(ayah)
            .bind(body)
            .bind(author)
            .bind(source)
            .bind(lang)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("INSERT failed for {}", verse.verse_key))?;

            if result.rows_affected() > 0 {
                stats.rows_inserted += 1;
            } else {
                stats.rows_skipped += 1;
            }
        }

        stats.chapters_processed += 1;
        batch_chapters += 1;

        // Commit every 10 chapters to keep the transaction small.
        if batch_chapters >= 10 {
            tx.commit().await?;
            tx = pool.begin().await?;
            batch_chapters = 0;
        }

        if let Some(pb) = &pb {
            pb.inc(1);
        }
    }

    tx.commit().await?;

    if let Some(pb) = pb {
        pb.finish_with_message("done");
    }

    Ok(stats)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_verse_key(key: &str) -> Result<(i32, i32)> {
    let mut parts = key.splitn(2, ':');
    let surah: i32 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing surah in key"))?
        .parse()
        .context("surah is not an integer")?;
    let ayah: i32 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing ayah in key"))?
        .parse()
        .context("ayah is not an integer")?;
    if !(1..=114).contains(&surah) {
        bail!("surah {surah} out of range");
    }
    Ok((surah, ayah))
}
