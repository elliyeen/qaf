# Prompt: audit-project

You are the **project auditor** for the Qaf workspace — a Rust workspace providing word-level access to Quranic data: a SQLite-backed data layer (`quran-db`), a REST API (`quran-api`), and an MCP server (`quran-mcp`) for AI assistant integration.

Your job is to produce two reports:
- `reports/status-report.md` — current health of every crate
- `reports/backlog-report.md` — known gaps, incomplete features, and open work

Operate under the **Itqān Standard**: precision over speed, verify before committing, no shortcuts.

---

## Your Task

Work through every section below in order. Do not skip sections. Record every finding.

---

## 1. Workspace Structure

Verify the workspace members match `Cargo.toml`:

```
crates/quran-db
crates/quran-api
crates/quran-mcp
crates/quran-import
crates/quran-tafsir-import
crates/qaf-core
```

For each crate, confirm:
- `Cargo.toml` exists and is valid
- `src/lib.rs` or `src/main.rs` exists
- The crate compiles: `cargo build -p <crate>`

Report any crate that is missing, broken, or fails to build. A failing crate is a **blocker**.

---

## 2. Test Status

Run tests in build-order dependency sequence:

```bash
cargo test -p quran-db
cargo test -p qaf-core
cargo test -p quran-import
cargo test -p quran-tafsir-import
cargo test -p quran-api
cargo test -p quran-mcp
cargo test --workspace
```

For each run, record:
- Pass count
- Fail count
- Ignored count
- Any test output showing panics or assertion failures

**A single test failure is a blocker for release.**

---

## 3. Lint Status

```bash
cargo clippy --workspace -- -D warnings
```

Record every warning or error. Any warning promoted to error is a **blocker**.

---

## 4. Documentation

```bash
cargo doc --workspace --no-deps 2>&1
```

Record any:
- Missing rustdoc on public items
- Doc-test failures
- Compilation warnings in doc generation

Also check the markdown docs:
- `README.md` — does it reflect the current crate list and commands?
- `AGENTS.md` — does the agent roster match crates that actually exist?
- `CLAUDE.md` — are all commands listed still valid?
- `docs/` — any stale architecture references?

Flag any doc that references a crate, table, or command that no longer exists.

---

## 5. Database & Migrations

- Verify `migrations/` contains at least `0001_initial.sql`
- Confirm migration is idempotent (`IF NOT EXISTS` guards on all DDL)
- Verify `qaf.db` is excluded from version control (`.gitignore`)
- Check that `DATABASE_URL=sqlite:qaf.db` is the only required env var for the API

---

## 6. Seed Data

- Confirm `data/seed/sample_words.json` exists and contains the 7 canonical words from Sūrat al-Fātiḥah (1:1–2)
- Confirm no tests hardcode row counts that would break if seed data changed

---

## 7. Build Order Compliance

Verify the build order rule from `CLAUDE.md` is respected:

1. `quran-db` compiles and tests pass
2. `quran-api` depends only on `quran-db` (not on `quran-mcp`)
3. `quran-mcp` depends only on `quran-db` (not on `quran-api`)

Check `Cargo.toml` dependency declarations for each crate. Flag any circular or out-of-order dependency.

---

## 8. Backlog Assessment

Evaluate each item listed in `CLAUDE.md` under "What Is Not Built Yet":

| Item | Status | Blocking Release? |
|------|--------|-------------------|
| Full Quranic corpus import (6,236 ayahs, ~77,000 words) | ? | ? |
| Arabic morphology from QAC (tanzil.net / corpus.quran.com) | ? | ? |
| Tafsir integration | ? | ? |
| Vector embeddings for semantic search | ? | ? |
| Authentication on quran-api | ? | ? |

For each item, determine:
- **Not started** / **In progress** / **Partially complete** / **Complete**
- Whether it blocks the current release or is deferred

Also scan the codebase for `TODO`, `FIXME`, `HACK`, `XXX`, `unimplemented!()`, and `todo!()` markers:

```bash
grep -rn "TODO\|FIXME\|HACK\|XXX\|unimplemented!\|todo!" --include="*.rs" crates/
```

Add every hit to the backlog report with file:line reference.

---

## 9. Agent Gate Compliance

Verify the gate dependency order from `AGENTS.md` is satisfiable:

```
db-agent → ingest-agent, ontology-agent, api-agent, mcp-agent → docs-agent → review-agent
```

Confirm that no crate has been merged with a failing upstream gate. Check git log for the last merge commit on each crate branch (if applicable).

---

## Output

### `reports/status-report.md`

```markdown
# Project Status Report
**Date:** YYYY-MM-DD
**Auditor:** audit-project prompt

## Overall Verdict: PASS | BLOCK

## Crate Health
| Crate | Builds | Tests | Clippy | Notes |
|-------|--------|-------|--------|-------|
| quran-db | ✅/❌ | ✅/❌ | ✅/❌ | |
| quran-api | ✅/❌ | ✅/❌ | ✅/❌ | |
| quran-mcp | ✅/❌ | ✅/❌ | ✅/❌ | |
| quran-import | ✅/❌ | ✅/❌ | ✅/❌ | |
| quran-tafsir-import | ✅/❌ | ✅/❌ | ✅/❌ | |
| qaf-core | ✅/❌ | ✅/❌ | ✅/❌ | |

## Blockers
(list each blocker with crate, file:line, and description)

## Warnings (non-blocking)
(list each warning)

## Documentation Health
(summary of doc gaps)

## Database Health
(migration status, seed data status)
```

### `reports/backlog-report.md`

```markdown
# Backlog Report
**Date:** YYYY-MM-DD
**Auditor:** audit-project prompt

## Known Incomplete Features
(table of backlog items with status)

## In-Code TODOs / FIXMEs
(list with file:line references)

## Deferred Items (not blocking current release)
(list)

## Items That Must Ship Before Next Release
(list)
```

---

## Rules

- Never mark a section as passing if you have not run the command or read the file.
- If a command fails to run (missing toolchain, etc.), report it as a blocker, do not skip.
- Do not editorialize. Report facts: pass/fail counts, exact file:line references, exact error text.
- Gate is binary: **PASS** or **BLOCK**. No "mostly passing."
