# Qaf (قاف)

Word-level Quranic data access layer — Rust workspace.

---

## Crates

| Crate | Path | Purpose |
|-------|------|---------|
| `quran-db` | `crates/quran-db` | SQLite data layer — words, morphology, ontology, tadabbur, translations, irab, structure, recitations. |
| `quran-api` | `crates/quran-api` | REST API (Axum, port 3000). |
| `quran-mcp` | `crates/quran-mcp` | MCP server (rmcp 1.4, stdio transport) — 13 tools for AI assistant integration. |
| `quran-import` | `crates/quran-import` | CLI — imports QAC morphology file and seeds structural/recitation data. |
| `quran-tafsir-import` | `crates/quran-tafsir-import` | CLI — imports tafsir from the quran.com API. |
| `qaf-core` | `crates/qaf-core` | Shared types (`Surah`, `Ayah`, `Page`, `Juz`) and `QafError`. |

---

## Quick Start

```bash
# Apply migrations
sqlx migrate run --database-url sqlite:qaf.db

# REST API (port 3000)
DATABASE_URL=sqlite:qaf.db PORT=3000 cargo run -p quran-api

# MCP server (stdio)
DATABASE_URL=sqlite:qaf.db cargo run -p quran-mcp
```

---

## Build

```bash
cargo build --workspace
```

---

## Test

```bash
# All tests
cargo test --workspace

# Single crate
cargo test -p quran-db
```

---

## Lint

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

---

## Database

- File: `qaf.db` (SQLite, not committed)
- Migrations: `migrations/` — apply via `sqlx migrate run` or `sqlite3 qaf.db < migrations/XXXX.sql`
- Test databases: `sqlite::memory:` with migrations applied on startup

---

## MCP — Claude Desktop

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

## Standard

> "Indeed, Allah loves that when any of you does a job, he does it with itqān (perfection)."
> — al-Bayhaqī, Shu'ab al-Īmān no. 5312, graded ḥasan by al-Albānī (al-Silsilah al-Ṣaḥīḥah no. 1113)
