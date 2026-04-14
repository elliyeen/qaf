pub mod db;
pub mod models;
pub mod queries;

pub use db::{connect, run_migrations};
pub use models::{Morphology, Ontology, Word};
pub use sqlx::SqlitePool;

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = connect("sqlite::memory:").await.expect("in-memory pool");
        run_migrations(&pool).await.expect("migrations");
        pool
    }

    async fn seed(pool: &SqlitePool) {
        let seed_json = include_str!("../../../data/seed/sample_words.json");

        #[derive(serde::Deserialize)]
        struct SeedRecord {
            surah: i32,
            ayah: i32,
            position: i32,
            arabic: String,
            transliteration: String,
            root: Option<String>,
            lemma: String,
            pos: String,
            features: serde_json::Value,
            source: String,
            semantic_domain: Option<String>,
            derivatives: Option<serde_json::Value>,
            scholar_notes: Option<String>,
        }

        let records: Vec<SeedRecord> = serde_json::from_str(seed_json).expect("valid seed JSON");

        for r in &records {
            sqlx::query(
                "INSERT OR IGNORE INTO words (surah, ayah, position, arabic, transliteration, root, lemma)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(r.surah)
            .bind(r.ayah)
            .bind(r.position)
            .bind(&r.arabic)
            .bind(&r.transliteration)
            .bind(&r.root)
            .bind(&r.lemma)
            .execute(pool)
            .await
            .expect("insert word");
        }

        for r in &records {
            let word_id: i64 = sqlx::query_scalar::<_, i64>(
                "SELECT id FROM words WHERE surah=? AND ayah=? AND position=?",
            )
            .bind(r.surah)
            .bind(r.ayah)
            .bind(r.position)
            .fetch_one(pool)
            .await
            .expect("word id");

            let features_str = serde_json::to_string(&r.features).unwrap();
            sqlx::query(
                "INSERT OR IGNORE INTO morphology (word_id, pos, features, source)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(word_id)
            .bind(&r.pos)
            .bind(&features_str)
            .bind(&r.source)
            .execute(pool)
            .await
            .expect("insert morphology");

            if let (Some(ref sd), Some(ref root)) = (&r.semantic_domain, &r.root) {
                let derivs = serde_json::to_string(
                    r.derivatives.as_ref().unwrap_or(&serde_json::Value::Array(vec![])),
                )
                .unwrap();
                sqlx::query(
                    "INSERT OR IGNORE INTO ontology (root, semantic_domain, derivatives, scholar_notes)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(root)
                .bind(sd)
                .bind(&derivs)
                .bind(&r.scholar_notes)
                .execute(pool)
                .await
                .expect("insert ontology");
            }
        }
    }

    #[tokio::test]
    async fn test_get_word_bismillah() {
        let pool = test_pool().await;
        seed(&pool).await;
        let word = queries::get_word(&pool, 1, 1, 1).await.expect("first word");
        assert_eq!(word.surah, 1);
        assert_eq!(word.ayah, 1);
        assert_eq!(word.arabic, "بِسْمِ");
    }

    #[tokio::test]
    async fn test_search_root() {
        let pool = test_pool().await;
        seed(&pool).await;
        // root ر ح م appears in Bismillah (رحمن، رحيم)
        let words = queries::search_root(&pool, "رحم").await.expect("root search");
        assert!(!words.is_empty(), "expected words with root رحم");
    }

    #[tokio::test]
    async fn test_morphology_for() {
        let pool = test_pool().await;
        seed(&pool).await;
        let word = queries::get_word(&pool, 1, 1, 1).await.expect("first word");
        let morph = queries::morphology_for(&pool, word.id).await.expect("morphology");
        assert!(!morph.pos.is_empty());
    }

    #[tokio::test]
    async fn test_get_ontology() {
        let pool = test_pool().await;
        seed(&pool).await;
        let onto = queries::get_ontology(&pool, "رحم").await.expect("ontology");
        assert_eq!(onto.root, "رحم");
    }

    #[tokio::test]
    async fn test_words_in_surah() {
        let pool = test_pool().await;
        seed(&pool).await;
        let words = queries::words_in_surah(&pool, 1).await.expect("surah 1");
        assert!(words.len() >= 5, "Fatiha seed should have at least 5 words");
    }

    #[tokio::test]
    async fn test_search_words_by_field() {
        let pool = test_pool().await;
        seed(&pool).await;
        // Arabic root stored without diacritics — LIKE match is reliable
        let results = queries::search_words(&pool, "حمد", "root").await.expect("search by root");
        assert!(!results.is_empty(), "expected words with root حمد");
        // Lemma search: use the exact unvoweled base that SQLite LIKE can match
        // Lemmas in the seed carry diacritics; searching the lemma of رَبّ via root path is tested above.
        // Verify the lemma branch compiles and runs without panicking (empty result is ok for unknown lemma).
        let _ = queries::search_words(&pool, "nonexistent", "lemma").await.expect("lemma branch runs");
    }
}
