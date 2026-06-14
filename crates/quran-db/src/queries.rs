use crate::models::{Ayah, AyahTranslation, HadithCrossRef, Juz, Morphology, MushafPage, Ontology, QuranCrossRef, Recitation, Reflection, Surah, TadabburPage, TajweedSpan, Theme, Word, WordDetail, WordIrab, WordSegment, WordToken};
use crate::text::strip_diacritics;
use anyhow::{Context, Result};
use sqlx::SqlitePool;

// ─── Irab controlled vocabulary ──────────────────────────────────────────────

/// Valid values for `WordIrab::word_type` (النوع).
pub const VALID_IRAB_WORD_TYPES: &[&str] = &["ism", "fil", "harf"];

/// Valid values for `WordIrab::case_marker` (الإعراب).
pub const VALID_IRAB_CASE_MARKERS: &[&str] =
    &["marfu", "mansub", "majrur", "majzum", "mabni"];

/// Valid values for `WordIrab::case_sign` (علامة الإعراب).
pub const VALID_IRAB_CASE_SIGNS: &[&str] = &[
    "damma",       // ضمة
    "dammatan",    // ضمتان (tanwin rafa')
    "fatha",       // فتحة
    "fatahtan",    // فتحتان (tanwin nasb)
    "kasra",       // كسرة
    "kasratan",    // كسرتان (tanwin jarr)
    "sukun",       // سكون
    "waw",         // واو (sound masculine plural / dual rafa')
    "alif",        // ألف (dual rafa')
    "ya",          // ياء (dual / sound masc. plural nasb+jarr)
    "nun_deletion",// حذف النون (af'al al-khamsa jussive / nasb)
    "fatha_subst", // فتحة نيابة عن الكسرة (diptote / mamnuʿ min al-sarf)
];

/// Valid values for `WordIrab::grammatical_function` (الوظيفة النحوية).
pub const VALID_IRAB_FUNCTIONS: &[&str] = &[
    "mubtada",      // مبتدأ
    "khabar",       // خبر
    "fail",         // فاعل
    "naib_fail",    // نائب فاعل
    "mafuul_bihi",  // مفعول به
    "mudaf",        // مضاف
    "mudaf_ilayh",  // مضاف إليه
    "nat",          // نعت / صفة
    "hal",          // حال
    "tamyiz",       // تمييز
    "badal",        // بدل
    "atf",          // معطوف
    "tawkid",       // توكيد
    "zarf",         // ظرف
    "mafuul_mutlaq",// مفعول مطلق
    "mafuul_fih",   // مفعول فيه
    "mafuul_lah",   // مفعول لأجله
    "isim_kana",    // اسم كان وأخواتها
    "khabar_kana",  // خبر كان وأخواتها
    "isim_inna",    // اسم إن وأخواتها
    "khabar_inna",  // خبر إن وأخواتها
    "jar",          // جار (حرف الجر نفسه)
    "majrur_bijar", // مجرور بحرف الجر
    "munada",       // منادى
    "fil",          // الفعل نفسه (for verb irab notes)
    "harf",         // الحرف نفسه (لا محل له من الإعراب)
];

// ─── Reference bounds ────────────────────────────────────────────────────────

/// Ayah count per surah in the standard Uthmani muṣḥaf (index 0 = surah 1).
pub const AYAH_COUNTS: [i32; 114] = [
      7, 286, 200, 176, 120, 165, 206,  75, 129, 109, // 1–10
    123, 111,  43,  52,  99, 128, 111, 110,  98, 135, // 11–20
    112,  78, 118,  64,  77, 227,  93,  88,  69,  60, // 21–30
     34,  30,  73,  54,  45,  83, 182,  88,  75,  85, // 31–40
     54,  53,  89,  59,  37,  35,  38,  29,  18,  45, // 41–50
     60,  49,  62,  55,  78,  96,  29,  22,  24,  13, // 51–60
     14,  11,  11,  18,  12,  12,  30,  52,  52,  44, // 61–70
     28,  28,  20,  56,  40,  31,  50,  40,  46,  42, // 71–80
     29,  19,  36,  25,  22,  17,  19,  26,  30,  20, // 81–90
     15,  21,  11,   8,   8,  19,   5,   8,   8,  11, // 91–100
     11,   8,   3,   9,   5,   4,   7,   3,   6,   3, // 101–110
      5,   4,   5,   6,                               // 111–114
];

/// Validate that `surah` is a canonical Quran surah number (1–114).
///
/// Use this in contexts where only the surah is provided (e.g. fetching all
/// words in a surah).  For `(surah, ayah)` pairs use [`validate_ref`].
pub fn validate_surah(surah: i32) -> Result<()> {
    if surah < 1 || surah > 114 {
        anyhow::bail!("invalid surah {}: must be 1–114", surah);
    }
    Ok(())
}

