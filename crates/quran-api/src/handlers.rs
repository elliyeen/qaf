use crate::errors::ApiError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use quran_db::{queries, validate_ref, validate_surah, list_recitations, recitation_ayah, VALID_IRAB_CASE_MARKERS, VALID_IRAB_CASE_SIGNS, VALID_IRAB_FUNCTIONS, VALID_IRAB_WORD_TYPES};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

fn to_json<T: serde::Serialize>(v: T) -> Result<Json<Value>, ApiError> {
    Ok(Json(serde_json::to_value(v).map_err(|e| ApiError::Internal(e.into()))?))
}

/// Convert a bounds-validation error to `ApiError::BadRequest`.
#[inline]
fn check_ref(surah: i32, ayah: i32) -> Result<(), ApiError> {
    validate_ref(surah, ayah).map_err(|e| ApiError::BadRequest(e.to_string()))
}

/// Convert a surah-only bounds-validation error to `ApiError::BadRequest`.
#[inline]
fn check_surah(surah: i32) -> Result<(), ApiError> {
    validate_surah(surah).map_err(|e| ApiError::BadRequest(e.to_string()))
}

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "build": option_env!("GIT_SHA").unwrap_or("dev"),
        "env": std::env::var("APP_ENV").unwrap_or_else(|_| "development".into()),
    }))
}

pub async fn get_word(
    State(pool): State<SqlitePool>,
    Path((surah, ayah, position)): Path<(i32, i32, i32)>,
) -> Result<Json<Value>, ApiError> {
    check_ref(surah, ayah)?;
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
    check_surah(surah)?;
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

// ─── Tadabbur handlers ────────────────────────────────────────────────────────

/// GET /tadabbur/:surah/:ayah
/// Returns the full composite page: words + morphology, root ontology,
/// reflections, themes, and cross-references for a single ayah.
pub async fn get_tadabbur_page(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
) -> Result<Json<Value>, ApiError> {
    check_ref(surah, ayah)?;
    let page = queries::tadabbur_page(&pool, surah, ayah)
        .await
        .map_err(ApiError::Internal)?;
    to_json(page)
}

// ── Reflections ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateReflectionBody {
    pub body: String,
    pub author: Option<String>,
    pub source: Option<String>,
    /// ISO 639-1 code; defaults to "en".
    pub lang: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateReflectionBody {
    pub body: String,
}

/// GET /tadabbur/:surah/:ayah/reflect?limit=N
pub async fn list_reflections(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    check_ref(surah, ayah)?;
    let limit: Option<i64> = params
        .get("limit")
        .and_then(|v| v.parse().ok());
    let refs = queries::reflections_for(&pool, surah, ayah, limit)
        .await
        .map_err(ApiError::Internal)?;
    to_json(refs)
}

/// POST /tadabbur/:surah/:ayah/reflect
pub async fn create_reflection(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
    Json(body): Json<CreateReflectionBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    check_ref(surah, ayah)?;
    if body.body.trim().is_empty() {
        return Err(ApiError::BadRequest("`body` must not be empty".into()));
    }
    let lang = body.lang.as_deref().unwrap_or("en");
    let id = queries::insert_reflection(
        &pool,
        surah,
        ayah,
        &body.body,
        body.author.as_deref(),
        body.source.as_deref(),
        lang,
    )
    .await
    .map_err(ApiError::Internal)?;
    let reflection = queries::get_reflection_by_id(&pool, id)
        .await
        .map_err(ApiError::Internal)?;
    let v = serde_json::to_value(reflection).map_err(|e| ApiError::Internal(e.into()))?;
    Ok((StatusCode::CREATED, Json(v)))
}

/// PUT /tadabbur/:surah/:ayah/reflect/:id
pub async fn update_reflection(
    State(pool): State<SqlitePool>,
    Path((_surah, _ayah, id)): Path<(i32, i32, i64)>,
    Json(body): Json<UpdateReflectionBody>,
) -> Result<Json<Value>, ApiError> {
    if body.body.trim().is_empty() {
        return Err(ApiError::BadRequest("`body` must not be empty".into()));
    }
    queries::update_reflection_body(&pool, id, &body.body)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    let reflection = queries::get_reflection_by_id(&pool, id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    to_json(reflection)
}

/// DELETE /tadabbur/:surah/:ayah/reflect/:id
pub async fn delete_reflection(
    State(pool): State<SqlitePool>,
    Path((_surah, _ayah, id)): Path<(i32, i32, i64)>,
) -> Result<StatusCode, ApiError> {
    let deleted = queries::delete_reflection(&pool, id)
        .await
        .map_err(ApiError::Internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("reflection id={} not found", id)))
    }
}

