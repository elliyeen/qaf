use crate::models::{Morphology, Ontology, Word};
use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// Fetch a single word by its Quran coordinate.
pub async fn get_word(pool: &SqlitePool, surah: i32, ayah: i32, position: i32) -> Result<Word> {
    sqlx::query_as::<_, Word>(
        "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma
         FROM words
         WHERE surah = ? AND ayah = ? AND position = ?",
    )
    .bind(surah)
    .bind(ayah)
    .bind(position)
    .fetch_one(pool)
    .await
    .with_context(|| format!("word not found: {}:{}:{}", surah, ayah, position))
}

/// Return every word in the corpus that shares a given Arabic root.
pub async fn search_root(pool: &SqlitePool, root: &str) -> Result<Vec<Word>> {
    sqlx::query_as::<_, Word>(
        "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma
         FROM words
         WHERE root = ?
         ORDER BY surah, ayah, position",
    )
    .bind(root)
    .fetch_all(pool)
    .await
    .with_context(|| format!("root search failed for: {}", root))
}

/// Fetch all words in a surah ordered by ayah then position.
pub async fn words_in_surah(pool: &SqlitePool, surah: i32) -> Result<Vec<Word>> {
    sqlx::query_as::<_, Word>(
        "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma
         FROM words
         WHERE surah = ?
         ORDER BY ayah, position",
    )
    .bind(surah)
    .fetch_all(pool)
    .await
    .with_context(|| format!("words_in_surah failed for surah {}", surah))
}

/// Search words by root or lemma substring.
pub async fn search_words(pool: &SqlitePool, query: &str, field: &str) -> Result<Vec<Word>> {
    let pattern = format!("%{}%", query);
    let sql = match field {
        "lemma" => {
            "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma
             FROM words WHERE lemma LIKE ? ORDER BY surah, ayah, position"
        }
        _ => {
            "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma
             FROM words WHERE root LIKE ? ORDER BY surah, ayah, position"
        }
    };
    sqlx::query_as::<_, Word>(sql)
        .bind(pattern)
        .fetch_all(pool)
        .await
        .with_context(|| format!("search_words failed: field={} q={}", field, query))
}

/// Get morphological data for a word by its id.
pub async fn morphology_for(pool: &SqlitePool, word_id: i64) -> Result<Morphology> {
    #[derive(sqlx::FromRow)]
    struct MorphRow {
        word_id: i64,
        pos: String,
        features: String,
        source: String,
    }

    let row = sqlx::query_as::<_, MorphRow>(
        "SELECT word_id, pos, features, source FROM morphology WHERE word_id = ?",
    )
    .bind(word_id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("morphology not found for word_id {}", word_id))?;

    let features: serde_json::Value = serde_json::from_str(&row.features)
        .unwrap_or(serde_json::Value::Object(Default::default()));

    Ok(Morphology {
        word_id: row.word_id,
        pos: row.pos,
        features,
        source: row.source,
    })
}

/// Get ontology record for an Arabic root.
pub async fn get_ontology(pool: &SqlitePool, root: &str) -> Result<Ontology> {
    #[derive(sqlx::FromRow)]
    struct OntoRow {
        root: String,
        semantic_domain: String,
        derivatives: String,
        scholar_notes: Option<String>,
    }

    let row = sqlx::query_as::<_, OntoRow>(
        "SELECT root, semantic_domain, derivatives, scholar_notes
         FROM ontology WHERE root = ?",
    )
    .bind(root)
    .fetch_one(pool)
    .await
    .with_context(|| format!("ontology not found for root: {}", root))?;

    let derivatives: serde_json::Value = serde_json::from_str(&row.derivatives)
        .unwrap_or(serde_json::Value::Array(vec![]));

    Ok(Ontology {
        root: row.root,
        semantic_domain: row.semantic_domain,
        derivatives,
        scholar_notes: row.scholar_notes,
    })
}
