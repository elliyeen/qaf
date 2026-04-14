use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Word {
    pub id: i64,
    pub surah: i32,
    pub ayah: i32,
    pub position: i32,
    pub arabic: String,
    pub transliteration: String,
    pub root: Option<String>,
    pub lemma: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Morphology {
    pub word_id: i64,
    pub pos: String,
    /// JSON object of grammatical features, e.g. {"case":"nominative","number":"singular"}
    pub features: serde_json::Value,
    /// Provenance, e.g. "quranic-corpus"
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Ontology {
    /// Arabic 3-letter root, e.g. "رحم"
    pub root: String,
    pub semantic_domain: String,
    /// JSON array of derivative word forms
    pub derivatives: serde_json::Value,
    pub scholar_notes: Option<String>,
}
