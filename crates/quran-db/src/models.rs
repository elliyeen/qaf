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

// ─── Tadabbur layer ──────────────────────────────────────────────────────────

/// A textual reflection on a single ayah (from the `reflections` table).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reflection {
    pub id: i64,
    pub surah: i32,
    pub ayah: i32,
    /// Plain text or Markdown body.
    pub body: String,
    /// e.g. "Ibn Kathīr", "al-Ṭabarī"
    pub author: Option<String>,
    /// Book title, hadith ref, etc.
    pub source: Option<String>,
    /// ISO 639-1 language code, default "en".
    pub lang: String,
    pub created_at: String,
}

/// A named subject-matter category (from the `themes` table).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Theme {
    pub id: i64,
    /// Arabic label, e.g. "التوحيد"
    pub name_ar: String,
    /// English label, e.g. "Divine Oneness"
    pub name_en: String,
    pub description: Option<String>,
}

/// A human-language translation of a single ayah (from the `translations` table).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AyahTranslation {
    pub id: i64,
    pub surah: i32,
    pub ayah: i32,
    /// Full translation text of the ayah.
    pub text: String,
    /// e.g. "Sahih International", "Yusuf Ali"
    pub translator: Option<String>,
    /// ISO 639-1 language code, default "en".
    pub lang: String,
    /// Book title or URL.
    pub source: Option<String>,
    pub created_at: String,
}

/// A directed semantic link between two ayahs (from the `cross_references` table).
///
/// `relation` is one of: `elaborates | contrasts | repeats | explains | fulfills`
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CrossReference {
    pub id: i64,
    pub from_surah: i32,
    pub from_ayah: i32,
    pub to_surah: i32,
    pub to_ayah: i32,
    pub relation: String,
    pub note: Option<String>,
}

/// A directed Quran-to-Quran semantic link, enriched with a human-readable
/// `reference` string (e.g. `"27:30"`).  This is the display-ready variant of
/// [`CrossReference`]; `reference` is computed by the query layer so callers
/// never need to format the coordinate themselves — and never need `r#ref`.
///
/// `relation` is one of: `elaborates | contrasts | repeats | explains | fulfills`
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct QuranCrossRef {
    pub id: i64,
    pub from_surah: i32,
    pub from_ayah: i32,
    pub to_surah: i32,
    pub to_ayah: i32,
    /// Human-readable target coordinate, e.g. `"27:30"`.
    /// Field is named `reference` (not `ref`) to avoid the Rust keyword.
    pub reference: String,
    pub relation: String,
    pub note: Option<String>,
}

/// A link from a Quran ayah to a hadith that explains, corroborates, or
/// contextualises it (from the `hadith_cross_references` table).
///
/// All three citation fields — `collection`, `hadith_number`, and `grade` —
/// are required, in line with the project's itqān citation standard.
/// `reference` is a pre-formatted human-readable string (e.g.
/// `"Ṣaḥīḥ al-Bukhārī 3"`) so callers never need `r#ref`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HadithCrossRef {
    pub id: i64,
    pub surah: i32,
    pub ayah: i32,
    /// Human-readable citation string, e.g. `"Ṣaḥīḥ al-Bukhārī 3"`.
    /// Named `reference` (not `ref`) to avoid the Rust keyword.
    pub reference: String,
    /// Full collection name, e.g. `"Ṣaḥīḥ al-Bukhārī"`, `"Ṣaḥīḥ Muslim"`.
    pub collection: String,
    /// Number within the collection; `String` because some editions use
    /// suffixes such as `"3432a"`.
    pub hadith_number: String,
    /// Authenticity grade: `ṣaḥīḥ | ḥasan | ḍa'īf | mawḍū'`.
    pub grade: String,
    /// Scholar who assigned the grade, e.g. `"al-Albānī"`, `"Ibn Ḥajar"`.
    pub grader: Option<String>,
    /// How the hadith relates to the ayah:
    /// `explains | corroborates | restricts | abrogates | contextualises`
    pub relation: Option<String>,
    /// Optional scholarly note on the connection.
    pub note: Option<String>,
}

// ─── Token / Segment layer ───────────────────────────────────────────────────

/// A Quranic word token — one orthographic word at a fixed position in an ayah.
///
/// This is the token-level view of the `words` table.  All coordinate fields
/// mirror [`Word`].  The `token_ref` is the canonical identifier string in the
/// format `tok:SSS:AAA:PPP` (zero-padded three digits each), e.g.
/// `"tok:001:001:001"` for Sūrat al-Fātiḥah 1:1 word 1.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WordToken {
    pub id: i64,
    pub surah: i32,
    pub ayah: i32,
    pub position: i32,
    pub arabic: String,
    pub transliteration: String,
    pub root: Option<String>,
    pub lemma: String,
    /// Canonical token reference, e.g. `"tok:001:001:001"`.
    pub token_ref: String,
}

/// A morphological segment within a [`WordToken`] — a prefix, stem, or suffix.
///
/// Arabic words are composed of multiple bound morphemes.  Each segment has
/// its own part-of-speech tag and grammatical features.
///
/// `segment_ref` follows the format `seg:SSS:AAA:PPP:SS` where the last two
/// digits are the 1-based segment index, e.g. `"seg:001:001:001:01"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordSegment {
    pub id: i64,
    pub word_id: i64,
    /// 1-based ordinal within the token (prefix = 1, stem = 2, suffix = 3…).
    pub segment_index: i32,
    /// Arabic text of this segment.
    pub arabic: String,
    /// Part-of-speech tag, e.g. `"PREP"`, `"DET"`, `"N"`, `"V"`, `"PN"`.
    pub pos: String,
    /// JSON object of morphological features,
    /// e.g. `{"case":"gen","number":"sg","gender":"m"}`.
    pub features: serde_json::Value,
    /// Canonical segment reference, e.g. `"seg:001:001:001:01"`.
    pub segment_ref: String,
}

