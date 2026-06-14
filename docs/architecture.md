# Qaf — Architecture

> **Cloudflare-first.** The database is SQLite-compatible (D1). The API and MCP server target Cloudflare Workers. No external auth providers, no third-party databases, no vendor-specific ORMs.

---

## System Overview

Qaf is a Rust workspace that provides word-level access to the Quranic corpus. It is composed of three layers:

```
┌──────────────────────────────────────────────────────┐
│                   Consumers                          │
│   Claude Desktop (MCP)    HTTP clients (REST API)    │
└────────────────┬───────────────────┬─────────────────┘
                 │                   │
     ┌───────────▼──────┐ ┌──────────▼──────────┐
     │   quran-mcp      │ │    quran-api         │
     │ rmcp 1.4 · stdio │ │  Axum · port 3000    │
     └────────┬─────────┘ └──────────┬───────────┘
              │                       │
     ┌────────▼───────────────────────▼───────────┐
     │                quran-db                    │
     │  SQLite via sqlx · Words · Morphology      │
     │  Tadabbur · Reflections · Translations     │
     │  Irab · Structure · Recitations · Tokens   │
     └────────────────────────────────────────────┘
```

---

## Crate Responsibilities

| Crate | Binary | Responsibility |
|---|---|---|
| `qaf-core` | — | Shared types (`Surah`, `Ayah`, `Page`, `Juz`) and `QafError`. No I/O. |
| `quran-db` | — | Repository layer over SQLite. All SQL, models, and migrations live here. |
| `quran-api` | `quran-api` | REST API (Axum, port 3000). |
| `quran-mcp` | `quran-mcp` | MCP server (rmcp 1.4, stdio). 13 tools for AI assistant integration. |
| `quran-import` | `quran-import`, `seed-structure`, `seed-recitations` | CLI: QAC morphology import, structural seed, recitation seed. |
| `quran-tafsir-import` | `quran-tafsir-import` | CLI: tafsir import via quran.com API. |

---

## Database

### Engine

SQLite — accessed via `sqlx` with async `runtime-tokio`.

- Local development: `qaf.db` file at workspace root
- Tests: `sqlite::memory:` with migrations applied on startup via `sqlx::migrate!`
- Production target: **Cloudflare D1** (SQLite-compatible, no driver changes needed)

### Migrations

Located in `migrations/`, numbered sequentially:

| File | Purpose |
|---|---|
| `0001_initial.sql` | `words`, `morphology`, `ontology` tables |
| `0002_add_bare_columns.sql` | `lemma_bare` for diacritic-insensitive search |
| `0003_tadabbur.sql` | `reflections`, thematic tags, cross-references |
| `0004_translations.sql` | Translation text per ayah |
| `0005_hadith_cross_refs.sql` | Hadith cross-reference links |
| `0006_irab.sql` | Grammatical parsing (iʿrāb) |
| `0007_structure.sql` | Surah/juz/hizb structural metadata |
| `0008_word_tokens.sql` | Tokenised word forms for search |

All DDL uses `IF NOT EXISTS` — migrations are safe to re-run.

Apply:
```bash
sqlx migrate run --database-url sqlite:qaf.db
```

---

## Public Interfaces

### REST API (`qaf-api`)

- Framework: **Axum 0.7**
- Default port: `3000`
- All responses: `application/json`
- Error shape: `{ "error": "<message>" }`

Run:
```bash
DATABASE_URL=sqlite:qaf.db PORT=3000 cargo run -p quran-api
```

Production target: **Cloudflare Workers** (via a WASM-compatible build or a Worker that proxies to a self-hosted binary).

### MCP Server (`qaf-mcp`)

- SDK: **rmcp 1.4.0** (`server` + `macros` + `transport-io` features)
- Transport: **stdio** (one line: `.serve(stdio())`)
- Tool definitions: `#[tool]` proc-macro on each handler function

Run:
```bash
DATABASE_URL=sqlite:qaf.db cargo run -p quran-mcp
```

Claude Desktop config:
```json
{
  "mcpServers": {
    "qaf": {
      "command": "/path/to/qaf-mcp",
      "env": { "DATABASE_URL": "sqlite:/path/to/qaf.db?mode=rwc" }
    }
  }
}
```

---

## Ingestion Pipeline (`quran-import` / `quran-tafsir-import`)

```
QAC morphology .txt
        │
        ▼
  quran-import CLI
        │  --qac path/to/quranic-corpus-morphology.txt
        │
        ▼
  parse tab-delimited records
        │
        ▼
  upsert into words + morphology tables
        │
        ▼
  progress bar (indicatif) + final summary

quran.com API
        │
        ▼
  quran-tafsir-import CLI
        │  --tafsir-id 169
        │
        ▼
  INSERT OR IGNORE into reflections table
```

External data sources used by ingest:

| Source | Data | Licence |
|---|---|---|
| Quranic Arabic Corpus (corpus.quran.com) | Morphological analysis | CC BY 3.0 |
| quran.com API | Tafsir, translations | Public API |

No proprietary or vendor-locked data sources.

---

## Deployment Targets

| Environment | Database | API Host | MCP Host |
|---|---|---|---|
| Local dev | `qaf.db` (file) | `localhost:3000` | stdio |
| CI | `sqlite::memory:` | Not started | Not started |
| Production | **Cloudflare D1** | **Cloudflare Workers** | stdio (local binary) |

Cloudflare is the only supported production hosting target. No other cloud providers, no Supabase, no PlanetScale, no Neon.

---

## Dependency Graph

```
qaf-core (shared types only)

quran-db (no internal deps)
  ├── quran-api
  ├── quran-mcp
  ├── quran-import
  └── quran-tafsir-import
```

No circular dependencies. `qaf-core` and `quran-db` have no internal dependencies.

---

## What Is Not Built Yet

| Feature | Notes |
|---|---|
| Full corpus import | All 6,236 ayahs, ~77,000 words — pipeline exists, data load not run |
| Arabic morphology (full) | QAC file import implemented; full run pending |
| Tafsir integration | API client exists in `quran-tafsir-import`; data not loaded |
| Vector embeddings | Not started — semantic search requires embedding pipeline |
| Auth on `quran-api` | Endpoints are currently open; Cloudflare Access is the intended gate |
| Cloudflare D1 migration | Local SQLite works; D1 wiring pending Wrangler config |
| Audio routes | Requires R2 storage, reciter audio files, and timestamp data |
| Bookmark routes | Requires auth and a user table |
