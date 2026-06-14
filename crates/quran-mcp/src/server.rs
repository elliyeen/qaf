use crate::schema::{
    AddReflectionInput, GetAyahWordsInput, GetCrossRefsInput, GetMorphologyInput, GetOntologyInput,
    GetReflectionsInput, GetTadabburPageInput, GetWordInput, SearchRootInput, SearchWordsInput,
};
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

    /// Return every word in an ayah paired with its morphological analysis.
    #[tool(description = "Return every word in an ayah paired with its morphological analysis")]
    async fn get_ayah_words(
        &self,
        Parameters(GetAyahWordsInput { surah, ayah }): Parameters<GetAyahWordsInput>,
    ) -> Result<CallToolResult, McpError> {
        let words = queries::get_ayah_words(&self.pool, surah, ayah)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&words)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Assemble the full tadabbur (contemplation) page for an ayah: words, roots, reflections, themes, and cross-references.
    #[tool(description = "Assemble the full tadabbur (contemplation) page for an ayah: words with morphology, root ontologies, scholarly reflections, themes, and cross-references to related ayahs")]
    async fn get_tadabbur_page(
        &self,
        Parameters(GetTadabburPageInput { surah, ayah }): Parameters<GetTadabburPageInput>,
    ) -> Result<CallToolResult, McpError> {
        let page = queries::tadabbur_page(&self.pool, surah, ayah)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&page)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Return scholarly reflections recorded for a given ayah, oldest first.
    #[tool(description = "Return scholarly reflections recorded for a given ayah, oldest first")]
    async fn get_reflections(
        &self,
        Parameters(GetReflectionsInput { surah, ayah, limit }): Parameters<GetReflectionsInput>,
    ) -> Result<CallToolResult, McpError> {
        let refs = queries::reflections_for(&self.pool, surah, ayah, limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&refs)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Return all outgoing semantic cross-references from a given ayah to related ayahs.
    #[tool(description = "Return all outgoing semantic cross-references from a given ayah to related ayahs, including relation type (elaborates, contrasts, repeats, explains, fulfills)")]
    async fn get_cross_refs(
        &self,
        Parameters(GetCrossRefsInput { surah, ayah }): Parameters<GetCrossRefsInput>,
    ) -> Result<CallToolResult, McpError> {
        let refs = queries::cross_refs_for(&self.pool, surah, ayah)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&refs)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Search words by lemma or root substring (diacritic-insensitive).
    #[tool(description = "Search words by lemma or root substring. Pass field='lemma' for dictionary form search or field='root' for trilateral root search. Both are diacritic-insensitive so you may pass harakat-stripped or fully vowelled Arabic.")]
    async fn search_words(
        &self,
        Parameters(SearchWordsInput { query, field }): Parameters<SearchWordsInput>,
    ) -> Result<CallToolResult, McpError> {
        let words = queries::search_words(&self.pool, &query, &field)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&words)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Insert a new reflection for an ayah and return its id.
    #[tool(description = "Insert a new scholarly or personal reflection for an ayah. Returns the new reflection id. Required: surah, ayah, body, lang (ISO 639-1). Optional: author, source.")]
    async fn add_reflection(
        &self,
        Parameters(AddReflectionInput { surah, ayah, body, author, source, lang }): Parameters<
            AddReflectionInput,
        >,
    ) -> Result<CallToolResult, McpError> {
        let id = queries::insert_reflection(
            &self.pool,
            surah,
            ayah,
            &body,
            author.as_deref(),
            source.as_deref(),
            &lang,
        )
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "{{\"id\": {id}}}"
        ))]))
    }
}
