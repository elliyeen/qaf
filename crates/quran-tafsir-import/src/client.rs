//! Thin async client for the quran.com public API v4.
//!
//! Relevant endpoints used here:
//!
//!   GET /api/v4/resources/tafsirs
//!       → lists every available tafsir resource with its id, author, language.
//!
//!   GET /api/v4/tafsirs/{tafsir_id}/by_chapter/{chapter}
//!       ?page={n}&per_page=50
//!       → returns one page of tafsir entries for all verses in a chapter.
//!         `meta.next_page` is null when the last page is reached.
//!
//! The API is public and requires no authentication key.
//! Rate-limit: ~100 req/s in practice; we sleep 80ms between chapter fetches.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

const BASE: &str = "https://api.quran.com/api/v4";

// ─── Resource listing ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct TafsirResource {
    pub id: u32,
    pub name: String,
    pub author_name: String,
    pub slug: String,
    pub language_name: String,
}

#[derive(Deserialize)]
struct ResourcesResponse {
    tafsirs: Vec<TafsirResource>,
}

// ─── Per-verse tafsir entry ───────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct TafsirVerse {
    /// Verse coordinate, e.g. "2:255"
    pub verse_key: String,
    /// HTML/text content of the tafsir for this verse.
    pub text: Option<String>,
}

#[derive(Deserialize)]
struct Pagination {
    next_page: Option<u32>,
}

#[derive(Deserialize)]
struct TafsirPageResponse {
    tafsirs: Vec<TafsirVerse>,
    pagination: Pagination,
}

// ─── Client ──────────────────────────────────────────────────────────────────

pub struct QuranClient {
    http: Client,
}

impl QuranClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("qaf/0.1 (quran-tafsir-import; github.com/qaf)")
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { http })
    }

    /// List all available tafsir resources from quran.com.
    pub async fn list_tafsirs(&self) -> Result<Vec<TafsirResource>> {
        let url = format!("{}/resources/tafsirs", BASE);
        let resp: ResourcesResponse = self
            .http
            .get(&url)
            .send()
            .await
            .context("GET /resources/tafsirs failed")?
            .error_for_status()
            .context("quran.com returned an error for /resources/tafsirs")?
            .json()
            .await
            .context("failed to parse /resources/tafsirs JSON")?;
        Ok(resp.tafsirs)
    }

    /// Fetch *all* tafsir verses for a given chapter, handling pagination.
    ///
    /// Sleeps 80 ms between page requests to respect the public rate limit.
    pub async fn chapter_tafsir(
        &self,
        tafsir_id: u32,
        chapter: u32,
    ) -> Result<Vec<TafsirVerse>> {
        let mut verses: Vec<TafsirVerse> = Vec::new();
        let mut page: u32 = 1;

        loop {
            let url = format!(
                "{}/tafsirs/{}/by_chapter/{}?page={}&per_page=50",
                BASE, tafsir_id, chapter, page
            );

            let resp: TafsirPageResponse = self
                .http
                .get(&url)
                .send()
                .await
                .with_context(|| format!("GET tafsir {tafsir_id} chapter {chapter} p{page}"))?
                .error_for_status()
                .with_context(|| format!("quran.com error: tafsir {tafsir_id} chapter {chapter}"))?
                .json()
                .await
                .with_context(|| format!("JSON parse error: tafsir {tafsir_id} chapter {chapter}"))?;

            verses.extend(resp.tafsirs);

            match resp.pagination.next_page {
                Some(next) => {
                    page = next;
                    sleep(Duration::from_millis(80)).await;
                }
                None => break,
            }
        }

        Ok(verses)
    }
}
