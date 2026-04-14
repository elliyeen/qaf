use crate::schema::{GetMorphologyInput, GetOntologyInput, GetWordInput, SearchRootInput};
use quran_db::{queries, SqlitePool};
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    tool, tool_router, ErrorData as McpError,
};

/// The Quranic word-level MCP server.
/// All tools call quran_db::queries directly — no HTTP hop.
#[derive(Clone)]
pub struct QuranServer {
    pub pool: SqlitePool,
}

#[tool_router(server_handler)]
impl QuranServer {
    /// Fetch a single Quranic word with full morphological data.
    #[tool(description = "Fetch a single Quranic word with full morphological data")]
    async fn get_word(
        &self,
        Parameters(GetWordInput { surah, ayah, position }): Parameters<GetWordInput>,
    ) -> Result<CallToolResult, McpError> {
        let word = queries::get_word(&self.pool, surah, ayah, position)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&word)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Find all words in the Quran sharing a given Arabic root.
    #[tool(description = "Find all words in the Quran sharing a given Arabic root")]
    async fn search_root(
        &self,
        Parameters(SearchRootInput { root }): Parameters<SearchRootInput>,
    ) -> Result<CallToolResult, McpError> {
        let words = queries::search_root(&self.pool, &root)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&words)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get part-of-speech and grammatical features for a word.
    #[tool(description = "Get part-of-speech and grammatical features for a word")]
    async fn get_morphology(
        &self,
        Parameters(GetMorphologyInput { word_id }): Parameters<GetMorphologyInput>,
    ) -> Result<CallToolResult, McpError> {
        let morph = queries::morphology_for(&self.pool, word_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&morph)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get semantic domain, derivatives, and scholar notes for a root.
    #[tool(description = "Get semantic domain, derivatives, and scholar notes for a root")]
    async fn get_ontology(
        &self,
        Parameters(GetOntologyInput { root }): Parameters<GetOntologyInput>,
    ) -> Result<CallToolResult, McpError> {
        let onto = queries::get_ontology(&self.pool, &root)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&onto)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}
