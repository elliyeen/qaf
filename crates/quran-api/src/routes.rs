use crate::handlers;
use axum::{
    routing::get,
    Router,
};
use sqlx::SqlitePool;

pub fn build_router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/word/:surah/:ayah/:pos", get(handlers::get_word))
        .route("/root/:root", get(handlers::get_root))
        .route("/morphology/:word_id", get(handlers::get_morphology))
        .route("/search", get(handlers::search))
        .route("/surah/:num/words", get(handlers::surah_words))
        .route("/ontology/:root", get(handlers::get_ontology))
        .with_state(pool)
}