// ── Themes ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateThemeBody {
    pub name_ar: String,
    pub name_en: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct TagThemeBody {
    pub note: Option<String>,
}

/// GET /themes
pub async fn list_themes(
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, ApiError> {
    let themes = queries::list_all_themes(&pool)
        .await
        .map_err(ApiError::Internal)?;
    to_json(themes)
}

/// POST /themes
pub async fn create_theme(
    State(pool): State<SqlitePool>,
    Json(body): Json<CreateThemeBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    if body.name_ar.trim().is_empty() || body.name_en.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "`name_ar` and `name_en` must not be empty".into(),
        ));
    }
    let id = queries::insert_theme(&pool, &body.name_ar, &body.name_en, body.description.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let theme = queries::get_theme_by_id(&pool, id)
        .await
        .map_err(ApiError::Internal)?;
    let v = serde_json::to_value(theme).map_err(|e| ApiError::Internal(e.into()))?;
    Ok((StatusCode::CREATED, Json(v)))
}

/// GET /tadabbur/:surah/:ayah/themes
pub async fn list_ayah_themes(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
) -> Result<Json<Value>, ApiError> {
    check_ref(surah, ayah)?;
    let themes = queries::themes_for(&pool, surah, ayah)
        .await
        .map_err(ApiError::Internal)?;
    to_json(themes)
}

/// POST /tadabbur/:surah/:ayah/themes/:theme_id
pub async fn tag_theme(
    State(pool): State<SqlitePool>,
    Path((surah, ayah, theme_id)): Path<(i32, i32, i64)>,
    body: Option<Json<TagThemeBody>>,
) -> Result<StatusCode, ApiError> {
    check_ref(surah, ayah)?;
    let note = body.as_ref().and_then(|b| b.note.as_deref());
    // Verify theme exists first.
    queries::get_theme_by_id(&pool, theme_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    queries::tag_ayah_theme(&pool, surah, ayah, theme_id, note)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::CREATED)
}

// ── Cross-references ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateXrefBody {
    pub to_surah: i32,
    pub to_ayah: i32,
    /// One of: elaborates | contrasts | repeats | explains | fulfills
    pub relation: String,
    pub note: Option<String>,
}

const VALID_RELATIONS: &[&str] = &["elaborates", "contrasts", "repeats", "explains", "fulfills"];

/// GET /tadabbur/:surah/:ayah/xref
pub async fn list_xrefs(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
) -> Result<Json<Value>, ApiError> {
    check_ref(surah, ayah)?;
    let xrefs = queries::cross_refs_for(&pool, surah, ayah)
        .await
        .map_err(ApiError::Internal)?;
    to_json(xrefs)
}

/// POST /tadabbur/:surah/:ayah/xref
pub async fn create_xref(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
    Json(body): Json<CreateXrefBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    check_ref(surah, ayah)?;
    check_ref(body.to_surah, body.to_ayah)?;
    if !VALID_RELATIONS.contains(&body.relation.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "`relation` must be one of: {}",
            VALID_RELATIONS.join(", ")
        )));
    }
    let maybe_id = queries::insert_cross_ref(
        &pool,
        surah,
        ayah,
        body.to_surah,
        body.to_ayah,
        &body.relation,
        body.note.as_deref(),
    )
    .await
    .map_err(ApiError::Internal)?;
    let id = match maybe_id {
        Some(id) => id,
        None => return Err(ApiError::BadRequest("cross-reference already exists".into())),
    };
    let xref = queries::get_cross_ref_by_id(&pool, id)
        .await
        .map_err(ApiError::Internal)?;
    let v = serde_json::to_value(xref).map_err(|e| ApiError::Internal(e.into()))?;
    Ok((StatusCode::CREATED, Json(v)))
}

