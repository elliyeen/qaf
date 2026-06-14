/// Input schemas for each MCP tool.
/// Each struct must derive Deserialize + schemars::JsonSchema so rmcp can
/// generate the JSON Schema the AI host exposes to Claude.

use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetWordInput {
    #[schemars(description = "Surah number (1–114)")]
    pub surah: i32,
    #[schemars(description = "Ayah number within the surah")]
    pub ayah: i32,
    #[schemars(description = "Word position within the ayah (1-indexed)")]
    pub position: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchRootInput {
    #[schemars(description = "Arabic three-letter root, e.g. رحم or حمد")]
    pub root: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetMorphologyInput {
    #[schemars(description = "Internal word id from the words table")]
    pub word_id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetOntologyInput {
    #[schemars(description = "Arabic three-letter root to look up semantic domain and derivatives")]
    pub root: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetAyahWordsInput {
    #[schemars(description = "Surah number (1–114)")]
    pub surah: i32,
    #[schemars(description = "Ayah number within the surah")]
    pub ayah: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetTadabburPageInput {
    #[schemars(description = "Surah number (1–114)")]
    pub surah: i32,
    #[schemars(description = "Ayah number within the surah")]
    pub ayah: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetReflectionsInput {
    #[schemars(description = "Surah number (1–114)")]
    pub surah: i32,
    #[schemars(description = "Ayah number within the surah")]
    pub ayah: i32,
    #[schemars(description = "Maximum number of reflections to return (omit for all)")]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetCrossRefsInput {
    #[schemars(description = "Surah number (1–114)")]
    pub surah: i32,
    #[schemars(description = "Ayah number within the surah")]
    pub ayah: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchWordsInput {
    #[schemars(description = "Arabic text to search for (diacritic-insensitive)")]
    pub query: String,
    #[schemars(description = "Field to search: 'lemma' or 'root'")]
    pub field: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddReflectionInput {
    #[schemars(description = "Surah number (1–114)")]
    pub surah: i32,
    #[schemars(description = "Ayah number within the surah")]
    pub ayah: i32,
    #[schemars(description = "Reflection body text")]
    pub body: String,
    #[schemars(description = "Author name (optional)")]
    pub author: Option<String>,
    #[schemars(description = "Source / book title (optional)")]
    pub source: Option<String>,
    #[schemars(description = "ISO 639-1 language code, e.g. 'en' or 'ar'")]
    pub lang: String,
}

// ── Irab inputs ──────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetWordIrabInput {
    #[schemars(description = "Internal word id from the words table")]
    pub word_id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetAyahIrabInput {
    #[schemars(description = "Surah number (1–114)")]
    pub surah: i32,
    #[schemars(description = "Ayah number within the surah")]
    pub ayah: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddIrabInput {
    #[schemars(description = "Internal word id from the words table")]
    pub word_id: i64,
    #[schemars(description = "النوع — word type: ism (اسم) | fil (فعل) | harf (حرف)")]
    pub word_type: String,
    #[schemars(description = "الإعراب — case marker: marfu | mansub | majrur | majzum | mabni. Omit for pure particles.")]
    pub case_marker: Option<String>,
    #[schemars(description = "علامة الإعراب — case sign: damma | dammatan | fatha | fatahtan | kasra | kasratan | sukun | waw | alif | ya | nun_deletion | fatha_subst")]
    pub case_sign: Option<String>,
    #[schemars(description = "الوظيفة النحوية — syntactic function, e.g. mubtada | khabar | fail | naib_fail | mafuul_bihi | mudaf | mudaf_ilayh | nat | hal | tamyiz | badal | atf | tawkid | zarf | mafuul_mutlaq | isim_kana | khabar_kana | isim_inna | khabar_inna | jar | majrur_bijar | munada | fil | harf")]
    pub grammatical_function: Option<String>,
    #[schemars(description = "Optional sub-classification, e.g. fil_madhi | fil_mudari | fil_amr | ism_mawsul | damir_munfasil | damir_muttasil")]
    pub subtype: Option<String>,
    #[schemars(description = "Full Arabic irab phrase, e.g. 'مبتدأ مرفوع وعلامة رفعه الضمة الظاهرة على آخره'")]
    pub note: Option<String>,
    #[schemars(description = "Provenance: 'manual' (default) or 'quranic-corpus'")]
    pub source: Option<String>,
}
