//! Integration tests for quran-api.
//!
//! Each test spins up the full axum router against an in-memory SQLite database,
//! seeds it with the 7 words from Sūrat al-Fātiḥah (1:1–2), and fires a
//! `tower::ServiceExt::oneshot` request — no real TCP port required.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use quran_api::build_router;
use quran_db::{connect, run_migrations, SqlitePool};
use serde_json::{json, Value};
use tower::ServiceExt;

// ── Percent-encoded Arabic roots used in path / query segments ───────────────
/// رحم  (raḥima – mercy; root of al-Raḥmān + al-Raḥīm)
const ROOT_RAHM_PCT: &str = "%D8%B1%D8%AD%D9%85";
/// حمد  (ḥamida – praise; root of al-ḥamdu)
const ROOT_HAMD_PCT: &str = "%D8%AD%D9%85%D8%AF";

// ── Seed data ─────────────────────────────────────────────────────────────────
const SEED_JSON: &str = include_str!("../../../data/seed/sample_words.json");

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
    features: Value,
    source: String,
    semantic_domain: Option<String>,
    derivatives: Option<Value>,
    scholar_notes: Option<String>,
}

async fn test_pool() -> SqlitePool {
    let pool = connect("sqlite::memory:").await.expect("in-memory pool");
    run_migrations(&pool).await.expect("migrations");
    seed(&pool).await;
    pool
}

