/// Batch-import parsed QAC words into the Qaf SQLite database.
///
/// Strategy:
///   1. Parse the full QAC file in memory (fast — it's text).
///   2. Open ONE SQLite transaction.
///   3. For each word: INSERT OR IGNORE into words, fetch the rowid,
///      INSERT OR IGNORE into morphology.
///   4. Collect distinct roots; INSERT OR IGNORE into ontology (stub entries,
///      semantic_domain / derivatives / notes filled in later).
///   5. COMMIT.
///
/// Running the import a second time is idempotent — all inserts use
/// INSERT OR IGNORE and the UNIQUE constraint on (surah, ayah, position).
use crate::qac;
use crate::transliterate::arabic_to_translit;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use quran_db::strip_diacritics;
use sqlx::SqlitePool;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct ImportStats {
    pub words_processed: u64,
    pub words_inserted: u64,
    pub morphology_inserted: u64,
    pub ontology_inserted: u64,
}

pub async fn import(
    pool: &SqlitePool,
    qac_path: &str,
    reset: bool,
    source: &str,
    show_progress: bool,
) -> Result<ImportStats> {
    tracing::info!("parsing QAC file: {}", qac_path);
    let words = qac::parse(qac_path)?;
    let total = words.len() as u64;
    tracing::info!("{} words ready to import", total);

    if reset {
        tracing::warn!("--reset: clearing words, morphology, ontology tables");
        sqlx::query("DELETE FROM morphology").execute(pool).await?;
        sqlx::query("DELETE FROM ontology").execute(pool).await?;
        sqlx::query("DELETE FROM words").execute(pool).await?;
    }

    let pb = if show_progress {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        pb.set_message("importing words");
        Some(pb)
    } else {
        None
    };

    let mut stats = ImportStats::default();
    let mut ontology_roots: BTreeSet<String> = BTreeSet::new();

    // One transaction for the whole corpus.
    let mut tx = pool.begin().await?;

    for word in words.values() {
        stats.words_processed += 1;

        let lemma_bare = strip_diacritics(&word.lemma);
        let translit = arabic_to_translit(&word.arabic);
        let features_str = serde_json::to_string(&word.features)?;

        // Insert word.
        let result = sqlx::query(
            "INSERT OR IGNORE INTO words
             (surah, ayah, position, arabic, transliteration, root, lemma, lemma_bare)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(word.surah)
        .bind(word.ayah)
        .bind(word.position)
        .bind(&word.arabic)
        .bind(&translit)
        .bind(&word.root)
        .bind(&word.lemma)
        .bind(&lemma_bare)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() > 0 {
            stats.words_inserted += 1;
        }

        // Fetch the word's rowid (existing or just-inserted).
        let word_id: i64 = sqlx::query_scalar(
            "SELECT id FROM words WHERE surah=? AND ayah=? AND position=?",
        )
        .bind(word.surah)
        .bind(word.ayah)
        .bind(word.position)
        .fetch_one(&mut *tx)
        .await?;

        // Insert morphology.
        let m_result = sqlx::query(
            "INSERT OR IGNORE INTO morphology (word_id, pos, features, source)
             VALUES (?, ?, ?, ?)",
        )
        .bind(word_id)
        .bind(&word.pos)
        .bind(&features_str)
        .bind(source)
        .execute(&mut *tx)
        .await?;

        if m_result.rows_affected() > 0 {
            stats.morphology_inserted += 1;
        }

        // Collect distinct roots for ontology.
        if let Some(root) = &word.root {
            if !root.is_empty() {
                ontology_roots.insert(root.clone());
            }
        }

        if let Some(pb) = &pb {
            pb.inc(1);
        }

        // Flush to DB every 5000 words so SQLite's page cache doesn't bloat.
        // We commit the batch and start a new transaction to keep memory usage flat.
        if stats.words_processed % 5000 == 0 {
            tx.commit().await?;
            tx = pool.begin().await?;
            tracing::debug!("checkpoint at {} words", stats.words_processed);
        }
    }

    // Insert ontology stubs for every distinct root encountered.
    for root in &ontology_roots {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO ontology (root, semantic_domain, derivatives, scholar_notes)
             VALUES (?, ?, ?, ?)",
        )
        .bind(root)
        .bind("")         // semantic_domain: to be enriched later
        .bind("[]")       // derivatives: empty JSON array
        .bind(None::<String>)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() > 0 {
            stats.ontology_inserted += 1;
        }
    }

    tx.commit().await?;

    if let Some(pb) = pb {
        pb.finish_with_message("done");
    }

    Ok(stats)
}