// ── Translations ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateTranslationBody {
    pub text: String,
    pub translator: Option<String>,
    /// ISO 639-1 code; defaults to "en".
    pub lang: Option<String>,
    pub source: Option<String>,
}

// ── Irab ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateIrabBody {
    /// النوع: `ism` | `fil` | `harf`
    pub word_type: String,
    /// الإعراب: `marfu` | `mansub` | `majrur` | `majzum` | `mabni`
    pub case_marker: Option<String>,
    /// علامة الإعراب, e.g. `damma`, `fatha`, `waw`.
    pub case_sign: Option<String>,
    /// Syntactic function, e.g. `mubtada`, `fail`, `mafuul_bihi`.
    pub grammatical_function: Option<String>,
    /// Optional sub-classification, e.g. `fil_madhi`.
    pub subtype: Option<String>,
    /// Full Arabic irab phrase (optional).
    pub note: Option<String>,
    /// Provenance; defaults to `"manual"`.
    pub source: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateIrabBody {
    pub word_type: String,
    pub case_marker: Option<String>,
    pub case_sign: Option<String>,
    pub grammatical_function: Option<String>,
    pub subtype: Option<String>,
    pub note: Option<String>,
    pub source: Option<String>,
}

fn validate_irab_body(
    word_type: &str,
    case_marker: Option<&str>,
    case_sign: Option<&str>,
    grammatical_function: Option<&str>,
) -> Result<(), ApiError> {
    if !VALID_IRAB_WORD_TYPES.contains(&word_type) {
        return Err(ApiError::BadRequest(format!(
            "`word_type` must be one of: {}",
            VALID_IRAB_WORD_TYPES.join(", ")
        )));
    }
    if let Some(cm) = case_marker {
        if !VALID_IRAB_CASE_MARKERS.contains(&cm) {
            return Err(ApiError::BadRequest(format!(
                "`case_marker` must be one of: {}",
                VALID_IRAB_CASE_MARKERS.join(", ")
            )));
        }
    }
    if let Some(cs) = case_sign {
        if !VALID_IRAB_CASE_SIGNS.contains(&cs) {
            return Err(ApiError::BadRequest(format!(
                "`case_sign` must be one of: {}",
                VALID_IRAB_CASE_SIGNS.join(", ")
            )));
        }
    }
    if let Some(gf) = grammatical_function {
        if !VALID_IRAB_FUNCTIONS.contains(&gf) {
            return Err(ApiError::BadRequest(format!(
                "`grammatical_function` must be one of: {}",
                VALID_IRAB_FUNCTIONS.join(", ")
            )));
        }
    }
    Ok(())
}

