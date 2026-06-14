# Contributing to Qaf

## Itqān Standard

> "Indeed, Allah loves that when any of you does a job, he does it with itqān (perfection)."
> — al-Bayhaqī, Shuʿab al-Īmān no. 5312, graded ḥasan by al-Albānī (al-Silsilah al-Ṣaḥīḥah no. 1113)

Precision over speed. Every change must pass its review gate before merging.

---

## Build Order

Changes must be made in dependency order:

```
quran-db  →  quran-api
          →  quran-mcp
          →  quran-import
          →  quran-tafsir-import
```

Never merge into `quran-api` or `quran-mcp` until `cargo test -p quran-db` passes cleanly.

---

## Before Opening a PR

```bash
# All tests must pass
cargo test --workspace

# No clippy warnings
cargo clippy --workspace -- -D warnings

# Formatted
cargo fmt --check
```

All three must be green. A PR that fails any of these will be blocked.

---

## Schema Changes

- Add a new numbered migration file: `migrations/XXXX_description.sql`
- All DDL must use `IF NOT EXISTS` — migrations must be safe to re-run
- Update `docs/architecture.md` schema table
- Add tests in `quran-db` that cover the new tables

---

## API Changes

- New routes go in `crates/quran-api/src/routes.rs` and `handlers.rs`
- All handlers must return typed errors via `ApiError` — no bare `unwrap()` or `expect()`
- Add integration tests in `crates/quran-api/tests/integration.rs`

---

## MCP Changes

- New tools go in `crates/quran-mcp/src/server.rs`
- Input schemas go in `crates/quran-mcp/src/schema.rs`
- Tool descriptions must be clear enough for an AI to use without additional context
- Include Quranic citation format in tool descriptions where applicable

---

## Citation Rules

### Quranic References
Always cite: **Surah name (Arabic + English) · Surah number : Ayah number**

### Hadith References
Every hadith must include:
1. Collection name
2. Hadith number in that collection
3. Grade and grading scholar

---

## Commit Style

```
type: short description (≤72 chars)

Optional body explaining why, not what.
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`

---

## What Not to Submit

- Audio binary files (`.mp3`, `.m4a`) — these go to R2
- `qaf.db` or any `.db` file
- `data/import/quran-morphology.txt` — re-downloadable source data
- Credentials or `.env` files
