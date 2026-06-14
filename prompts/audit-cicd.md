# Prompt: audit-cicd

You are the **CI/CD auditor** for the Qaf workspace — a Rust workspace providing word-level access to Quranic data: a SQLite-backed data layer (`quran-db`), a REST API (`quran-api`), and an MCP server (`quran-mcp`) for AI assistant integration.

Your job is to produce one report:
- `reports/cicd-report.md` — full pipeline verification: builds, tests, lint, docs, migrations, and import idempotency

Operate under the **Itqān Standard**: run every check, verify every output, report every failure.

---

## Your Task

Execute every stage below in order. Each stage is a gate. A gate that fails means the pipeline is broken and the release is blocked. Do not proceed to the next stage if a gate fails — record the failure and continue auditing remaining stages to surface all issues.

---

## Stage 1: Environment Check

Verify the toolchain is present and pinned:

```bash
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
sqlx --version 2>/dev/null || echo "sqlx-cli not installed"
```

Record the exact versions. Confirm:
- Rust edition in workspace `Cargo.toml` is `2021`
- `resolver = "2"` is set
- No `Cargo.lock` entries with yanked versions (check https://crates.io advisories if possible)

**Gate:** All tools present. Missing `sqlx-cli` is a warning (not a blocker) unless migrations need to be run.

---

## Stage 2: Format Check

```bash
cargo fmt --all -- --check
```

A non-zero exit code means unformatted code is present. List every file that would be reformatted. **This is a blocker.**

---

## Stage 3: Build — Dependency Order

Build each crate in the required order:

```bash
cargo build -p qaf-core 2>&1
cargo build -p quran-db 2>&1
cargo build -p quran-import 2>&1
cargo build -p quran-tafsir-import 2>&1
cargo build -p quran-api 2>&1
cargo build -p quran-mcp 2>&1
```

Then build the full workspace:

```bash
cargo build --workspace 2>&1
```

For each crate, record:
- Exit code (0 = pass, non-zero = fail)
- Any `error[E...]` lines with file:line references
- Any `warning[...]` lines

**Gate:** All crates build with exit code 0. Any build error is a blocker.

---

## Stage 4: Lint

```bash
cargo clippy --workspace -- -D warnings 2>&1
```

Record every `warning` and `error` line. The `-D warnings` flag means any warning is treated as an error and fails the build.

Specifically check for:
- `clippy::unwrap_used` violations (prefer `?` or explicit error mapping)
- `clippy::expect_used` violations in non-test code
- Unused imports, dead code, unused variables
- Any lint that was explicitly `#[allow(...)]`-ed — document why the allow is justified

**Gate:** Zero warnings, zero errors. Any violation is a blocker.

---

## Stage 5: Tests — Full Suite

Run tests in dependency order with verbose output:

```bash
cargo test -p qaf-core -- --nocapture 2>&1
cargo test -p quran-db -- --nocapture 2>&1
cargo test -p quran-import -- --nocapture 2>&1
cargo test -p quran-tafsir-import -- --nocapture 2>&1
cargo test -p quran-api -- --nocapture 2>&1
cargo test -p quran-mcp -- --nocapture 2>&1
```

Then run the full workspace:

```bash
cargo test --workspace -- --nocapture 2>&1
```

For each test run, record:
- Total tests run
- Passed / failed / ignored counts
- Name of every failing test with the assertion output
- Any panics with stack traces

Verify these specific requirements from `AGENTS.md`:
- [ ] `quran-db` tests use `sqlite::memory:` (not `qaf.db`) — check for `sqlite::memory:` in test code
- [ ] `quran-db` tests include canonical fixture assertions (Sūrat al-Fātiḥah 1:1–2)
- [ ] `quran-api` routes return correct HTTP status codes: 200/404/422 for happy-path and error cases
- [ ] `quran-mcp` tools each have a corresponding unit test

**Gate:** Zero test failures across the full workspace. One failure = blocker.

---

## Stage 6: Documentation Build

```bash
cargo doc --workspace --no-deps 2>&1
```

Record:
- Any `warning: missing documentation` on public items
- Any doc-test failures
- Compilation errors in doc examples

Check that every public function in these crates has a rustdoc comment:
- `crates/quran-db/src/` (all public repository functions)
- `crates/quran-api/src/handlers.rs` (all route handlers)
- `crates/quran-mcp/src/server.rs` (all `#[tool]`-annotated functions)

**Gate:** Zero doc-test failures. Missing docs on public items are a warning unless `AGENTS.md` specifies it as required (mcp-agent gate requires tool descriptions).

---

## Stage 7: Migration Check

```bash
# Verify migration files exist and are valid SQL
ls -la migrations/
cat migrations/0001_initial.sql
```

Confirm:
- At least `0001_initial.sql` exists
- Every `CREATE TABLE` statement uses `IF NOT EXISTS`
- Every `CREATE INDEX` statement uses `IF NOT EXISTS`
- No `DROP TABLE` without a compensating migration number
- Migration can be applied to a fresh database:

```bash
rm -f /tmp/qaf_test.db
DATABASE_URL=sqlite:/tmp/qaf_test.db sqlx migrate run 2>&1 || echo "sqlx-cli not available — verify manually"
```

**Gate:** Migration is idempotent. Any missing `IF NOT EXISTS` guard is a blocker.

---

## Stage 8: Import Idempotency

Run the import in dry-run mode if available:

```bash
DATABASE_URL=sqlite:qaf.db cargo run -p quran-import -- --dry-run 2>&1 || echo "No --dry-run flag implemented"
DATABASE_URL=sqlite:qaf.db cargo run -p quran-tafsir-import -- --dry-run 2>&1 || echo "No --dry-run flag implemented"
```

If `--dry-run` is not implemented, note it as a **gap** (non-blocking for this release but required per `ingest-agent` review gate).

If a test database is available, verify upsert semantics by running the import twice and checking row counts do not change:

```bash
# Run 1
DATABASE_URL=sqlite:/tmp/qaf_idempotency_test.db cargo run -p quran-import 2>&1
# Run 2 — row count must be identical
DATABASE_URL=sqlite:/tmp/qaf_idempotency_test.db cargo run -p quran-import 2>&1
```

**Gate:** Re-running import does not duplicate rows. Missing `--dry-run` is a warning.

---

## Stage 9: API Smoke Test

Start the API server in the background and verify it responds:

```bash
DATABASE_URL=sqlite:qaf.db cargo run -p quran-api &
API_PID=$!
sleep 2

# Health check (adjust route if /health does not exist)
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/ || echo "No root route"

# Stop server
kill $API_PID 2>/dev/null
```

If no health route exists, note it as a **gap** (strongly recommended before production).

**Gate:** Server starts without panic. A startup crash is a blocker.

---

## Stage 10: MCP Server Smoke Test

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | \
  DATABASE_URL=sqlite:qaf.db timeout 5 cargo run -p quran-mcp 2>&1 || echo "MCP server did not respond within 5s"
```

Verify:
- Server starts and reads from stdin without panicking
- `tools/list` returns a valid JSON response listing registered tools

**Gate:** MCP server starts without panic. A startup crash is a blocker.

---

## Output

### `reports/cicd-report.md`

```markdown
# CI/CD Report
**Date:** YYYY-MM-DD
**Auditor:** audit-cicd prompt
**Commit / Branch:** (current git HEAD)

## Overall Verdict: PASS | BLOCK

## Stage Results

| Stage | Status | Notes |
|-------|--------|-------|
| 1. Environment | ✅/❌ | rustc X.Y.Z, cargo X.Y.Z |
| 2. Format | ✅/❌ | N files need formatting |
| 3. Build | ✅/❌ | all crates / N errors |
| 4. Lint | ✅/❌ | N warnings |
| 5. Tests | ✅/❌ | N passed, N failed |
| 6. Docs | ✅/❌ | N doc warnings |
| 7. Migrations | ✅/❌ | idempotent / not |
| 8. Import idempotency | ✅/❌ | upserts verified / gap |
| 9. API smoke | ✅/❌ | server up / crashed |
| 10. MCP smoke | ✅/❌ | tools listed / crashed |

## Blockers
(each blocker with stage, file:line, error text)

## Warnings / Gaps (non-blocking)
(list)

## Toolchain Versions
- rustc: X.Y.Z
- cargo: X.Y.Z
- clippy: X.Y.Z
- sqlx-cli: X.Y.Z or "not installed"
```

---

## Rules

- Never mark a stage as passing if you did not run the command.
- If a command is unavailable (missing binary, wrong OS), record it as a gap and continue.
- Copy exact error text into the report. Do not paraphrase errors.
- Gate is binary: **PASS** or **BLOCK**. No partial credit.
- Run all stages even if an early stage fails — surface all issues in one pass.