async fn seed(pool: &SqlitePool) {
    let records: Vec<SeedRecord> =
        serde_json::from_str(SEED_JSON).expect("valid seed JSON");

    for r in &records {
        let lemma_bare = quran_db::strip_diacritics(&r.lemma);
        sqlx::query(
            "INSERT OR IGNORE INTO words
             (surah, ayah, position, arabic, transliteration, root, lemma, lemma_bare)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(r.surah)
        .bind(r.ayah)
        .bind(r.position)
        .bind(&r.arabic)
        .bind(&r.transliteration)
        .bind(&r.root)
        .bind(&r.lemma)
        .bind(&lemma_bare)
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

        sqlx::query(
            "INSERT OR IGNORE INTO morphology (word_id, pos, features, source)
             VALUES (?, ?, ?, ?)",
        )
        .bind(word_id)
        .bind(&r.pos)
        .bind(serde_json::to_string(&r.features).unwrap())
        .bind(&r.source)
        .execute(pool)
        .await
        .expect("insert morphology");

        if let (Some(sd), Some(root)) = (&r.semantic_domain, &r.root) {
            let derivs = serde_json::to_string(
                r.derivatives.as_ref().unwrap_or(&Value::Array(vec![])),
            )
            .unwrap();
            sqlx::query(
                "INSERT OR IGNORE INTO ontology
                 (root, semantic_domain, derivatives, scholar_notes)
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

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Fire a GET against a fresh pool (single-shot, no shared state).
async fn get(uri: &str) -> (StatusCode, Value) {
    let pool = test_pool().await;
    get_with_pool(&pool, uri).await
}

/// Fire a GET using an existing pool (for multi-step tests).
async fn get_with_pool(pool: &SqlitePool, uri: &str) -> (StatusCode, Value) {
    let app = build_router(pool.clone());
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Fire a POST with a JSON body using an existing pool.
async fn post_with_pool(pool: &SqlitePool, uri: &str, body: Value) -> (StatusCode, Value) {
    let app = build_router(pool.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Fire a PUT with a JSON body using an existing pool.
async fn put_with_pool(pool: &SqlitePool, uri: &str, body: Value) -> (StatusCode, Value) {
    let app = build_router(pool.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Fire a DELETE using an existing pool; returns status only (body often empty).
async fn delete_with_pool(pool: &SqlitePool, uri: &str) -> StatusCode {
    let app = build_router(pool.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    response.status()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// GET /health → { "status": "ok" }
#[tokio::test]
async fn test_health() {
    let (status, body) = get("/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

/// GET /word/1/1/1 → bismillah first word (بِسْمِ)
#[tokio::test]
async fn test_get_word_bismillah() {
    let (status, body) = get("/word/1/1/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["surah"], 1);
    assert_eq!(body["ayah"], 1);
    assert_eq!(body["position"], 1);
    assert_eq!(body["arabic"], "بِسْمِ");
    assert_eq!(body["transliteration"], "bismi");
}

/// GET /word/:s/:a/:p — valid bounds but position not in DB → 404
#[tokio::test]
async fn test_get_word_not_found() {
    // Surah 1, ayah 1 is valid (1–7 ayahs); position 999 does not exist.
    let (status, _) = get("/word/1/1/999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// GET /root/رحم → at least 2 words (al-Raḥmān + al-Raḥīm)
#[tokio::test]
async fn test_get_root_rahm() {
    let uri = format!("/root/{}", ROOT_RAHM_PCT);
    let (status, body) = get(&uri).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("array of words");
    assert!(
        arr.len() >= 2,
        "expected ≥2 words with root رحم, got {}",
        arr.len()
    );
    assert!(arr.iter().all(|w| w["root"] == "رحم"));
}

/// GET /root/حمد → al-ḥamdu (1:2:1)
#[tokio::test]
async fn test_get_root_hamd() {
    let uri = format!("/root/{}", ROOT_HAMD_PCT);
    let (status, body) = get(&uri).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("array");
    assert!(!arr.is_empty(), "expected word with root حمد");
}

/// GET /morphology/1 → morphology for word id 1 (بِسْمِ)
#[tokio::test]
async fn test_get_morphology_word1() {
    let (status, body) = get("/morphology/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["word_id"], 1);
    assert!(!body["pos"].as_str().unwrap_or("").is_empty());
}

/// GET /morphology/:id with unknown id → 404
#[tokio::test]
async fn test_get_morphology_not_found() {
    let (status, _) = get("/morphology/9999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// GET /search?q=رحم → root search returns mercy-related words
#[tokio::test]
async fn test_search_root_rahm() {
    let uri = format!("/search?q={}", ROOT_RAHM_PCT);
    let (status, body) = get(&uri).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("array");
    assert!(!arr.is_empty(), "root search for رحم returned nothing");
}

/// GET /search?q=حمد&field=root → praise words
#[tokio::test]
async fn test_search_root_field_explicit() {
    let uri = format!("/search?q={}&field=root", ROOT_HAMD_PCT);
    let (status, body) = get(&uri).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("array");
    assert!(!arr.is_empty(), "root field search for حمد returned nothing");
}

/// GET /search (missing q) → 400
#[tokio::test]
async fn test_search_missing_q() {
    let (status, _) = get("/search").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// GET /surah/1/words → all 7 seeded words from al-Fātiḥah
#[tokio::test]
async fn test_surah_words() {
    let (status, body) = get("/surah/1/words").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 7, "seed has 7 words; got {}", arr.len());
    // ordered by ayah then position
    assert_eq!(arr[0]["arabic"], "بِسْمِ");
}

/// GET /surah/999/words → 400 Bad Request (out-of-range surah)
#[tokio::test]
async fn test_surah_words_empty() {
    let (status, body) = get("/surah/999/words").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"].as_str().unwrap_or("").contains("invalid surah"),
        "expected 'invalid surah' in error, got: {:?}", body
    );
}

/// GET /ontology/رحم → mercy-and-compassion domain
#[tokio::test]
async fn test_get_ontology_rahm() {
    let uri = format!("/ontology/{}", ROOT_RAHM_PCT);
    let (status, body) = get(&uri).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["root"], "رحم");
    assert_eq!(body["semantic_domain"], "mercy-and-compassion");
    let derivs = body["derivatives"].as_array().expect("derivatives array");
    assert!(!derivs.is_empty());
}

/// GET /ontology/:root for root with no ontology entry → 404
#[tokio::test]
async fn test_get_ontology_not_found() {
    // PN roots (Allah) have no ontology entry in seed
    let (status, _) = get("/ontology/%D9%81%D9%82%D9%87").await; // فقه — not in seed
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── Tadabbur: composite page ──────────────────────────────────────────────────

/// GET /tadabbur/1/1 → page with 4 words (Bismillah) and at least 1 root
#[tokio::test]
async fn test_get_tadabbur_page() {
    let (status, body) = get("/tadabbur/1/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["surah"], 1);
    assert_eq!(body["ayah"], 1);
    let words = body["words"].as_array().expect("words array");
    assert_eq!(words.len(), 4, "Bismillah has 4 words");
    assert!(
        words.iter().all(|w| w["morphology"].is_object()),
        "every word should carry its morphology"
    );
    let roots = body["roots"].as_array().expect("roots array");
    assert!(!roots.is_empty(), "1:1 has at least one root with ontology");
    // reflections / themes / cross_refs start empty
    assert_eq!(body["reflections"].as_array().unwrap().len(), 0);
    assert_eq!(body["themes"].as_array().unwrap().len(), 0);
    assert_eq!(body["cross_refs"].as_array().unwrap().len(), 0);
}

// ── Tadabbur: reflections CRUD ────────────────────────────────────────────────

/// Full reflection lifecycle: create → list → update → delete
#[tokio::test]
async fn test_reflection_crud() {
    let pool = test_pool().await;

    // CREATE — 201 with the new reflection
    let (status, body) = post_with_pool(
        &pool,
        "/tadabbur/1/1/reflect",
        json!({ "body": "The Basmalah opens with Divine mercy.", "author": "Test", "lang": "en" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = body["id"].as_i64().expect("id");
    assert_eq!(body["body"], "The Basmalah opens with Divine mercy.");
    assert_eq!(body["author"], "Test");

    // LIST — should have 1 entry
    let (status, body) = get_with_pool(&pool, "/tadabbur/1/1/reflect").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);

    // UPDATE — new body text
    let (status, body) = put_with_pool(
        &pool,
        &format!("/tadabbur/1/1/reflect/{}", id),
        json!({ "body": "Updated text." }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["body"], "Updated text.");
    assert_eq!(body["id"], id);

    // DELETE — 204 No Content
    let status = delete_with_pool(&pool, &format!("/tadabbur/1/1/reflect/{}", id)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // LIST — should be empty again
    let (status, body) = get_with_pool(&pool, "/tadabbur/1/1/reflect").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

/// POST /tadabbur/:surah/:ayah/reflect with empty body → 400
#[tokio::test]
async fn test_create_reflection_empty_body() {
    let pool = test_pool().await;
    let (status, _) = post_with_pool(
        &pool,
        "/tadabbur/1/1/reflect",
        json!({ "body": "   " }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// PUT /tadabbur/:s/:a/reflect/:id that does not exist → 404
#[tokio::test]
async fn test_update_reflection_not_found() {
    let pool = test_pool().await;
    let (status, _) = put_with_pool(
        &pool,
        "/tadabbur/1/1/reflect/9999",
        json!({ "body": "Nope." }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// DELETE /tadabbur/:s/:a/reflect/:id that does not exist → 404
#[tokio::test]
async fn test_delete_reflection_not_found() {
    let pool = test_pool().await;
    let status = delete_with_pool(&pool, "/tadabbur/1/1/reflect/9999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── Tadabbur: themes CRUD ─────────────────────────────────────────────────────

/// Full theme lifecycle: create theme → tag ayah → list ayah themes
#[tokio::test]
async fn test_theme_lifecycle() {
    let pool = test_pool().await;

    // GET /themes (empty)
    let (status, body) = get_with_pool(&pool, "/themes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);

    // POST /themes → 201
    let (status, body) = post_with_pool(
        &pool,
        "/themes",
        json!({ "name_ar": "الرحمة", "name_en": "Mercy", "description": "Divine mercy" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let theme_id = body["id"].as_i64().expect("theme id");
    assert_eq!(body["name_en"], "Mercy");

    // GET /themes → 1 theme
    let (status, body) = get_with_pool(&pool, "/themes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    // POST /tadabbur/1/1/themes/:theme_id → 201
    let (status, _) = post_with_pool(
        &pool,
        &format!("/tadabbur/1/1/themes/{}", theme_id),
        json!({ "note": "Bismillah names ar-Raḥmān and ar-Raḥīm" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Idempotent: tag again → still 201
    let (status, _) = post_with_pool(
        &pool,
        &format!("/tadabbur/1/1/themes/{}", theme_id),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // GET /tadabbur/1/1/themes → 1 theme
    let (status, body) = get_with_pool(&pool, "/tadabbur/1/1/themes").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name_en"], "Mercy");
}

/// POST /themes with missing fields → 400
#[tokio::test]
async fn test_create_theme_missing_fields() {
    let pool = test_pool().await;
    let (status, _) =
        post_with_pool(&pool, "/themes", json!({ "name_ar": "الرحمة" })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// POST /tadabbur/:s/:a/themes/:id with nonexistent theme → 404
#[tokio::test]
async fn test_tag_nonexistent_theme() {
    let pool = test_pool().await;
    let (status, _) =
        post_with_pool(&pool, "/tadabbur/1/1/themes/9999", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── Tadabbur: cross-references CRUD ──────────────────────────────────────────

/// Create a cross-reference then list it
#[tokio::test]
async fn test_xref_create_and_list() {
    let pool = test_pool().await;

    // GET (empty)
    let (status, body) = get_with_pool(&pool, "/tadabbur/1/1/xref").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);

    // POST → 201
    let (status, body) = post_with_pool(
        &pool,
        "/tadabbur/1/1/xref",
        json!({
            "to_surah": 27,
            "to_ayah": 30,
            "relation": "repeats",
            "note": "Bismillah appears verbatim in Sūrat al-Naml 27:30"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {}", body);
    assert_eq!(body["from_surah"], 1);
    assert_eq!(body["to_surah"], 27);
    assert_eq!(body["relation"], "repeats");

    // GET → 1 xref
    let (status, body) = get_with_pool(&pool, "/tadabbur/1/1/xref").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

/// POST xref with invalid relation → 400
#[tokio::test]
async fn test_xref_invalid_relation() {
    let pool = test_pool().await;
    let (status, _) = post_with_pool(
        &pool,
        "/tadabbur/1/1/xref",
        json!({ "to_surah": 2, "to_ayah": 1, "relation": "random" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// POST duplicate xref → 400 (already exists)
#[tokio::test]
async fn test_xref_duplicate() {
    let pool = test_pool().await;
    let body = json!({ "to_surah": 27, "to_ayah": 30, "relation": "repeats" });
    let (s1, _) = post_with_pool(&pool, "/tadabbur/1/1/xref", body.clone()).await;
    let (s2, _) = post_with_pool(&pool, "/tadabbur/1/1/xref", body).await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(s2, StatusCode::BAD_REQUEST);
}

// ── Irab CRUD ─────────────────────────────────────────────────────────────────

/// Full irab lifecycle: create → get → update → delete
#[tokio::test]
async fn test_irab_crud() {
    let pool = test_pool().await;

    // POST /word/1/1/1/irab — بِسْمِ is ism, majrur, kasra (مضاف إليه).
    let (status, body) = post_with_pool(
        &pool,
        "/word/1/1/1/irab",
        json!({
            "word_type": "ism",
            "case_marker": "majrur",
            "case_sign": "kasra",
            "grammatical_function": "mudaf_ilayh",
            "note": "اسم مجرور بحرف الجر وعلامة جره الكسرة الظاهرة على آخره"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {}", body);
    let id = body["id"].as_i64().expect("id");
    assert_eq!(body["word_type"], "ism");
    assert_eq!(body["case_marker"], "majrur");
    assert_eq!(body["case_sign"], "kasra");
    assert_eq!(body["grammatical_function"], "mudaf_ilayh");
    assert_eq!(body["source"], "manual");
    let word_id = body["word_id"].as_i64().expect("word_id");

    // GET /irab/:word_id
    let (status, body) = get_with_pool(&pool, &format!("/irab/{}", word_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
    assert_eq!(body["word_type"], "ism");

    // GET /tadabbur/1/1/irab — should contain our record
    let (status, body) = get_with_pool(&pool, "/tadabbur/1/1/irab").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["case_marker"], "majrur");

    // PUT /irab/id/:id — correct the grammatical function
    let (status, body) = put_with_pool(
        &pool,
        &format!("/irab/id/{}", id),
        json!({
            "word_type": "ism",
            "case_marker": "majrur",
            "case_sign": "kasra",
            "grammatical_function": "mudaf",
            "note": "مضاف مجرور"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "PUT body: {}", body);
    assert_eq!(body["grammatical_function"], "mudaf");

    // DELETE /irab/id/:id → 204
    let status = delete_with_pool(&pool, &format!("/irab/id/{}", id)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // GET /irab/:word_id → 404 now
    let (status, _) = get_with_pool(&pool, &format!("/irab/{}", word_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// POST /word/:s/:a/:p/irab with invalid word_type → 400
#[tokio::test]
async fn test_irab_invalid_word_type() {
    let pool = test_pool().await;
    let (status, body) = post_with_pool(
        &pool,
        "/word/1/1/1/irab",
        json!({ "word_type": "noun" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"].as_str().unwrap_or("").contains("word_type"),
        "expected word_type error, got: {}", body
    );
}

/// POST /word/:s/:a/:p/irab with invalid case_marker → 400
#[tokio::test]
async fn test_irab_invalid_case_marker() {
    let pool = test_pool().await;
    let (status, body) = post_with_pool(
        &pool,
        "/word/1/1/1/irab",
        json!({ "word_type": "ism", "case_marker": "genitive" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap_or("").contains("case_marker"));
}

/// POST duplicate irab for same word → 400
#[tokio::test]
async fn test_irab_duplicate() {
    let pool = test_pool().await;
    let payload = json!({ "word_type": "ism", "case_marker": "majrur", "case_sign": "kasra" });
    let (s1, _) = post_with_pool(&pool, "/word/1/1/1/irab", payload.clone()).await;
    let (s2, body) = post_with_pool(&pool, "/word/1/1/1/irab", payload).await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(s2, StatusCode::BAD_REQUEST, "body: {}", body);
}

/// GET /tadabbur/1/1 — words array should carry irab after insert
#[tokio::test]
async fn test_tadabbur_page_words_carry_irab() {
    let pool = test_pool().await;

    // Before insert: irab field is null.
    let (_, page) = get_with_pool(&pool, "/tadabbur/1/1").await;
    assert!(
        page["words"][0]["irab"].is_null(),
        "irab should be null before insertion"
    );

    // Insert irab for position 1 (بِسْمِ).
    post_with_pool(
        &pool,
        "/word/1/1/1/irab",
        json!({ "word_type": "ism", "case_marker": "majrur", "case_sign": "kasra",
                "grammatical_function": "mudaf_ilayh" }),
    )
    .await;

    // After insert: words[0].irab should be populated.
    let (status, page) = get_with_pool(&pool, "/tadabbur/1/1").await;
    assert_eq!(status, StatusCode::OK);
    let irab = &page["words"][0]["irab"];
    assert!(irab.is_object(), "irab should be an object, got: {}", irab);
    assert_eq!(irab["word_type"], "ism");
    assert_eq!(irab["case_marker"], "majrur");
}

/// DELETE /irab/id/:id that does not exist → 404
#[tokio::test]
async fn test_irab_delete_not_found() {
    let pool = test_pool().await;
    let status = delete_with_pool(&pool, "/irab/id/9999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