/// Validate that `(surah, ayah)` is a canonical Quran reference.
///
/// Returns `Err` with a human-readable message when:
/// - `surah` is outside `1..=114`
/// - `ayah` is outside `1..=<surah_length>`
///
/// Call this at the top of any query that accepts a Quran coordinate so
/// callers receive a clear error rather than a silent empty result or a
/// generic DB error.
pub fn validate_ref(surah: i32, ayah: i32) -> Result<()> {
    validate_surah(surah)?;
    let max_ayah = AYAH_COUNTS[(surah - 1) as usize];
    if ayah < 1 || ayah > max_ayah {
        anyhow::bail!(
            "invalid ayah {} for surah {}: must be 1–{}",
            ayah, surah, max_ayah
        );
    }
    Ok(())
}

// ─── Core word queries ────────────────────────────────────────────────────────

/// Fetch a single word by its Quran coordinate.
pub async fn get_word(pool: &SqlitePool, surah: i32, ayah: i32, position: i32) -> Result<Word> {
    validate_ref(surah, ayah)?;
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
    validate_surah(surah)?;
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
///
/// Lemma search is diacritic-insensitive: both the stored value and the
/// caller's query are stripped of harakat via `text::strip_diacritics`
/// before the LIKE comparison.  Root values are stored without diacritics
/// already, so root search works as-is.
pub async fn search_words(pool: &SqlitePool, query: &str, field: &str) -> Result<Vec<Word>> {
    match field {
        "lemma" => {
            // Strip diacritics from both the stored column (lemma_bare) and the query.
            let bare_query = strip_diacritics(query);
            let pattern = format!("%{}%", bare_query);
            sqlx::query_as::<_, Word>(
                "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma
                 FROM words WHERE lemma_bare LIKE ? ORDER BY surah, ayah, position",
            )
            .bind(pattern)
            .fetch_all(pool)
            .await
            .with_context(|| format!("search_words(lemma) failed: q={}", query))
        }
        _ => {
            // Root field: stored without diacritics. Strip query anyway so
            // callers can pass "رَحْم" and still match stored root "رحم".
            let bare_query = strip_diacritics(query);
            let pattern = format!("%{}%", bare_query);
            sqlx::query_as::<_, Word>(
                "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma
                 FROM words WHERE root LIKE ? ORDER BY surah, ayah, position",
            )
            .bind(pattern)
            .fetch_all(pool)
            .await
            .with_context(|| format!("search_words(root) failed: q={}", query))
        }
    }
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

// ─── Tadabbur layer — reads ──────────────────────────────────────────────────

/// Fetch all words in an ayah, each paired with morphological analysis and irab.
pub async fn get_ayah_words(pool: &SqlitePool, surah: i32, ayah: i32) -> Result<Vec<WordDetail>> {
    validate_ref(surah, ayah)?;
    let words = sqlx::query_as::<_, Word>(
        "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma
         FROM words WHERE surah = ? AND ayah = ? ORDER BY position",
    )
    .bind(surah)
    .bind(ayah)
    .fetch_all(pool)
    .await
    .with_context(|| format!("get_ayah_words failed for {}:{}", surah, ayah))?;

    let mut details = Vec::with_capacity(words.len());
    for word in words {
        let morphology = morphology_for(pool, word.id).await.ok();
        let irab = get_irab_for_word(pool, word.id).await.ok();
        details.push(WordDetail { word, morphology, irab });
    }
    Ok(details)
}

/// Collect deduplicated ontology entries for every root appearing in an ayah.
/// Roots with no ontology record are silently skipped.
pub async fn roots_for_ayah(pool: &SqlitePool, surah: i32, ayah: i32) -> Result<Vec<Ontology>> {
    validate_ref(surah, ayah)?;
    let roots: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT root FROM words
         WHERE surah = ? AND ayah = ? AND root IS NOT NULL
         ORDER BY root",
    )
    .bind(surah)
    .bind(ayah)
    .fetch_all(pool)
    .await
    .with_context(|| format!("roots_for_ayah failed for {}:{}", surah, ayah))?;

    let mut out = Vec::with_capacity(roots.len());
    for root in roots {
        if let Ok(onto) = get_ontology(pool, &root).await {
            out.push(onto);
        }
    }
    Ok(out)
}

/// Return reflections recorded for a given ayah, oldest first.
/// Pass `limit = None` to return all rows.
pub async fn reflections_for(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
    limit: Option<i64>,
) -> Result<Vec<Reflection>> {
    validate_ref(surah, ayah)?;
    sqlx::query_as::<_, Reflection>(
        "SELECT id, surah, ayah, body, author, source, lang, created_at
         FROM reflections WHERE surah = ? AND ayah = ? ORDER BY created_at LIMIT ?",
    )
    .bind(surah)
    .bind(ayah)
    .bind(limit.unwrap_or(i64::MAX))
    .fetch_all(pool)
    .await
    .with_context(|| format!("reflections_for {}:{}", surah, ayah))
}

/// Return all themes tagged on a given ayah, ordered alphabetically by English name.
pub async fn themes_for(pool: &SqlitePool, surah: i32, ayah: i32) -> Result<Vec<Theme>> {
    validate_ref(surah, ayah)?;
    sqlx::query_as::<_, Theme>(
        "SELECT t.id, t.name_ar, t.name_en, t.description
         FROM themes t
         JOIN ayah_themes at ON t.id = at.theme_id
         WHERE at.surah = ? AND at.ayah = ?
         ORDER BY t.name_en",
    )
    .bind(surah)
    .bind(ayah)
    .fetch_all(pool)
    .await
    .with_context(|| format!("themes_for {}:{}", surah, ayah))
}

/// Return all outgoing cross-references from a given ayah, ordered by target coordinate.
///
/// Each row includes a pre-formatted `reference` string (e.g. `"27:30"`) so
/// callers get a display-ready value without needing `r#ref`.
pub async fn cross_refs_for(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
) -> Result<Vec<QuranCrossRef>> {
    validate_ref(surah, ayah)?;
    sqlx::query_as::<_, QuranCrossRef>(
        "SELECT id, from_surah, from_ayah, to_surah, to_ayah,
                printf('%d:%d', to_surah, to_ayah) AS reference,
                relation, note
         FROM cross_references
         WHERE from_surah = ? AND from_ayah = ?
         ORDER BY to_surah, to_ayah",
    )
    .bind(surah)
    .bind(ayah)
    .fetch_all(pool)
    .await
    .with_context(|| format!("cross_refs_for {}:{}", surah, ayah))
}

/// Return all translations recorded for a given ayah, ordered by lang then insertion time.
pub async fn translations_for(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
) -> Result<Vec<AyahTranslation>> {
    validate_ref(surah, ayah)?;
    sqlx::query_as::<_, AyahTranslation>(
        "SELECT id, surah, ayah, text, translator, lang, source, created_at
         FROM translations WHERE surah = ? AND ayah = ? ORDER BY lang, created_at",
    )
    .bind(surah)
    .bind(ayah)
    .fetch_all(pool)
    .await
    .with_context(|| format!("translations_for {}:{}", surah, ayah))
}

/// Insert a translation for an ayah. Returns the new row's id.
pub async fn insert_translation(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
    text: &str,
    translator: Option<&str>,
    lang: &str,
    source: Option<&str>,
) -> Result<i64> {
    validate_ref(surah, ayah)?;
    sqlx::query(
        "INSERT INTO translations (surah, ayah, text, translator, lang, source)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(surah)
    .bind(ayah)
    .bind(text)
    .bind(translator)
    .bind(lang)
    .bind(source)
    .execute(pool)
    .await
    .map(|r| r.last_insert_rowid())
    .with_context(|| format!("insert_translation {}:{}", surah, ayah))
}

/// Assemble a complete `TadabburPage` for a single ayah.
///
/// All sub-queries run sequentially; a missing ayah (no words) returns an
/// empty page rather than an error so callers can distinguish "no data" from
/// a hard failure.
pub async fn tadabbur_page(pool: &SqlitePool, surah: i32, ayah: i32) -> Result<TadabburPage> {
    validate_ref(surah, ayah)?;
    let words = get_ayah_words(pool, surah, ayah).await?;
    let roots = roots_for_ayah(pool, surah, ayah).await?;
    let reflections = reflections_for(pool, surah, ayah, None).await?;
    let themes = themes_for(pool, surah, ayah).await?;
    let cross_refs = cross_refs_for(pool, surah, ayah).await?;
    let hadith_cross_refs = hadith_cross_refs_for(pool, surah, ayah).await?;
    let translations = translations_for(pool, surah, ayah).await?;
    let slug = format!("{}:{}", surah, ayah);
    Ok(TadabburPage { surah, ayah, slug, words, roots, reflections, themes, cross_refs, hadith_cross_refs, translations })
}

// ─── Tadabbur layer — writes ─────────────────────────────────────────────────

/// Insert a reflection for an ayah. Returns the new row's id.
pub async fn insert_reflection(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
    body: &str,
    author: Option<&str>,
    source: Option<&str>,
    lang: &str,
) -> Result<i64> {
    validate_ref(surah, ayah)?;
    sqlx::query(
        "INSERT INTO reflections (surah, ayah, body, author, source, lang) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(surah)
    .bind(ayah)
    .bind(body)
    .bind(author)
    .bind(source)
    .bind(lang)
    .execute(pool)
    .await
    .map(|r| r.last_insert_rowid())
    .with_context(|| format!("insert_reflection {}:{}", surah, ayah))
}

/// Insert a theme. Returns the new row's id.
/// Errors on duplicate `name_ar` or `name_en` (UNIQUE constraint).
pub async fn insert_theme(
    pool: &SqlitePool,
    name_ar: &str,
    name_en: &str,
    description: Option<&str>,
) -> Result<i64> {
    sqlx::query(
        "INSERT INTO themes (name_ar, name_en, description) VALUES (?, ?, ?)",
    )
    .bind(name_ar)
    .bind(name_en)
    .bind(description)
    .execute(pool)
    .await
    .map(|r| r.last_insert_rowid())
    .with_context(|| format!("insert_theme: {}", name_en))
}

/// Tag an ayah with a theme. No-op if the (surah, ayah, theme_id) triple already exists.
pub async fn tag_ayah_theme(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
    theme_id: i64,
    note: Option<&str>,
) -> Result<()> {
    validate_ref(surah, ayah)?;
    sqlx::query(
        "INSERT OR IGNORE INTO ayah_themes (surah, ayah, theme_id, note) VALUES (?, ?, ?, ?)",
    )
    .bind(surah)
    .bind(ayah)
    .bind(theme_id)
    .bind(note)
    .execute(pool)
    .await
    .with_context(|| format!("tag_ayah_theme {}:{} → theme {}", surah, ayah, theme_id))?;
    Ok(())
}

/// Fetch a single reflection by its primary key.
pub async fn get_reflection_by_id(pool: &SqlitePool, id: i64) -> Result<Reflection> {
    sqlx::query_as::<_, Reflection>(
        "SELECT id, surah, ayah, body, author, source, lang, created_at
         FROM reflections WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("reflection not found: id={}", id))
}

/// Update the body text of an existing reflection.
/// Returns an error if the reflection does not exist.
pub async fn update_reflection_body(pool: &SqlitePool, id: i64, body: &str) -> Result<()> {
    let rows = sqlx::query("UPDATE reflections SET body = ? WHERE id = ?")
        .bind(body)
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("update_reflection_body id={}", id))?
        .rows_affected();
    anyhow::ensure!(rows > 0, "reflection not found: id={}", id);
    Ok(())
}

/// Delete a reflection by id. Returns true if a row was deleted, false if not found.
pub async fn delete_reflection(pool: &SqlitePool, id: i64) -> Result<bool> {
    let rows = sqlx::query("DELETE FROM reflections WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("delete_reflection id={}", id))?
        .rows_affected();
    Ok(rows > 0)
}

/// Return all themes, ordered by English name.
pub async fn list_all_themes(pool: &SqlitePool) -> Result<Vec<Theme>> {
    sqlx::query_as::<_, Theme>(
        "SELECT id, name_ar, name_en, description FROM themes ORDER BY name_en",
    )
    .fetch_all(pool)
    .await
    .context("list_all_themes")
}

/// Fetch a single theme by its primary key.
pub async fn get_theme_by_id(pool: &SqlitePool, id: i64) -> Result<Theme> {
    sqlx::query_as::<_, Theme>(
        "SELECT id, name_ar, name_en, description FROM themes WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("theme not found: id={}", id))
}

/// Fetch a single cross-reference by its primary key.
pub async fn get_cross_ref_by_id(pool: &SqlitePool, id: i64) -> Result<QuranCrossRef> {
    sqlx::query_as::<_, QuranCrossRef>(
        "SELECT id, from_surah, from_ayah, to_surah, to_ayah,
                printf('%d:%d', to_surah, to_ayah) AS reference,
                relation, note
         FROM cross_references WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("cross_reference not found: id={}", id))
}

/// Return all hadith cross-references for a given ayah, ordered by collection then number.
pub async fn hadith_cross_refs_for(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
) -> Result<Vec<HadithCrossRef>> {
    validate_ref(surah, ayah)?;
    sqlx::query_as::<_, HadithCrossRef>(
        "SELECT id, surah, ayah, reference, collection, hadith_number,
                grade, grader, relation, note
         FROM hadith_cross_references
         WHERE surah = ? AND ayah = ?
         ORDER BY collection, hadith_number",
    )
    .bind(surah)
    .bind(ayah)
    .fetch_all(pool)
    .await
    .with_context(|| format!("hadith_cross_refs_for {}:{}", surah, ayah))
}

/// Insert a hadith cross-reference for an ayah.
///
/// Returns `Some(id)` on success, `None` if the
/// `(surah, ayah, collection, hadith_number)` tuple already exists.
pub async fn insert_hadith_cross_ref(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
    reference: &str,
    collection: &str,
    hadith_number: &str,
    grade: &str,
    grader: Option<&str>,
    relation: Option<&str>,
    note: Option<&str>,
) -> Result<Option<i64>> {
    validate_ref(surah, ayah)?;
    let result = sqlx::query(
        "INSERT OR IGNORE INTO hadith_cross_references
         (surah, ayah, reference, collection, hadith_number, grade, grader, relation, note)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(surah)
    .bind(ayah)
    .bind(reference)
    .bind(collection)
    .bind(hadith_number)
    .bind(grade)
    .bind(grader)
    .bind(relation)
    .bind(note)
    .execute(pool)
    .await
    .with_context(|| {
        format!(
            "insert_hadith_cross_ref {}:{} → {} {}",
            surah, ayah, collection, hadith_number
        )
    })?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    Ok(Some(result.last_insert_rowid()))
}

// ─── Irab layer ───────────────────────────────────────────────────────────────

/// Fetch the irab record for a single word.
///
/// Returns `Err` if no irab has been recorded for this word yet.
pub async fn get_irab_for_word(pool: &SqlitePool, word_id: i64) -> Result<WordIrab> {
    sqlx::query_as::<_, WordIrab>(
        "SELECT id, word_id, word_type, case_marker, case_sign,
                grammatical_function, subtype, note, source
         FROM word_irab WHERE word_id = ?",
    )
    .bind(word_id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("irab not found for word_id {}", word_id))
}

/// Fetch the irab record by its primary key.
pub async fn get_irab_by_id(pool: &SqlitePool, id: i64) -> Result<WordIrab> {
    sqlx::query_as::<_, WordIrab>(
        "SELECT id, word_id, word_type, case_marker, case_sign,
                grammatical_function, subtype, note, source
         FROM word_irab WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("irab not found: id={}", id))
}

/// Fetch all irab records for every word in an ayah, ordered by word position.
pub async fn irab_for_ayah(pool: &SqlitePool, surah: i32, ayah: i32) -> Result<Vec<WordIrab>> {
    validate_ref(surah, ayah)?;
    sqlx::query_as::<_, WordIrab>(
        "SELECT wi.id, wi.word_id, wi.word_type, wi.case_marker, wi.case_sign,
                wi.grammatical_function, wi.subtype, wi.note, wi.source
         FROM word_irab wi
         JOIN words w ON wi.word_id = w.id
         WHERE w.surah = ? AND w.ayah = ?
         ORDER BY w.position",
    )
    .bind(surah)
    .bind(ayah)
    .fetch_all(pool)
    .await
    .with_context(|| format!("irab_for_ayah {}:{}", surah, ayah))
}

/// Insert an irab record for a word.
///
/// `word_type` is required (`ism` | `fil` | `harf`).
/// All other fields are optional — pass `None` for fields not yet known.
///
/// Returns `Some(id)` on insert, `None` if an irab record already exists
/// for this `word_id` (UNIQUE constraint, INSERT OR IGNORE).
#[allow(clippy::too_many_arguments)]
pub async fn insert_irab(
    pool: &SqlitePool,
    word_id: i64,
    word_type: &str,
    case_marker: Option<&str>,
    case_sign: Option<&str>,
    grammatical_function: Option<&str>,
    subtype: Option<&str>,
    note: Option<&str>,
    source: &str,
) -> Result<Option<i64>> {
    if !VALID_IRAB_WORD_TYPES.contains(&word_type) {
        anyhow::bail!(
            "invalid word_type '{}': must be one of {}",
            word_type,
            VALID_IRAB_WORD_TYPES.join(", ")
        );
    }
    if let Some(cm) = case_marker {
        if !VALID_IRAB_CASE_MARKERS.contains(&cm) {
            anyhow::bail!(
                "invalid case_marker '{}': must be one of {}",
                cm,
                VALID_IRAB_CASE_MARKERS.join(", ")
            );
        }
    }
    if let Some(cs) = case_sign {
        if !VALID_IRAB_CASE_SIGNS.contains(&cs) {
            anyhow::bail!(
                "invalid case_sign '{}': must be one of {}",
                cs,
                VALID_IRAB_CASE_SIGNS.join(", ")
            );
        }
    }
    if let Some(gf) = grammatical_function {
        if !VALID_IRAB_FUNCTIONS.contains(&gf) {
            anyhow::bail!(
                "invalid grammatical_function '{}': must be one of {}",
                gf,
                VALID_IRAB_FUNCTIONS.join(", ")
            );
        }
    }

    let result = sqlx::query(
        "INSERT OR IGNORE INTO word_irab
         (word_id, word_type, case_marker, case_sign, grammatical_function, subtype, note, source)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(word_id)
    .bind(word_type)
    .bind(case_marker)
    .bind(case_sign)
    .bind(grammatical_function)
    .bind(subtype)
    .bind(note)
    .bind(source)
    .execute(pool)
    .await
    .with_context(|| format!("insert_irab for word_id {}", word_id))?;

    if result.rows_affected() == 0 {
        return Ok(None); // Already exists — UNIQUE on word_id.
    }
    Ok(Some(result.last_insert_rowid()))
}

/// Update an existing irab record by its id.
///
/// All fields may be updated; pass the current value to leave a field unchanged.
/// Returns `Err` if no row with `id` exists.
#[allow(clippy::too_many_arguments)]
pub async fn update_irab(
    pool: &SqlitePool,
    id: i64,
    word_type: &str,
    case_marker: Option<&str>,
    case_sign: Option<&str>,
    grammatical_function: Option<&str>,
    subtype: Option<&str>,
    note: Option<&str>,
    source: &str,
) -> Result<()> {
    if !VALID_IRAB_WORD_TYPES.contains(&word_type) {
        anyhow::bail!(
            "invalid word_type '{}': must be one of {}",
            word_type,
            VALID_IRAB_WORD_TYPES.join(", ")
        );
    }
    if let Some(cm) = case_marker {
        if !VALID_IRAB_CASE_MARKERS.contains(&cm) {
            anyhow::bail!(
                "invalid case_marker '{}': must be one of {}",
                cm,
                VALID_IRAB_CASE_MARKERS.join(", ")
            );
        }
    }
    if let Some(cs) = case_sign {
        if !VALID_IRAB_CASE_SIGNS.contains(&cs) {
            anyhow::bail!(
                "invalid case_sign '{}': must be one of {}",
                cs,
                VALID_IRAB_CASE_SIGNS.join(", ")
            );
        }
    }
    if let Some(gf) = grammatical_function {
        if !VALID_IRAB_FUNCTIONS.contains(&gf) {
            anyhow::bail!(
                "invalid grammatical_function '{}': must be one of {}",
                gf,
                VALID_IRAB_FUNCTIONS.join(", ")
            );
        }
    }

    let rows = sqlx::query(
        "UPDATE word_irab
         SET word_type = ?, case_marker = ?, case_sign = ?,
             grammatical_function = ?, subtype = ?, note = ?, source = ?
         WHERE id = ?",
    )
    .bind(word_type)
    .bind(case_marker)
    .bind(case_sign)
    .bind(grammatical_function)
    .bind(subtype)
    .bind(note)
    .bind(source)
    .bind(id)
    .execute(pool)
    .await
    .with_context(|| format!("update_irab id={}", id))?
    .rows_affected();

    anyhow::ensure!(rows > 0, "irab not found: id={}", id);
    Ok(())
}

/// Delete an irab record by its id.
///
/// Returns `true` if a row was deleted, `false` if not found.
pub async fn delete_irab(pool: &SqlitePool, id: i64) -> Result<bool> {
    let rows = sqlx::query("DELETE FROM word_irab WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("delete_irab id={}", id))?
        .rows_affected();
    Ok(rows > 0)
}

/// Insert a cross-reference between two ayahs.
/// Returns `Some(id)` on success, `None` if the (from, to, relation) triple
/// already exists (the INSERT OR IGNORE was silently skipped).
pub async fn insert_cross_ref(
    pool: &SqlitePool,
    from_surah: i32,
    from_ayah: i32,
    to_surah: i32,
    to_ayah: i32,
    relation: &str,
    note: Option<&str>,
) -> Result<Option<i64>> {
    validate_ref(from_surah, from_ayah)?;
    validate_ref(to_surah, to_ayah)?;
    let result = sqlx::query(
        "INSERT OR IGNORE INTO cross_references
         (from_surah, from_ayah, to_surah, to_ayah, relation, note)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(from_surah)
    .bind(from_ayah)
    .bind(to_surah)
    .bind(to_ayah)
    .bind(relation)
    .bind(note)
    .execute(pool)
    .await
    .with_context(|| {
        format!(
            "insert_cross_ref {}:{} → {}:{} ({})",
            from_surah, from_ayah, to_surah, to_ayah, relation
        )
    })?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    Ok(Some(result.last_insert_rowid()))
}

// ─── Structural queries (QAF-1.2) ────────────────────────────────────────────

/// Insert a juz record.  Idempotent — duplicate id is silently ignored.
pub async fn insert_juz(pool: &SqlitePool, id: i32, name_ar: &str) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO juz (id, name_ar) VALUES (?, ?)")
        .bind(id)
        .bind(name_ar)
        .execute(pool)
        .await
        .with_context(|| format!("insert_juz id={}", id))?;
    Ok(())
}

/// Fetch a juz by its number (1–30).
pub async fn get_juz(pool: &SqlitePool, id: i32) -> Result<Juz> {
    sqlx::query_as::<_, Juz>("SELECT id, name_ar FROM juz WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .with_context(|| format!("juz {} not found", id))
}

/// Insert a page record.  Idempotent — duplicate id is silently ignored.
pub async fn insert_page(pool: &SqlitePool, id: i32, juz_id: Option<i32>) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO pages (id, juz_id) VALUES (?, ?)")
        .bind(id)
        .bind(juz_id)
        .execute(pool)
        .await
        .with_context(|| format!("insert_page id={}", id))?;
    Ok(())
}

/// Fetch a page by its number (1–604).
pub async fn get_page(pool: &SqlitePool, id: i32) -> Result<MushafPage> {
    sqlx::query_as::<_, MushafPage>("SELECT id, juz_id FROM pages WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .with_context(|| format!("page {} not found", id))
}

/// Insert a surah record.  Idempotent — duplicate id is silently ignored.
///
/// `revelation_type` must be `"makki"` or `"madani"`.
pub async fn insert_surah(
    pool: &SqlitePool,
    id: i32,
    name_ar: &str,
    name_en: &str,
    name_en_meaning: &str,
    revelation_type: &str,
    ayah_count: i32,
) -> Result<()> {
    if !matches!(revelation_type, "makki" | "madani") {
        anyhow::bail!(
            "invalid revelation_type '{}': must be 'makki' or 'madani'",
            revelation_type
        );
    }
    sqlx::query(
        "INSERT OR IGNORE INTO surahs
             (id, name_ar, name_en, name_en_meaning, revelation_type, ayah_count)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(name_ar)
    .bind(name_en)
    .bind(name_en_meaning)
    .bind(revelation_type)
    .bind(ayah_count)
    .execute(pool)
    .await
    .with_context(|| format!("insert_surah id={}", id))?;
    Ok(())
}

/// Fetch a surah by its number (1–114).
pub async fn get_surah(pool: &SqlitePool, id: i32) -> Result<Surah> {
    sqlx::query_as::<_, Surah>(
        "SELECT id, name_ar, name_en, name_en_meaning, revelation_type, ayah_count
         FROM surahs WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("surah {} not found", id))
}

/// Insert an ayah record.  Idempotent — duplicate `(surah_id, ayah_number)` returns `None`.
///
/// Returns the row id on first insert, or `None` if the row already exists.
pub async fn insert_ayah(
    pool: &SqlitePool,
    surah_id: i32,
    ayah_number: i32,
    text_uthmani: Option<&str>,
    page_id: Option<i32>,
    juz_id: Option<i32>,
) -> Result<Option<i64>> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO ayahs
             (surah_id, ayah_number, text_uthmani, page_id, juz_id)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(surah_id)
    .bind(ayah_number)
    .bind(text_uthmani)
    .bind(page_id)
    .bind(juz_id)
    .execute(pool)
    .await
    .with_context(|| format!("insert_ayah {}:{}", surah_id, ayah_number))?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }
    Ok(Some(result.last_insert_rowid()))
}

/// Fetch a single ayah by its surah and ayah number.
pub async fn get_ayah(pool: &SqlitePool, surah_id: i32, ayah_number: i32) -> Result<Ayah> {
    sqlx::query_as::<_, Ayah>(
        "SELECT id, surah_id, ayah_number, text_uthmani, page_id, juz_id
         FROM ayahs WHERE surah_id = ? AND ayah_number = ?",
    )
    .bind(surah_id)
    .bind(ayah_number)
    .fetch_one(pool)
    .await
    .with_context(|| format!("ayah {}:{} not found", surah_id, ayah_number))
}

/// Fetch all ayahs for a surah, ordered by `ayah_number`.
pub async fn ayahs_for_surah(pool: &SqlitePool, surah_id: i32) -> Result<Vec<Ayah>> {
    sqlx::query_as::<_, Ayah>(
        "SELECT id, surah_id, ayah_number, text_uthmani, page_id, juz_id
         FROM ayahs WHERE surah_id = ? ORDER BY ayah_number",
    )
    .bind(surah_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("ayahs_for_surah {} failed", surah_id))
}

// ─── Token / Segment layer (QAF-2.1) ─────────────────────────────────────────

/// Format the canonical token reference string for a Quran word coordinate.
///
/// Format: `tok:SSS:AAA:PPP` (surah, ayah, position — each zero-padded to 3 digits).
///
/// Examples:
/// - `(1, 1, 1)`   → `"tok:001:001:001"`
/// - `(60, 12, 23)` → `"tok:060:012:023"`
pub fn token_ref(surah: i32, ayah: i32, position: i32) -> String {
    format!("tok:{:03}:{:03}:{:03}", surah, ayah, position)
}

/// Format the canonical segment reference string for a word segment.
///
/// Format: `seg:SSS:AAA:PPP:SS` (surah, ayah, position — 3 digits; segment index — 2 digits).
///
/// Examples:
/// - `(1, 1, 1, 1)`    → `"seg:001:001:001:01"`
/// - `(60, 12, 23, 2)` → `"seg:060:012:023:02"`
pub fn segment_ref(surah: i32, ayah: i32, position: i32, seg_idx: i32) -> String {
    format!("seg:{:03}:{:03}:{:03}:{:02}", surah, ayah, position, seg_idx)
}

/// Parse a `tok:SSS:AAA:PPP` string into its `(surah, ayah, position)` components.
///
/// Returns `Err` if the string does not match the expected format.
pub fn parse_token_ref(s: &str) -> Result<(i32, i32, i32)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 4 || parts[0] != "tok" {
        anyhow::bail!("invalid token_ref '{}': expected tok:SSS:AAA:PPP", s);
    }
    let surah: i32 = parts[1].parse().with_context(|| format!("invalid surah in token_ref '{}'", s))?;
    let ayah: i32  = parts[2].parse().with_context(|| format!("invalid ayah in token_ref '{}'", s))?;
    let pos: i32   = parts[3].parse().with_context(|| format!("invalid position in token_ref '{}'", s))?;
    Ok((surah, ayah, pos))
}

/// Fetch a single [`WordToken`] by its Quran coordinate.
///
/// The `token_ref` field is computed in the SELECT — no stored column required.
pub async fn get_token(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
    position: i32,
) -> Result<WordToken> {
    validate_ref(surah, ayah)?;
    sqlx::query_as::<_, WordToken>(
        "SELECT id, surah, ayah, position, arabic, transliteration, root, lemma,
                printf('tok:%03d:%03d:%03d', surah, ayah, position) AS token_ref
         FROM words
         WHERE surah = ? AND ayah = ? AND position = ?",
    )
    .bind(surah)
    .bind(ayah)
    .bind(position)
    .fetch_one(pool)
    .await
    .with_context(|| format!("token not found: tok:{:03}:{:03}:{:03}", surah, ayah, position))
}

/// Fetch a single [`WordToken`] by its canonical reference string.
///
/// The ref is parsed into `(surah, ayah, position)` and then delegated to
/// [`get_token`].  Returns `Err` for malformed refs.
pub async fn get_token_by_ref(pool: &SqlitePool, tok_ref: &str) -> Result<WordToken> {
    let (surah, ayah, position) = parse_token_ref(tok_ref)?;
    get_token(pool, surah, ayah, position).await
}

/// Return all segments for a token, ordered by `segment_index`.
pub async fn segments_for_token(pool: &SqlitePool, word_id: i64) -> Result<Vec<WordSegment>> {
    #[derive(sqlx::FromRow)]
    struct SegRow {
        id: i64,
        word_id: i64,
        segment_index: i32,
        arabic: String,
        pos: String,
        features: String,
        segment_ref: String,
    }

    let rows = sqlx::query_as::<_, SegRow>(
        "SELECT id, word_id, segment_index, arabic, pos, features, segment_ref
         FROM word_segments
         WHERE word_id = ?
         ORDER BY segment_index",
    )
    .bind(word_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("segments_for_token word_id={}", word_id))?;

    rows.into_iter()
        .map(|r| {
            let features: serde_json::Value = serde_json::from_str(&r.features)
                .unwrap_or(serde_json::Value::Object(Default::default()));
            Ok(WordSegment {
                id: r.id,
                word_id: r.word_id,
                segment_index: r.segment_index,
                arabic: r.arabic,
                pos: r.pos,
                features,
                segment_ref: r.segment_ref,
            })
        })
        .collect()
}

/// Insert a segment for a token.
///
/// `surah`, `ayah`, `position` are used to compute the `segment_ref` — they
/// must match the parent token's coordinates.
///
/// Returns `Some(id)` on success, `None` if the `(word_id, segment_index)` pair
/// already exists (INSERT OR IGNORE was silently skipped).
pub async fn insert_segment(
    pool: &SqlitePool,
    word_id: i64,
    segment_index: i32,
    arabic: &str,
    pos: &str,
    features: &serde_json::Value,
    surah: i32,
    ayah: i32,
    position: i32,
) -> Result<Option<i64>> {
    let seg_ref = segment_ref(surah, ayah, position, segment_index);
    let features_str = serde_json::to_string(features)
        .context("failed to serialise segment features")?;

    let result = sqlx::query(
        "INSERT OR IGNORE INTO word_segments
         (word_id, segment_index, arabic, pos, features, segment_ref)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(word_id)
    .bind(segment_index)
    .bind(arabic)
    .bind(pos)
    .bind(features_str)
    .bind(&seg_ref)
    .execute(pool)
    .await
    .with_context(|| {
        format!(
            "insert_segment word_id={} seg_idx={} ref={}",
            word_id, segment_index, seg_ref
        )
    })?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }
    Ok(Some(result.last_insert_rowid()))
}

// ─── Recitation / Tajweed queries ────────────────────────────────────────────

/// Return all entries in the recitation catalogue, ordered by id.
pub async fn list_recitations(pool: &SqlitePool) -> Result<Vec<Recitation>> {
    sqlx::query_as::<_, Recitation>(
        "SELECT id, name, rawi, qari, description FROM recitations ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .context("list_recitations")
}

/// Fetch the text and tajweed spans for a single ayah in a named recitation.
///
/// Returns `Some((recitation, text, source, spans, colors))` where:
/// - `recitation` — catalogue entry for this riwāyah.
/// - `text`       — ayah text in that recitation's orthography (with tashkeel).
/// - `source`     — provenance label (e.g. `"tanzil.net"`, `"quranic-corpus/seed"`).
/// - `spans`      — all tajweed annotations for this text, ordered by `start_index`.
/// - `colors`     — full `(rule, color_hex)` pairs for this recitation; the
///                  renderer pre-loads this once rather than repeating colours in
///                  every span.
///
/// Returns `None` when the recitation name is unknown **or** when no text has
/// been imported for that (surah, ayah, recitation) combination yet.
pub async fn recitation_ayah(
    pool: &SqlitePool,
    surah: i32,
    ayah: i32,
    recitation_name: &str,
) -> Result<Option<(Recitation, String, Option<String>, Vec<TajweedSpan>, Vec<(String, String)>)>> {
    // 1 — Catalogue lookup.
    let rec: Option<Recitation> = sqlx::query_as(
        "SELECT id, name, rawi, qari, description FROM recitations WHERE name = ?",
    )
    .bind(recitation_name)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("lookup recitation '{}'", recitation_name))?;

    let rec = match rec {
        Some(r) => r,
        None => return Ok(None),
    };

    // 2 — Text row for this (recitation, surah, ayah).
    let text_row: Option<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT id, text, source
         FROM recitation_texts
         WHERE recitation_id = ? AND surah_id = ? AND ayah_number = ?",
    )
    .bind(rec.id)
    .bind(surah)
    .bind(ayah)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("recitation_texts {}:{} ({})", surah, ayah, recitation_name))?;

    let (rt_id, text, source) = match text_row {
        Some(row) => row,
        None => return Ok(None),
    };

    // 3 — Tajweed spans, ordered for sequential rendering.
    let spans: Vec<TajweedSpan> = sqlx::query_as(
        "SELECT start_index, length, rule, note
         FROM tajweed_spans
         WHERE recitation_text_id = ?
         ORDER BY start_index",
    )
    .bind(rt_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("tajweed_spans rt_id={}", rt_id))?;

    // 4 — Full colour map: all (rule → color_hex) for this recitation.
    //     Loaded once; the renderer applies it without touching tajweed_spans again.
    let colors: Vec<(String, String)> = sqlx::query_as(
        "SELECT rule, color_hex
         FROM tajweed_rule_colors
         WHERE recitation_id = ?
         ORDER BY rule",
    )
    .bind(rec.id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("tajweed_rule_colors recitation '{}'", recitation_name))?;

    Ok(Some((rec, text, source, spans, colors)))
}