/// GET /irab/:word_id
/// Returns the irab record for a word, identified by its internal id.
pub async fn get_irab(
    State(pool): State<SqlitePool>,
    Path(word_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let irab = queries::get_irab_for_word(&pool, word_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    to_json(irab)
}

/// GET /tadabbur/:surah/:ayah/irab
/// Returns all irab records for every word in the ayah, ordered by position.
pub async fn list_ayah_irab(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
) -> Result<Json<Value>, ApiError> {
    check_ref(surah, ayah)?;
    let irab = queries::irab_for_ayah(&pool, surah, ayah)
        .await
        .map_err(ApiError::Internal)?;
    to_json(irab)
}

/// POST /word/:surah/:ayah/:pos/irab
/// Create an irab record for the word at the given coordinate.
pub async fn create_irab(
    State(pool): State<SqlitePool>,
    Path((surah, ayah, pos)): Path<(i32, i32, i32)>,
    Json(body): Json<CreateIrabBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    check_ref(surah, ayah)?;
    validate_irab_body(
        &body.word_type,
        body.case_marker.as_deref(),
        body.case_sign.as_deref(),
        body.grammatical_function.as_deref(),
    )?;
    // Resolve word_id from coordinate.
    let word = queries::get_word(&pool, surah, ayah, pos)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    let source = body.source.as_deref().unwrap_or("manual");
    let maybe_id = queries::insert_irab(
        &pool,
        word.id,
        &body.word_type,
        body.case_marker.as_deref(),
        body.case_sign.as_deref(),
        body.grammatical_function.as_deref(),
        body.subtype.as_deref(),
        body.note.as_deref(),
        source,
    )
    .await
    .map_err(ApiError::Internal)?;
    let id = match maybe_id {
        Some(id) => id,
        None => return Err(ApiError::BadRequest(
            "irab already exists for this word; use PUT /irab/id/:id to update".into(),
        )),
    };
    let irab = queries::get_irab_by_id(&pool, id)
        .await
        .map_err(ApiError::Internal)?;
    let v = serde_json::to_value(irab).map_err(|e| ApiError::Internal(e.into()))?;
    Ok((StatusCode::CREATED, Json(v)))
}

/// PUT /irab/id/:id
/// Update an existing irab record by its primary key.
pub async fn update_irab(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateIrabBody>,
) -> Result<Json<Value>, ApiError> {
    validate_irab_body(
        &body.word_type,
        body.case_marker.as_deref(),
        body.case_sign.as_deref(),
        body.grammatical_function.as_deref(),
    )?;
    let source = body.source.as_deref().unwrap_or("manual");
    queries::update_irab(
        &pool,
        id,
        &body.word_type,
        body.case_marker.as_deref(),
        body.case_sign.as_deref(),
        body.grammatical_function.as_deref(),
        body.subtype.as_deref(),
        body.note.as_deref(),
        source,
    )
    .await
    .map_err(|e| ApiError::NotFound(e.to_string()))?;
    let irab = queries::get_irab_by_id(&pool, id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    to_json(irab)
}

/// DELETE /irab/id/:id
/// Delete an irab record by its primary key.
pub async fn delete_irab(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let deleted = queries::delete_irab(&pool, id)
        .await
        .map_err(ApiError::Internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("irab id={} not found", id)))
    }
}

/// GET /tadabbur/:surah/:ayah/translations
pub async fn list_translations(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
) -> Result<Json<Value>, ApiError> {
    check_ref(surah, ayah)?;
    let txns = queries::translations_for(&pool, surah, ayah)
        .await
        .map_err(ApiError::Internal)?;
    to_json(txns)
}

// ── Structure routes ──────────────────────────────────────────────────────────

/// GET /surah/:num
/// Returns surah metadata and all its ayahs from the structural tables.
pub async fn get_surah_meta(
    State(pool): State<SqlitePool>,
    Path(num): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    check_surah(num)?;
    let surah = queries::get_surah(&pool, num)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    let ayahs = queries::ayahs_for_surah(&pool, num)
        .await
        .map_err(ApiError::Internal)?;
    to_json(json!({
        "id": surah.id,
        "name_ar": surah.name_ar,
        "name_en": surah.name_en,
        "name_en_meaning": surah.name_en_meaning,
        "revelation_type": surah.revelation_type,
        "ayah_count": surah.ayah_count,
        "ayahs": ayahs,
    }))
}

/// GET /page/:num
/// Returns page metadata and all ayahs on that page (1–604).
pub async fn get_page_meta(
    State(pool): State<SqlitePool>,
    Path(num): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    if !(1..=604).contains(&num) {
        return Err(ApiError::BadRequest(format!(
            "page must be 1–604, got {}",
            num
        )));
    }
    let page = queries::get_page(&pool, num)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    let ayahs = sqlx::query_as::<_, quran_db::Ayah>(
        "SELECT id, surah_id, ayah_number, text_uthmani, page_id, juz_id
         FROM ayahs WHERE page_id = ? ORDER BY surah_id, ayah_number",
    )
    .bind(num)
    .fetch_all(&pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;
    to_json(json!({
        "page": page.id,
        "juz_id": page.juz_id,
        "ayahs": ayahs,
    }))
}

/// GET /juz/:num
/// Returns juz metadata, page range, and first/last ayah coordinates (1–30).
pub async fn get_juz_meta(
    State(pool): State<SqlitePool>,
    Path(num): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    if !(1..=30).contains(&num) {
        return Err(ApiError::BadRequest(format!(
            "juz must be 1–30, got {}",
            num
        )));
    }
    let juz = queries::get_juz(&pool, num)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    let (first_page, last_page) = sqlx::query_as::<_, (Option<i32>, Option<i32>)>(
        "SELECT MIN(id), MAX(id) FROM pages WHERE juz_id = ?",
    )
    .bind(num)
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;
    let start = sqlx::query_as::<_, (i32, i32)>(
        "SELECT surah_id, ayah_number FROM ayahs WHERE juz_id = ?
         ORDER BY surah_id ASC, ayah_number ASC LIMIT 1",
    )
    .bind(num)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;
    let end = sqlx::query_as::<_, (i32, i32)>(
        "SELECT surah_id, ayah_number FROM ayahs WHERE juz_id = ?
         ORDER BY surah_id DESC, ayah_number DESC LIMIT 1",
    )
    .bind(num)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;
    to_json(json!({
        "id": juz.id,
        "name_ar": juz.name_ar,
        "first_page": first_page,
        "last_page": last_page,
        "start": start.map(|(s, a)| json!({ "surah": s, "ayah": a })),
        "end": end.map(|(s, a)| json!({ "surah": s, "ayah": a })),
    }))
}

/// POST /tadabbur/:surah/:ayah/translations
pub async fn create_translation(
    State(pool): State<SqlitePool>,
    Path((surah, ayah)): Path<(i32, i32)>,
    Json(body): Json<CreateTranslationBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    check_ref(surah, ayah)?;
    if body.text.trim().is_empty() {
        return Err(ApiError::BadRequest("`text` must not be empty".into()));
    }
    let lang = body.lang.as_deref().unwrap_or("en");
    let id = queries::insert_translation(
        &pool,
        surah,
        ayah,
        &body.text,
        body.translator.as_deref(),
        lang,
        body.source.as_deref(),
    )
    .await
    .map_err(ApiError::Internal)?;
    // Return the newly-created row.
    let txn = sqlx::query_as::<_, quran_db::AyahTranslation>(
        "SELECT id, surah, ayah, text, translator, lang, source, created_at
         FROM translations WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;
    let v = serde_json::to_value(txn).map_err(|e| ApiError::Internal(e.into()))?;
    Ok((StatusCode::CREATED, Json(v)))
}

// ── Recitation routes ─────────────────────────────────────────────────────────

/// GET /recitations
/// Returns the full recitation catalogue (all riwāyāt registered in the DB).
pub async fn get_recitations(
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, ApiError> {
    let recs = list_recitations(&pool)
        .await
        .map_err(ApiError::Internal)?;
    to_json(recs)
}

/// GET /ayah/:surah/:ayah/recitation/:name
///
/// Returns the ayah text in the requested recitation, together with its
/// tajweed span annotations and the recitation's colour map.
///
/// Response shape:
/// ```json
/// {
///   "surah": 1,
///   "ayah": 1,
///   "recitation": { "id": 1, "name": "hafs", "rawi": "…", "qari": "…", "description": "…" },
///   "text": "بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ",
///   "source": "quranic-corpus/seed",
///   "spans": [
///     { "start_index": 22, "length": 3, "rule": "madd_tabii", "note": null },
///     …
///   ],
///   "colors": {
///     "ghunnah":  "#06A94D",
///     "madd_tabii": "#1F6CB0",
///     …
///   }
/// }
/// ```
///
/// 400 if `:surah` / `:ayah` are out of range.
/// 404 if the recitation name is unknown or its text has not been imported yet.
pub async fn get_recitation_ayah(
    State(pool): State<SqlitePool>,
    Path((surah, ayah, recitation_name)): Path<(i32, i32, String)>,
) -> Result<Json<Value>, ApiError> {
    check_ref(surah, ayah)?;

    let result = recitation_ayah(&pool, surah, ayah, &recitation_name)
        .await
        .map_err(ApiError::Internal)?;

    match result {
        None => Err(ApiError::NotFound(format!(
            "recitation '{}' not found or no text imported for {}:{}",
            recitation_name, surah, ayah
        ))),
        Some((rec, text, source, spans, colors)) => {
            // Convert the flat (rule, color_hex) pairs into a JSON object.
            let colors_obj: serde_json::Map<String, Value> = colors
                .into_iter()
                .map(|(rule, hex)| (rule, Value::String(hex)))
                .collect();

            to_json(json!({
                "surah":       surah,
                "ayah":        ayah,
                "recitation":  rec,
                "text":        text,
                "source":      source,
                "spans":       spans,
                "colors":      colors_obj,
            }))
        }
    }
}
