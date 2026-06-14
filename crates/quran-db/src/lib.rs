pub mod db;
pub mod models;
pub mod queries;
pub mod text;

pub use db::{connect, run_migrations};
pub use models::{Ayah, AyahTranslation, CrossReference, HadithCrossRef, Juz, Morphology, MushafPage, Ontology, QuranCrossRef, Recitation, Reflection, Surah, TadabburPage, TajweedSpan, Theme, Word, WordDetail, WordIrab, WordSegment, WordToken};
pub use queries::{validate_ref, validate_surah, AYAH_COUNTS, VALID_IRAB_CASE_MARKERS, VALID_IRAB_CASE_SIGNS, VALID_IRAB_FUNCTIONS, VALID_IRAB_WORD_TYPES};
pub use queries::{list_recitations, recitation_ayah};
pub use queries::{ayahs_for_surah, get_ayah, get_juz, get_page, get_surah, insert_ayah, insert_juz, insert_page, insert_surah};
pub use queries::{get_token, get_token_by_ref, insert_segment, parse_token_ref, segment_ref, segments_for_token, token_ref};
pub use sqlx::SqlitePool;
pub use text::strip_diacritics;

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
            let lemma_bare = crate::text::strip_diacritics(&r.lemma);
            sqlx::query(
                "INSERT OR IGNORE INTO words (surah, ayah, position, arabic, transliteration, root, lemma, lemma_bare)
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
    async fn test_search_root_field() {
        let pool = test_pool().await;
        seed(&pool).await;
        let results = queries::search_words(&pool, "حمد", "root").await.expect("search by root");
        assert!(!results.is_empty(), "expected words with root حمد");
    }

    #[tokio::test]
    async fn test_search_lemma_bare_diacritic_insensitive() {
        let pool = test_pool().await;
        seed(&pool).await;

        // "رب" (no diacritics) must match lemma "رَبّ" (with fatha + shadda).
        // This was the original blocker — diacritics in stored lemmas broke LIKE.
        let rabb = queries::search_words(&pool, "رب", "lemma").await.expect("lemma search رب");
        assert!(
            !rabb.is_empty(),
            "bare 'رب' should match vowelized lemma 'رَبّ' via lemma_bare column"
        );
        assert!(rabb.iter().any(|w| w.lemma == "رَبّ"));

        // "الله" (with regular alif) must match "ٱللَّه" (alef wasla + shadda + kasra).
        let allah = queries::search_words(&pool, "الله", "lemma").await.expect("lemma search الله");
        assert!(
            !allah.is_empty(),
            "bare 'الله' should match wasla-form 'ٱللَّه' via lemma_bare normalization"
        );

        // "رحمان" must match "رَحْمَان".
        let rahman = queries::search_words(&pool, "رحمان", "lemma").await.expect("lemma search رحمان");
        assert!(!rahman.is_empty(), "bare 'رحمان' should match 'رَحْمَان'");
    }

    // ─── Tadabbur layer ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_tadabbur_page_empty_collections() {
        // Fresh ayah with no reflections/themes/xrefs yet — must return empty
        // vecs, not errors.
        let pool = test_pool().await;
        seed(&pool).await;
        let page = queries::tadabbur_page(&pool, 1, 1).await.expect("tadabbur page 1:1");
        assert_eq!(page.surah, 1);
        assert_eq!(page.ayah, 1);
        // Bismillah (1:1) has 4 words in the seed fixture.
        assert_eq!(page.words.len(), 4, "Bismillah should have 4 words");
        // Every word should carry morphology from the seed.
        assert!(
            page.words.iter().all(|wd| wd.morphology.is_some()),
            "all seeded words should have morphology"
        );
        // No reflections/themes/xrefs/translations seeded yet.
        assert!(page.reflections.is_empty());
        assert!(page.themes.is_empty());
        assert!(page.cross_refs.is_empty());
        assert!(page.translations.is_empty());
    }

    #[tokio::test]
    async fn test_tadabbur_page_slug() {
        let pool = test_pool().await;
        seed(&pool).await;
        let page = queries::tadabbur_page(&pool, 1, 1).await.expect("page 1:1");
        assert_eq!(page.slug, "1:1");
        let page2 = queries::tadabbur_page(&pool, 1, 2).await.expect("page 1:2");
        assert_eq!(page2.slug, "1:2");
        // Verify ayat al-kursi format — no data needed, just the slug computation.
        let page3 = queries::tadabbur_page(&pool, 2, 255).await.expect("page 2:255");
        assert_eq!(page3.slug, "2:255");
    }

    #[tokio::test]
    async fn test_tadabbur_page_roots() {
        // 1:1 has roots سمو and رحم — both have ontology in the seed.
        let pool = test_pool().await;
        seed(&pool).await;
        let page = queries::tadabbur_page(&pool, 1, 1).await.expect("page");
        // ٱللَّهِ (position 2) has root = null, so it is skipped.
        assert!(
            page.roots.iter().any(|o| o.root == "رحم"),
            "root رحم should appear"
        );
        assert!(
            page.roots.iter().any(|o| o.root == "سمو"),
            "root سمو should appear"
        );
    }

    #[tokio::test]
    async fn test_insert_and_query_reflection() {
        let pool = test_pool().await;
        seed(&pool).await;
        let id = queries::insert_reflection(
            &pool, 1, 1,
            "The Bismillah is the key to every surah.",
            Some("Ibn Kathīr"),
            Some("Tafsīr Ibn Kathīr, vol. 1"),
            "en",
        )
        .await
        .expect("insert_reflection");

        let refls = queries::reflections_for(&pool, 1, 1, None).await.expect("reflections_for");
        assert_eq!(refls.len(), 1);
        assert_eq!(refls[0].id, id);
        assert_eq!(refls[0].author.as_deref(), Some("Ibn Kathīr"));
        assert_eq!(refls[0].lang, "en");
    }

    #[tokio::test]
    async fn test_theme_lifecycle() {
        let pool = test_pool().await;
        seed(&pool).await;

        let tid = queries::insert_theme(
            &pool,
            "الرحمة",
            "Mercy",
            Some("Divine mercy as expressed across the Quran"),
        )
        .await
        .expect("insert_theme");

        queries::tag_ayah_theme(&pool, 1, 1, tid, Some("Bismillah names ar-Raḥmān and ar-Raḥīm"))
            .await
            .expect("tag_ayah_theme");

        // Idempotent: second call must not error.
        queries::tag_ayah_theme(&pool, 1, 1, tid, None)
            .await
            .expect("tag_ayah_theme idempotent");

        let themes = queries::themes_for(&pool, 1, 1).await.expect("themes_for");
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].name_en, "Mercy");
        assert_eq!(themes[0].name_ar, "الرحمة");
    }

    #[tokio::test]
    async fn test_cross_reference_lifecycle() {
        let pool = test_pool().await;
        seed(&pool).await;

        let xid = queries::insert_cross_ref(
            &pool, 1, 1, 27, 30, "repeats",
            Some("The Bismillah appears verbatim in Sūrat al-Naml 27:30"),
        )
        .await
        .expect("insert_cross_ref")
        .expect("xref should be new (not a duplicate)");

        let xrefs = queries::cross_refs_for(&pool, 1, 1).await.expect("cross_refs_for");
        assert_eq!(xrefs.len(), 1);
        assert_eq!(xrefs[0].id, xid);
        assert_eq!(xrefs[0].to_surah, 27);
        assert_eq!(xrefs[0].to_ayah, 30);
        assert_eq!(xrefs[0].relation, "repeats");
        // reference field should be pre-formatted
        assert_eq!(xrefs[0].reference, "27:30");
    }

    #[tokio::test]
    async fn test_quran_cross_ref_reference_field() {
        // Verify that the `reference` field is correctly computed for several
        // target coordinates so that callers never need r#ref.
        let pool = test_pool().await;
        seed(&pool).await;

        queries::insert_cross_ref(&pool, 1, 1, 2, 255, "explains", None)
            .await.unwrap().unwrap();
        queries::insert_cross_ref(&pool, 1, 1, 112, 1, "elaborates", None)
            .await.unwrap().unwrap();

        let xrefs = queries::cross_refs_for(&pool, 1, 1).await.expect("cross_refs_for");
        assert_eq!(xrefs.len(), 2);
        // Ordered by (to_surah, to_ayah): 2:255 first, then 112:1.
        assert_eq!(xrefs[0].reference, "2:255");
        assert_eq!(xrefs[1].reference, "112:1");
    }

    #[tokio::test]
    async fn test_hadith_cross_ref_lifecycle() {
        let pool = test_pool().await;
        seed(&pool).await;

        let hid = queries::insert_hadith_cross_ref(
            &pool,
            1, 1,
            "Ṣaḥīḥ al-Bukhārī 1",
            "Ṣaḥīḥ al-Bukhārī",
            "1",
            "ṣaḥīḥ",
            None,
            Some("explains"),
            Some("The hadith of 'Actions are by intentions' explains the opening of the Fātiḥah."),
        )
        .await
        .expect("insert_hadith_cross_ref")
        .expect("should be new");

        let hrefs = queries::hadith_cross_refs_for(&pool, 1, 1)
            .await
            .expect("hadith_cross_refs_for");
        assert_eq!(hrefs.len(), 1);
        assert_eq!(hrefs[0].id, hid);
        assert_eq!(hrefs[0].reference, "Ṣaḥīḥ al-Bukhārī 1");
        assert_eq!(hrefs[0].collection, "Ṣaḥīḥ al-Bukhārī");
        assert_eq!(hrefs[0].hadith_number, "1");
        assert_eq!(hrefs[0].grade, "ṣaḥīḥ");
        assert_eq!(hrefs[0].grader, None);
        assert_eq!(hrefs[0].relation.as_deref(), Some("explains"));
    }

    #[tokio::test]
    async fn test_hadith_cross_ref_duplicate_guard() {
        // INSERT OR IGNORE: inserting the same (surah, ayah, collection, hadith_number)
        // twice must return None the second time, not error.
        let pool = test_pool().await;
        seed(&pool).await;

        let first = queries::insert_hadith_cross_ref(
            &pool, 1, 1, "Muslim 261", "Ṣaḥīḥ Muslim", "261", "ṣaḥīḥ",
            None, None, None,
        )
        .await.expect("first insert").expect("new row");

        let second = queries::insert_hadith_cross_ref(
            &pool, 1, 1, "Muslim 261", "Ṣaḥīḥ Muslim", "261", "ṣaḥīḥ",
            None, None, None,
        )
        .await.expect("second insert (should not error)");

        assert!(second.is_none(), "duplicate must return None");

        // Only one row in the table.
        let hrefs = queries::hadith_cross_refs_for(&pool, 1, 1).await.unwrap();
        assert_eq!(hrefs.len(), 1);
        assert_eq!(hrefs[0].id, first);
    }

    #[tokio::test]
    async fn test_tadabbur_page_includes_hadith_refs() {
        let pool = test_pool().await;
        seed(&pool).await;

        // No hadith refs yet — field must be an empty vec, not missing.
        let page = queries::tadabbur_page(&pool, 1, 1).await.expect("page");
        assert!(page.hadith_cross_refs.is_empty());

        queries::insert_hadith_cross_ref(
            &pool, 1, 1,
            "Ṣaḥīḥ al-Bukhārī 1",
            "Ṣaḥīḥ al-Bukhārī", "1", "ṣaḥīḥ",
            Some("al-Bukhārī"), Some("explains"), None,
        )
        .await.unwrap().unwrap();

        let page2 = queries::tadabbur_page(&pool, 1, 1).await.expect("page with hadith");
        assert_eq!(page2.hadith_cross_refs.len(), 1);
        assert_eq!(page2.hadith_cross_refs[0].reference, "Ṣaḥīḥ al-Bukhārī 1");
        assert_eq!(page2.hadith_cross_refs[0].grader.as_deref(), Some("al-Bukhārī"));
    }

    #[tokio::test]
    async fn test_tadabbur_page_full() {
        // Integration: seed all tadabbur data then verify the composite page.
        let pool = test_pool().await;
        seed(&pool).await;

        queries::insert_reflection(&pool, 1, 1, "Opening with the name of Allah.", None, None, "en")
            .await
            .unwrap();
        let tid = queries::insert_theme(&pool, "التسمية", "Basmalah", None).await.unwrap();
        queries::tag_ayah_theme(&pool, 1, 1, tid, None).await.unwrap();
        queries::insert_cross_ref(&pool, 1, 1, 27, 30, "repeats", None).await.unwrap();

        let page = queries::tadabbur_page(&pool, 1, 1).await.expect("full page");
        assert_eq!(page.words.len(), 4);
        assert_eq!(page.reflections.len(), 1);
        assert_eq!(page.themes.len(), 1);
        assert_eq!(page.cross_refs.len(), 1);
        assert!(!page.roots.is_empty());
    }

    // ─── validate_ref ─────────────────────────────────────────────────────────

    #[test]
    fn test_validate_ref_valid_bounds() {
        // First and last ayahs of the Quran.
        assert!(queries::validate_ref(1, 1).is_ok());
        assert!(queries::validate_ref(1, 7).is_ok());
        assert!(queries::validate_ref(114, 6).is_ok());
        // Al-Baqara has 286 ayahs.
        assert!(queries::validate_ref(2, 286).is_ok());
        // An-Naml 27:30 — target of the Bismillah xref.
        assert!(queries::validate_ref(27, 30).is_ok());
    }

    #[test]
    fn test_validate_ref_surah_zero() {
        let err = queries::validate_ref(0, 1).unwrap_err();
        assert!(err.to_string().contains("invalid surah"));
    }

    #[test]
    fn test_validate_ref_surah_too_large() {
        let err = queries::validate_ref(115, 1).unwrap_err();
        assert!(err.to_string().contains("invalid surah"));
    }

    #[test]
    fn test_validate_ref_ayah_zero() {
        let err = queries::validate_ref(1, 0).unwrap_err();
        assert!(err.to_string().contains("invalid ayah"));
    }

    #[test]
    fn test_validate_ref_ayah_exceeds_surah_length() {
        // Sūrat al-Fātiḥah (1) has only 7 ayahs.
        let err = queries::validate_ref(1, 8).unwrap_err();
        assert!(err.to_string().contains("invalid ayah"));
        assert!(err.to_string().contains("1–7"));

        // Al-Baqara (2) has 286 ayahs.
        let err = queries::validate_ref(2, 287).unwrap_err();
        assert!(err.to_string().contains("invalid ayah"));
        assert!(err.to_string().contains("1–286"));
    }

    #[test]
    fn test_validate_ref_negative_inputs() {
        assert!(queries::validate_ref(-1, 1).is_err());
        assert!(queries::validate_ref(1, -1).is_err());
    }

    // ─── Translations ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_translation_lifecycle() {
        let pool = test_pool().await;
        seed(&pool).await;

        let id = queries::insert_translation(
            &pool,
            1, 1,
            "In the name of Allah, the Entirely Merciful, the Especially Merciful.",
            Some("Sahih International"),
            "en",
            None,
        )
        .await
        .expect("insert_translation");

        let txns = queries::translations_for(&pool, 1, 1)
            .await
            .expect("translations_for");
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].id, id);
        assert_eq!(txns[0].translator.as_deref(), Some("Sahih International"));
        assert_eq!(txns[0].lang, "en");
        assert!(txns[0].text.contains("Merciful"));
    }

    #[tokio::test]
    async fn test_translation_multiple_langs() {
        let pool = test_pool().await;
        seed(&pool).await;

        queries::insert_translation(
            &pool, 1, 1,
            "In the name of Allah, the Entirely Merciful, the Especially Merciful.",
            Some("Sahih International"), "en", None,
        )
        .await.unwrap();

        queries::insert_translation(
            &pool, 1, 1,
            "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ",
            Some("النص العثماني"), "ar", None,
        )
        .await.unwrap();

        let txns = queries::translations_for(&pool, 1, 1).await.unwrap();
        assert_eq!(txns.len(), 2);
        // Results ordered by lang: "ar" < "en"
        assert_eq!(txns[0].lang, "ar");
        assert_eq!(txns[1].lang, "en");
    }

    #[tokio::test]
    async fn test_tadabbur_page_with_translation() {
        let pool = test_pool().await;
        seed(&pool).await;

        queries::insert_translation(
            &pool, 1, 1,
            "In the name of Allah, the Entirely Merciful, the Especially Merciful.",
            Some("Sahih International"), "en", None,
        )
        .await.unwrap();

        let page = queries::tadabbur_page(&pool, 1, 1)
            .await
            .expect("tadabbur_page with translation");

        assert_eq!(page.translations.len(), 1);
        assert_eq!(page.translations[0].translator.as_deref(), Some("Sahih International"));
        // Other fields are unaffected.
        assert_eq!(page.words.len(), 4);
    }

    #[test]
    fn test_ayah_counts_length() {
        // Sanity check — the constant must cover all 114 surahs.
        assert_eq!(queries::AYAH_COUNTS.len(), 114);
        // Every surah has at least 3 ayahs and at most 286.
        assert!(queries::AYAH_COUNTS.iter().all(|&n| n >= 3 && n <= 286));
    }

    // ─── Irab layer ───────────────────────────────────────────────────────────

    /// Helper: return word_id of position 1 in 1:1 (بِسْمِ).
    async fn word_id_pos1(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT id FROM words WHERE surah=1 AND ayah=1 AND position=1")
            .fetch_one(pool)
            .await
            .expect("word_id pos1")
    }

    #[tokio::test]
    async fn test_irab_insert_and_fetch() {
        let pool = test_pool().await;
        seed(&pool).await;

        let wid = word_id_pos1(&pool).await;

        // بِسْمِ — harf jar construction (باء الجر + اسم بعده)
        // For the full word بِسْمِ the function is "mudaf" (مضاف) and case is majrur.
        let irab_id = queries::insert_irab(
            &pool,
            wid,
            "ism",
            Some("majrur"),
            Some("kasra"),
            Some("mudaf_ilayh"),
            None,
            Some("اسم مجرور بحرف الجر وعلامة جره الكسرة الظاهرة"),
            "manual",
        )
        .await
        .expect("insert_irab")
        .expect("should be a new row");

        let fetched = queries::get_irab_by_id(&pool, irab_id)
            .await
            .expect("get_irab_by_id");

        assert_eq!(fetched.word_id, wid);
        assert_eq!(fetched.word_type, "ism");
        assert_eq!(fetched.case_marker.as_deref(), Some("majrur"));
        assert_eq!(fetched.case_sign.as_deref(), Some("kasra"));
        assert_eq!(fetched.grammatical_function.as_deref(), Some("mudaf_ilayh"));
        assert_eq!(fetched.source, "manual");
        assert!(fetched.note.as_deref().unwrap().contains("مجرور"));
    }

    #[tokio::test]
    async fn test_irab_duplicate_guard() {
        // INSERT OR IGNORE: inserting twice for the same word_id returns None.
        let pool = test_pool().await;
        seed(&pool).await;

        let wid = word_id_pos1(&pool).await;

        let first = queries::insert_irab(
            &pool, wid, "ism", Some("majrur"), Some("kasra"),
            Some("mudaf_ilayh"), None, None, "manual",
        )
        .await.expect("first insert").expect("new row");

        let second = queries::insert_irab(
            &pool, wid, "harf", None, None, None, None, None, "manual",
        )
        .await.expect("second insert (must not error)");

        assert!(second.is_none(), "duplicate word_id must return None");

        // Only one row in the table.
        let all = queries::irab_for_ayah(&pool, 1, 1).await.unwrap();
        assert_eq!(all.iter().filter(|r| r.word_id == wid).count(), 1);
        // The stored row is still from the first insert.
        assert_eq!(all[0].id, first);
    }

    #[tokio::test]
    async fn test_irab_update() {
        let pool = test_pool().await;
        seed(&pool).await;

        let wid = word_id_pos1(&pool).await;

        let id = queries::insert_irab(
            &pool, wid, "ism", Some("marfu"), Some("damma"),
            Some("mubtada"), None, None, "manual",
        )
        .await.unwrap().unwrap();

        // Correct: بِسْمِ is not mubtada; update to majrur + mudaf_ilayh.
        queries::update_irab(
            &pool, id, "ism", Some("majrur"), Some("kasra"),
            Some("mudaf_ilayh"), None,
            Some("اسم مجرور بحرف الجر الباء"),
            "manual",
        )
        .await.expect("update_irab");

        let updated = queries::get_irab_by_id(&pool, id).await.unwrap();
        assert_eq!(updated.case_marker.as_deref(), Some("majrur"));
        assert_eq!(updated.grammatical_function.as_deref(), Some("mudaf_ilayh"));
        assert!(updated.note.unwrap().contains("الباء"));
    }

    #[tokio::test]
    async fn test_irab_delete() {
        let pool = test_pool().await;
        seed(&pool).await;

        let wid = word_id_pos1(&pool).await;
        let id = queries::insert_irab(
            &pool, wid, "harf", None, None, Some("harf"), None, None, "manual",
        )
        .await.unwrap().unwrap();

        assert!(queries::delete_irab(&pool, id).await.unwrap());
        // Second delete returns false, not an error.
        assert!(!queries::delete_irab(&pool, id).await.unwrap());
    }

    #[tokio::test]
    async fn test_irab_for_ayah() {
        // Insert irab for two words in 1:1 and verify irab_for_ayah returns both.
        let pool = test_pool().await;
        seed(&pool).await;

        // Word positions 1 and 2 in 1:1.
        let wid1: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM words WHERE surah=1 AND ayah=1 AND position=1",
        ).fetch_one(&pool).await.unwrap();
        let wid2: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM words WHERE surah=1 AND ayah=1 AND position=2",
        ).fetch_one(&pool).await.unwrap();

        queries::insert_irab(
            &pool, wid1, "ism", Some("majrur"), Some("kasra"),
            Some("mudaf_ilayh"), None,
            Some("اسم مجرور بالباء"), "manual",
        ).await.unwrap().unwrap();

        // ٱللَّهِ (position 2) — مضاف إليه مجرور
        queries::insert_irab(
            &pool, wid2, "ism", Some("majrur"), Some("kasra"),
            Some("mudaf_ilayh"), None,
            Some("لفظ الجلالة مضاف إليه مجرور"), "manual",
        ).await.unwrap().unwrap();

        let irab = queries::irab_for_ayah(&pool, 1, 1).await.unwrap();
        assert_eq!(irab.len(), 2);
        // Ordered by word position.
        assert_eq!(irab[0].word_id, wid1);
        assert_eq!(irab[1].word_id, wid2);
    }

    #[tokio::test]
    async fn test_get_ayah_words_includes_irab() {
        // After inserting irab, get_ayah_words must populate WordDetail.irab.
        let pool = test_pool().await;
        seed(&pool).await;

        let wid: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM words WHERE surah=1 AND ayah=1 AND position=1",
        ).fetch_one(&pool).await.unwrap();

        // Before irab insert: irab field must be None.
        let words_before = queries::get_ayah_words(&pool, 1, 1).await.unwrap();
        assert!(words_before[0].irab.is_none(), "no irab yet");

        queries::insert_irab(
            &pool, wid, "ism", Some("majrur"), Some("kasra"),
            Some("mudaf_ilayh"), None, None, "manual",
        ).await.unwrap().unwrap();

        let words_after = queries::get_ayah_words(&pool, 1, 1).await.unwrap();
        let irab = words_after[0].irab.as_ref().expect("irab should be present");
        assert_eq!(irab.word_type, "ism");
        assert_eq!(irab.case_marker.as_deref(), Some("majrur"));
    }

    #[tokio::test]
    async fn test_irab_invalid_word_type_rejected() {
        let pool = test_pool().await;
        seed(&pool).await;
        let wid = word_id_pos1(&pool).await;
        let err = queries::insert_irab(
            &pool, wid, "noun", None, None, None, None, None, "manual",
        ).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("invalid word_type"));
    }

    #[tokio::test]
    async fn test_irab_invalid_case_marker_rejected() {
        let pool = test_pool().await;
        seed(&pool).await;
        let wid = word_id_pos1(&pool).await;
        let err = queries::insert_irab(
            &pool, wid, "ism", Some("genitive"), None, None, None, None, "manual",
        ).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("invalid case_marker"));
    }

    // ─── Structural layer (QAF-1.2) ───────────────────────────────────────────

    /// Pool with SQLite foreign-key enforcement enabled.
    async fn test_pool_with_fk() -> SqlitePool {
        use sqlx::sqlite::SqliteConnectOptions;
        use std::str::FromStr;

        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .expect("valid options")
            .foreign_keys(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(opts)
            .await
            .expect("in-memory FK pool");
        run_migrations(&pool).await.expect("migrations");
        pool
    }

    #[tokio::test]
    async fn test_migration_creates_structural_tables() {
        // Migration success: all four tables must exist after running migrations.
        let pool = test_pool().await;
        for table in &["surahs", "juz", "pages", "ayahs"] {
            let count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                table
            ))
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|_| panic!("could not query sqlite_master for {}", table));
            assert_eq!(count, 1, "table '{}' must exist after migration", table);
        }
    }

    #[tokio::test]
    async fn test_structural_insert_and_query() {
        // Insert juz, page, surah, ayah; then query each back.
        let pool = test_pool().await;

        // Juz 1 — "الجزء الأول"
        queries::insert_juz(&pool, 1, "الجزء الأول").await.expect("insert_juz");
        let juz = queries::get_juz(&pool, 1).await.expect("get_juz");
        assert_eq!(juz.id, 1);
        assert_eq!(juz.name_ar, "الجزء الأول");

        // Page 1 belongs to juz 1.
        queries::insert_page(&pool, 1, Some(1)).await.expect("insert_page");
        let page = queries::get_page(&pool, 1).await.expect("get_page");
        assert_eq!(page.id, 1);
        assert_eq!(page.juz_id, Some(1));

        // Sūrat al-Fātiḥah (الفاتحة) · 1:1–7
        queries::insert_surah(&pool, 1, "الفاتحة", "Al-Fatiha", "The Opening", "makki", 7)
            .await
            .expect("insert_surah");
        let surah = queries::get_surah(&pool, 1).await.expect("get_surah");
        assert_eq!(surah.id, 1);
        assert_eq!(surah.name_ar, "الفاتحة");
        assert_eq!(surah.name_en, "Al-Fatiha");
        assert_eq!(surah.name_en_meaning, "The Opening");
        assert_eq!(surah.revelation_type, "makki");
        assert_eq!(surah.ayah_count, 7);

        // Ayah 1:1 — Bismillah — on page 1, juz 1.
        let ayah_id = queries::insert_ayah(
            &pool,
            1,
            1,
            Some("بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ"),
            Some(1),
            Some(1),
        )
        .await
        .expect("insert_ayah")
        .expect("new row");
        assert!(ayah_id > 0);

        let ayah = queries::get_ayah(&pool, 1, 1).await.expect("get_ayah");
        assert_eq!(ayah.surah_id, 1);
        assert_eq!(ayah.ayah_number, 1);
        assert_eq!(ayah.page_id, Some(1));
        assert_eq!(ayah.juz_id, Some(1));
        assert!(ayah.text_uthmani.as_deref().unwrap().contains("بِسْمِ"));
    }

    #[tokio::test]
    async fn test_ayahs_for_surah_ordered() {
        // Insert all 7 ayahs of al-Fatiha; ayahs_for_surah must return them in order.
        let pool = test_pool().await;
        queries::insert_surah(&pool, 1, "الفاتحة", "Al-Fatiha", "The Opening", "makki", 7)
            .await
            .unwrap();
        for n in 1..=7 {
            queries::insert_ayah(&pool, 1, n, None, None, None).await.unwrap();
        }
        let ayahs = queries::ayahs_for_surah(&pool, 1).await.expect("ayahs_for_surah");
        assert_eq!(ayahs.len(), 7);
        for (i, a) in ayahs.iter().enumerate() {
            assert_eq!(a.ayah_number, (i + 1) as i32);
        }
    }

    #[tokio::test]
    async fn test_ayah_duplicate_is_idempotent() {
        let pool = test_pool().await;
        queries::insert_surah(&pool, 2, "البقرة", "Al-Baqara", "The Cow", "madani", 286)
            .await
            .unwrap();
        let first = queries::insert_ayah(&pool, 2, 255, None, None, None).await.unwrap();
        let second = queries::insert_ayah(&pool, 2, 255, None, None, None).await.unwrap();
        assert!(first.is_some(), "first insert must return Some(id)");
        assert!(second.is_none(), "duplicate must return None");
    }

    #[tokio::test]
    async fn test_surah_invalid_revelation_type_rejected() {
        let pool = test_pool().await;
        let err = queries::insert_surah(
            &pool, 3, "آل عمران", "Al-Imran", "The Family of Imran", "egyptian", 200,
        )
        .await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("invalid revelation_type"));
    }

    #[tokio::test]
    async fn test_fk_enforcement_ayah_requires_surah() {
        // With FK enforcement enabled, inserting an ayah that references a
        // non-existent surah_id must fail.
        let pool = test_pool_with_fk().await;
        let result = queries::insert_ayah(&pool, 999, 1, None, None, None).await;
        assert!(
            result.is_err(),
            "FK violation: surah_id=999 does not exist and must be rejected"
        );
    }

    #[tokio::test]
    async fn test_structural_indexed_query_by_page() {
        // Query by page_id uses idx_ayahs_page — must return only matching rows.
        let pool = test_pool().await;
        queries::insert_surah(&pool, 1, "الفاتحة", "Al-Fatiha", "The Opening", "makki", 7)
            .await
            .unwrap();
        queries::insert_page(&pool, 1, None).await.unwrap();
        queries::insert_page(&pool, 2, None).await.unwrap();

        // Ayah 1:1 on page 1, ayah 1:2 on page 2.
        queries::insert_ayah(&pool, 1, 1, None, Some(1), None).await.unwrap();
        queries::insert_ayah(&pool, 1, 2, None, Some(2), None).await.unwrap();

        let on_page_1: Vec<Ayah> = sqlx::query_as::<_, Ayah>(
            "SELECT id, surah_id, ayah_number, text_uthmani, page_id, juz_id
             FROM ayahs WHERE page_id = ? ORDER BY ayah_number",
        )
        .bind(1i32)
        .fetch_all(&pool)
        .await
        .expect("indexed query by page_id");

        assert_eq!(on_page_1.len(), 1);
        assert_eq!(on_page_1[0].ayah_number, 1);
    }

    #[tokio::test]
    async fn test_irab_tadabbur_page_includes_irab() {
        // Verify TadabburPage.words carries irab after it is inserted.
        let pool = test_pool().await;
        seed(&pool).await;

        let wid: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM words WHERE surah=1 AND ayah=1 AND position=1",
        ).fetch_one(&pool).await.unwrap();

        queries::insert_irab(
            &pool, wid, "ism", Some("majrur"), Some("kasra"),
            Some("mudaf_ilayh"), None,
            Some("اسم مجرور بحرف الجر الباء وعلامة جره الكسرة"), "manual",
        ).await.unwrap().unwrap();

        let page = queries::tadabbur_page(&pool, 1, 1).await.unwrap();
        let first_word = &page.words[0];
        let irab = first_word.irab.as_ref().expect("irab in tadabbur_page");
        assert_eq!(irab.grammatical_function.as_deref(), Some("mudaf_ilayh"));
    }

    // ─── Token / Segment layer (QAF-2.1) ─────────────────────────────────────

    #[test]
    fn test_token_ref_format() {
        // Canonical format: tok:SSS:AAA:PPP — each coordinate zero-padded to 3 digits.
        assert_eq!(queries::token_ref(1, 1, 1),   "tok:001:001:001");
        assert_eq!(queries::token_ref(60, 12, 23), "tok:060:012:023");
        assert_eq!(queries::token_ref(114, 6, 1),  "tok:114:006:001");
    }

    #[test]
    fn test_segment_ref_format() {
        // Canonical format: seg:SSS:AAA:PPP:SS — segment index zero-padded to 2 digits.
        assert_eq!(queries::segment_ref(1, 1, 1, 1),    "seg:001:001:001:01");
        assert_eq!(queries::segment_ref(60, 12, 23, 2), "seg:060:012:023:02");
        assert_eq!(queries::segment_ref(2, 255, 1, 3),  "seg:002:255:001:03");
    }

    #[test]
    fn test_parse_token_ref_valid() {
        let (s, a, p) = queries::parse_token_ref("tok:001:001:001").unwrap();
        assert_eq!((s, a, p), (1, 1, 1));

        let (s, a, p) = queries::parse_token_ref("tok:060:012:023").unwrap();
        assert_eq!((s, a, p), (60, 12, 23));
    }

    #[test]
    fn test_parse_token_ref_invalid() {
        assert!(queries::parse_token_ref("seg:001:001:001").is_err(), "wrong prefix");
        assert!(queries::parse_token_ref("tok:001:001").is_err(), "too few parts");
        assert!(queries::parse_token_ref("tok:abc:001:001").is_err(), "non-numeric surah");
    }

    #[tokio::test]
    async fn test_get_token_by_coord() {
        // get_token returns a WordToken with the correct token_ref.
        let pool = test_pool().await;
        seed(&pool).await;

        let tok = queries::get_token(&pool, 1, 1, 1).await.expect("get_token 1:1:1");
        assert_eq!(tok.surah, 1);
        assert_eq!(tok.ayah, 1);
        assert_eq!(tok.position, 1);
        assert_eq!(tok.arabic, "بِسْمِ");
        assert_eq!(tok.token_ref, "tok:001:001:001");
    }

    #[tokio::test]
    async fn test_get_token_by_ref() {
        // get_token_by_ref parses the ref and delegates to get_token.
        let pool = test_pool().await;
        seed(&pool).await;

        let tok = queries::get_token_by_ref(&pool, "tok:001:001:001")
            .await
            .expect("get_token_by_ref tok:001:001:001");

        assert_eq!(tok.surah, 1);
        assert_eq!(tok.ayah, 1);
        assert_eq!(tok.position, 1);
        assert_eq!(tok.token_ref, "tok:001:001:001");
    }

    #[tokio::test]
    async fn test_token_links_to_ayah() {
        // Token carries surah + ayah coordinates, establishing the link.
        let pool = test_pool().await;
        seed(&pool).await;

        // Bismillah has 4 words (positions 1–4).
        for pos in 1..=4 {
            let tok = queries::get_token(&pool, 1, 1, pos)
                .await
                .unwrap_or_else(|_| panic!("token 1:1:{} not found", pos));
            assert_eq!(tok.surah, 1, "surah must be 1");
            assert_eq!(tok.ayah, 1,  "ayah must be 1");
            assert_eq!(tok.position, pos);
            // token_ref must encode all three coordinates.
            assert_eq!(
                tok.token_ref,
                format!("tok:{:03}:{:03}:{:03}", 1, 1, pos)
            );
        }
    }

    #[tokio::test]
    async fn test_segment_lifecycle() {
        // Insert two segments for a token, then fetch them back.
        let pool = test_pool().await;
        seed(&pool).await;

        // word id for بِسْمِ (1:1:1)
        let word_id: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM words WHERE surah=1 AND ayah=1 AND position=1",
        )
        .fetch_one(&pool)
        .await
        .expect("word_id 1:1:1");

        // بِسْمِ breaks into: "بِ" (PREP, seg 1) + "اسْم" (N, seg 2)
        let features_prep = serde_json::json!({});
        let features_noun = serde_json::json!({"case": "gen", "number": "sg"});

        let id1 = queries::insert_segment(
            &pool, word_id, 1, "بِ", "PREP", &features_prep, 1, 1, 1,
        )
        .await
        .expect("insert_segment 1")
        .expect("new segment");

        let id2 = queries::insert_segment(
            &pool, word_id, 2, "اسْم", "N", &features_noun, 1, 1, 1,
        )
        .await
        .expect("insert_segment 2")
        .expect("new segment");

        assert!(id1 > 0);
        assert!(id2 > id1, "second id must be greater");

        // Fetch back.
        let segs = queries::segments_for_token(&pool, word_id)
            .await
            .expect("segments_for_token");

        assert_eq!(segs.len(), 2);
        // Ordered by segment_index.
        assert_eq!(segs[0].segment_index, 1);
        assert_eq!(segs[0].arabic, "بِ");
        assert_eq!(segs[0].pos, "PREP");
        assert_eq!(segs[0].segment_ref, "seg:001:001:001:01");
        assert_eq!(segs[0].word_id, word_id);

        assert_eq!(segs[1].segment_index, 2);
        assert_eq!(segs[1].arabic, "اسْم");
        assert_eq!(segs[1].pos, "N");
        assert_eq!(segs[1].segment_ref, "seg:001:001:001:02");
        assert_eq!(segs[1].features["case"], "gen");
    }

    #[tokio::test]
    async fn test_segment_links_to_token() {
        // Segment.word_id must match the parent token's id.
        let pool = test_pool().await;
        seed(&pool).await;

        let word_id: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM words WHERE surah=1 AND ayah=1 AND position=2",
        )
        .fetch_one(&pool)
        .await
        .expect("word_id 1:1:2");

        queries::insert_segment(
            &pool, word_id, 1, "ٱللَّهِ", "PN",
            &serde_json::json!({"case": "gen"}), 1, 1, 2,
        )
        .await
        .expect("insert_segment")
        .expect("new");

        let segs = queries::segments_for_token(&pool, word_id).await.unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].word_id, word_id, "segment.word_id must equal token's id");
        assert_eq!(segs[0].segment_ref, "seg:001:001:002:01");
    }

    #[tokio::test]
    async fn test_segment_duplicate_guard() {
        // INSERT OR IGNORE: inserting the same (word_id, segment_index) twice returns None.
        let pool = test_pool().await;
        seed(&pool).await;

        let word_id: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM words WHERE surah=1 AND ayah=1 AND position=1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let first = queries::insert_segment(
            &pool, word_id, 1, "بِ", "PREP", &serde_json::json!({}), 1, 1, 1,
        )
        .await.unwrap().expect("first insert");

        let second = queries::insert_segment(
            &pool, word_id, 1, "بِ", "PREP", &serde_json::json!({}), 1, 1, 1,
        )
        .await.unwrap();

        assert!(second.is_none(), "duplicate (word_id, segment_index) must return None");

        let segs = queries::segments_for_token(&pool, word_id).await.unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].id, first);
    }

    #[tokio::test]
    async fn test_migration_creates_word_segments_table() {
        let pool = test_pool().await;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='word_segments'",
        )
        .fetch_one(&pool)
        .await
        .expect("sqlite_master query");
        assert_eq!(count, 1, "word_segments table must exist after migration");
    }
}
