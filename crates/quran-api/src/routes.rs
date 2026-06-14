use crate::handlers;
use axum::{
    routing::{get, post, put},
    Router,
};
use sqlx::SqlitePool;

pub fn build_router(pool: SqlitePool) -> Router {
    Router::new()
        // ── Existing word / root / morphology routes ──────────────────────────
        .route("/health", get(handlers::health))
        .route("/word/:surah/:ayah/:pos", get(handlers::get_word))
        .route("/root/:root", get(handlers::get_root))
        .route("/morphology/:word_id", get(handlers::get_morphology))
        .route("/search", get(handlers::search))
        .route("/surah/:num", get(handlers::get_surah_meta))
        .route("/surah/:num/words", get(handlers::surah_words))
        .route("/page/:num", get(handlers::get_page_meta))
        .route("/juz/:num", get(handlers::get_juz_meta))
        .route("/ontology/:root", get(handlers::get_ontology))
        // ── Tadabbur: composite page ──────────────────────────────────────────
        .route("/tadabbur/:surah/:ayah", get(handlers::get_tadabbur_page))
        // ── Tadabbur: reflections ─────────────────────────────────────────────
        .route(
            "/tadabbur/:surah/:ayah/reflect",
            get(handlers::list_reflections).post(handlers::create_reflection),
        )
        .route(
            "/tadabbur/:surah/:ayah/reflect/:id",
            put(handlers::update_reflection).delete(handlers::delete_reflection),
        )
        // ── Tadabbur: themes ──────────────────────────────────────────────────
        .route("/themes", get(handlers::list_themes).post(handlers::create_theme))
        .route(
            "/tadabbur/:surah/:ayah/themes",
            get(handlers::list_ayah_themes),
        )
        .route(
            "/tadabbur/:surah/:ayah/themes/:theme_id",
            post(handlers::tag_theme),
        )
        // ── Tadabbur: cross-references ────────────────────────────────────────
        .route(
            "/tadabbur/:surah/:ayah/xref",
            get(handlers::list_xrefs).post(handlers::create_xref),
        )
        // ── Tadabbur: translations ────────────────────────────────────────────
        .route(
            "/tadabbur/:surah/:ayah/translations",
            get(handlers::list_translations).post(handlers::create_translation),
        )
        // ── Irab ─────────────────────────────────────────────────────────────
        // GET by word_id / ayah; POST by word coordinate; PUT+DELETE by irab id.
        .route("/irab/:word_id", get(handlers::get_irab))
        .route(
            "/tadabbur/:surah/:ayah/irab",
            get(handlers::list_ayah_irab),
        )
        .route(
            "/word/:surah/:ayah/:pos/irab",
            post(handlers::create_irab),
        )
        .route(
            "/irab/id/:id",
            put(handlers::update_irab).delete(handlers::delete_irab),
        )
        .with_state(pool)
}
