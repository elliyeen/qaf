# Qaf — Standing Orders

## Project

**Qaf** is a Rust workspace providing word-level access to Quranic data:
a SQLite-backed data layer (`quran-db`), a REST API (`quran-api`),
and an MCP server (`quran-mcp`) for AI assistant integration.

---

## Itqān Standard

> "Indeed, Allah loves that when any of you does a job, he does it with itqān (perfection)."
> — al-Bayhaqī, Shu'ab al-Īmān, no. 5312, graded ḥasan by al-Albānī (al-Silsilah al-Ṣaḥīḥah no. 1113)

**Precision over speed. Verify before committing. No shortcuts.**

---

## Citation Rules

### Quranic References
Always cite: **Surah name (Arabic + English) · Surah number : Ayah number**
Example: Sūrat al-Fātiḥah (الفاتحة) · 1:1

### Hadith References
Every hadith citation must include:
1. **Collection** — e.g. Ṣaḥīḥ al-Bukhārī, Ṣaḥīḥ Muslim, Sunan Abī Dāwūd
2. **Number** — the hadith or narration number in that collection
3. **Grade** — ṣaḥīḥ / ḥasan / ḍa'īf / mawḍū', and which scholar graded it

---

## Build Order

1. `quran-db` — data layer; must compile and all tests pass before proceeding
2. `quran-api` — REST wrapper; must compile and smoke-test before proceeding
3. `quran-mcp` — MCP server; build last

**Never merge into quran-api or quran-mcp until `cargo test -p quran-db` passes cleanly.**

---

## MCP Crate

**Using `rmcp` v1.4.0** (official Rust MCP SDK from modelcontextprotocol org).

Chosen because:
- It is the official Rust SDK maintained by the MCP team
- v1.4.0 is the latest stable release on crates.io (checked 2026-04-14)
- It ships `#[tool_router]` and `#[tool]` proc-macros that eliminate boilerplate
- `transport-io` feature provides stdio transport with one line: `.serve(stdio())`

Features enabled: `server`, `macros`, `transport-io`

---

## Database

- File: `qaf.db` (SQLite)
- Migrations: `migrations/0001_initial.sql` (applied via `sqlx::migrate!`)
- Run manually: `sqlx migrate run --database-url sqlite:qaf.db`
- In tests: `sqlite::memory:` with migrations applied on startup

---

## Running

```bash
# REST API (default port 3000)
DATABASE_URL=sqlite:qaf.db PORT=3000 cargo run -p quran-api

# MCP server over stdio
DATABASE_URL=sqlite:qaf.db cargo run -p quran-mcp

# All tests
cargo test --workspace
```

---

## Seed Data

`data/seed/sample_words.json` — 7 words from Sūrat al-Fātiḥah (1:1–2).
These are the canonical test fixtures. Do not alter without updating tests.

---

## What Is Not Built Yet

- Full Quranic corpus import (all 6,236 ayahs, ~77,000 words)
- Arabic morphology from the Quranic Corpus (tanzil.net / corpus.quran.com)
- Tafsir integration
- Vector embeddings for semantic search
- Authentication on quran-api
