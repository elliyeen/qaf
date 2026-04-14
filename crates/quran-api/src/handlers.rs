use crate::errors::ApiError;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use quran_db::queries;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

fn to_json<T: serde::Serialize>(v: T) -> Result<Json<Value>, ApiError> {
    Ok(Json(serde_json::to_value(v).map_err(|e| ApiError::Internal(e.into()))?))
}

pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

pub async fn get_word(
    State(pool): State<SqlitePool>,
    Path((surah, ayah, position)): Path<(i32, i32, i32)>,
) -> Result<Json<Value>, ApiError> {
    let word = queries::get_word(&pool, surah, ayah, position)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    to_json(word)
}

pub async fn get_root(
    State(pool): State<SqlitePool>,
    Path(root): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let words = queries::search_root(&pool, &root)
        .await
        .map_err(ApiError::Internal)?;
    to_json(words)
}

pub async fn get_morphology(
    State(pool): State<SqlitePool>,
    Path(word_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let morph = queries::morphology_for(&pool, word_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    to_json(morph)
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub field: Option<String>,
}

pub async fn search(
    State(pool): State<SqlitePool>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, ApiError> {
    if params.q.is_empty() {
        return Err(ApiError::BadRequest("query parameter `q` is required".into()));
    }
    let field = params.field.as_deref().unwrap_or("root");
    let words = queries::search_words(&pool, &params.q, field)
        .await
        .map_err(ApiError::Internal)?;
    to_json(words)
}

pub async fn surah_words(
    State(pool): State<SqlitePool>,
    Path(surah): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    let words = queries::words_in_surah(&pool, surah)
        .await
        .map_err(ApiError::Internal)?;
    to_json(words)
}

pub async fn get_ontology(
    State(pool): State<SqlitePool>,
    Path(root): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let onto = queries::get_ontology(&pool, &root)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    to_json(onto)
}
