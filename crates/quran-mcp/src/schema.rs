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