/// Per-word إعراب (grammatical analysis) at Ajurrumiyyah level.
///
/// All text fields use controlled vocabulary — see the `VALID_IRAB_*`
/// constants in `queries.rs` for the exhaustive lists.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WordIrab {
    pub id: i64,
    pub word_id: i64,
    /// النوع: `ism` | `fil` | `harf`
    pub word_type: String,
    /// الإعراب: `marfu` | `mansub` | `majrur` | `majzum` | `mabni`
    ///
    /// `None` for pure particles (حرف) that carry no case
    /// (مبني لا محل له من الإعراب).
    pub case_marker: Option<String>,
    /// علامة الإعراب:
    /// `damma` | `dammatan` | `fatha` | `fatahtan`
    /// | `kasra` | `kasratan` | `sukun` | `waw` | `alif` | `ya`
    /// | `nun_deletion` | `fatha_subst`
    pub case_sign: Option<String>,
    /// Syntactic function (الوظيفة النحوية), e.g.
    /// `mubtada`, `khabar`, `fail`, `mafuul_bihi`, `mudaf_ilayh`.
    pub grammatical_function: Option<String>,
    /// Optional sub-classification, e.g. `fil_madhi`, `fil_mudari`,
    /// `fil_amr`, `ism_mawsul`, `damir_munfasil`.
    pub subtype: Option<String>,
    /// Full irab phrase in Arabic, e.g.
    /// `"مبتدأ مرفوع وعلامة رفعه الضمة الظاهرة على آخره"`.
    pub note: Option<String>,
    /// Provenance: `"manual"` or `"quranic-corpus"`.
    pub source: String,
}

/// A word enriched with its morphological analysis and irab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordDetail {
    #[serde(flatten)]
    pub word: Word,
    /// `None` if morphology is not yet recorded for this word.
    pub morphology: Option<Morphology>,
    /// `None` if no irab has been recorded for this word yet.
    pub irab: Option<WordIrab>,
}

// ─── Structural layer ─────────────────────────────────────────────────────────

/// One of the 30 fixed divisions of the Quran (from the `juz` table).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Juz {
    /// Juz number, 1–30.
    pub id: i32,
    /// Arabic label, e.g. "الجزء الأول".
    pub name_ar: String,
}

/// A page in the standard Uthmani muṣḥaf (from the `pages` table).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MushafPage {
    /// Page number, 1–604.
    pub id: i32,
    /// Juz this page belongs to (populated by importer).
    pub juz_id: Option<i32>,
}

/// One of the 114 surahs of the Quran (from the `surahs` table).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Surah {
    /// Surah number, 1–114.
    pub id: i32,
    /// Arabic name, e.g. "الفاتحة".
    pub name_ar: String,
    /// Transliterated name, e.g. "Al-Fatiha".
    pub name_en: String,
    /// English meaning, e.g. "The Opening".
    pub name_en_meaning: String,
    /// Revelation context: `"makki"` or `"madani"`.
    pub revelation_type: String,
    /// Total ayah count in the standard muṣḥaf.
    pub ayah_count: i32,
}

/// A single verse (from the `ayahs` table).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Ayah {
    pub id: i64,
    /// FK → surahs.id
    pub surah_id: i32,
    pub ayah_number: i32,
    /// Uthmani Arabic text (nullable until imported).
    pub text_uthmani: Option<String>,
    /// FK → pages.id (nullable until imported).
    pub page_id: Option<i32>,
    /// FK → juz.id (nullable until imported).
    pub juz_id: Option<i32>,
}

/// Full contemplation context for a single ayah — the shape returned by
// ─── Recitation / Tajweed layer ──────────────────────────────────────────────

/// A named Quranic recitation variant (riwāyah / qirāʾah).
/// Stored in the `recitations` catalogue table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Recitation {
    pub id: i64,
    /// Stable ASCII slug, e.g. `"hafs"`, `"khalaf"`.
    pub name: String,
    /// Full name of the transmitter (rāwī), in Arabic.
    pub rawi: String,
    /// Full name of the reciter (qāriʾ), in Arabic.
    pub qari: String,
    /// Optional scholarly notes or source reference.
    pub description: Option<String>,
}

/// A single tajweed rule annotation on one character range of a recitation text.
/// `start_index` and `length` are 0-based Unicode character (codepoint) offsets
/// into `recitation_texts.text` — not byte offsets.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TajweedSpan {
    pub start_index: i64,
    pub length: i64,
    pub rule: String,
    pub note: Option<String>,
}

/// `queries::tadabbur_page`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TadabburPage {
    pub surah: i32,
    pub ayah: i32,
    /// Human-readable reference in `"surah:ayah"` form, e.g. `"2:255"`.
    /// Useful for logging, UI display, and MCP tool output.
    pub slug: String,
    /// Words of the ayah in order, each with its morphological analysis.
    pub words: Vec<WordDetail>,
    /// Deduplicated ontology entries for every root that appears in this ayah.
    pub roots: Vec<Ontology>,
    /// Scholarly reflections recorded for this ayah.
    pub reflections: Vec<Reflection>,
    /// Subject-matter themes tagged on this ayah.
    pub themes: Vec<Theme>,
    /// Outgoing semantic links to related ayahs (Quran→Quran), with a
    /// pre-formatted `reference` field (e.g. `"27:30"`).
    pub cross_refs: Vec<QuranCrossRef>,
    /// Links to ahadith that explain, corroborate, or contextualise this ayah.
    pub hadith_cross_refs: Vec<HadithCrossRef>,
    /// Human-language translations of this ayah.
    pub translations: Vec<AyahTranslation>,
}
