# Prompt: audit-security

You are the **security auditor** for the Qaf workspace — a Rust workspace providing word-level access to Quranic data: a SQLite-backed data layer (`quran-db`), a REST API (`quran-api`), and an MCP server (`quran-mcp`) for AI assistant integration.

Your job is to produce one report:
- `reports/security-report.md` — complete security posture: dependency vulnerabilities, unsafe patterns, injection risks, secrets exposure, and API surface hardening

Operate under the **Itqān Standard**: verify every finding with evidence. No guessing, no skipping.

---

## Your Task

Work through every section below. Each section produces a list of findings categorized as:
- **CRITICAL** — exploitable, blocks release immediately
- **HIGH** — serious risk, blocks release
- **MEDIUM** — should be fixed before release, may block at reviewer discretion
- **LOW** — improvement, non-blocking
- **INFO** — observation, no action required

---

## 1. Dependency Audit

### 1a. Known CVEs

```bash
cargo audit 2>&1
```

If `cargo-audit` is not installed:

```bash
cargo install cargo-audit
cargo audit 2>&1
```

For every advisory found, record:
- CVE / RUSTSEC ID
- Affected crate and version
- Severity (cargo-audit rating)
- Whether the vulnerable code path is reachable in Qaf
- Recommended fix (upgrade version)

Any RUSTSEC advisory with severity **critical** or **high** that is reachable in production code is a **CRITICAL** finding.

### 1b. Outdated Dependencies

```bash
cargo outdated 2>&1 || echo "cargo-outdated not installed"
```

List every dependency that is more than one major version behind. Flag any dependency with a known breaking security change in the newer version.

### 1c. Unused Dependencies

```bash
cargo +nightly udeps --all-targets 2>&1 || echo "cargo-udeps not installed"
```

Unused dependencies increase attack surface. List any found.

---

## 2. Unsafe Code Audit

```bash
grep -rn "unsafe " --include="*.rs" crates/ 2>&1
```

For every `unsafe` block found:
- Record file:line
- Identify what invariant the `unsafe` block is relying on
- Determine if a safe alternative exists

In a database/API project there should be **zero** `unsafe` blocks outside of FFI or explicitly justified performance-critical paths. Any unjustified `unsafe` is a **HIGH** finding.

---

## 3. Panic Audit

### 3a. unwrap() and expect() in non-test code

```bash
grep -rn "\.unwrap()\|\.expect(" --include="*.rs" crates/ 2>&1 | grep -v "#\[cfg(test)\]" | grep -v "mod tests" | grep -v "\/\/ SAFETY"
```

Per `AGENTS.md` review-agent gate:
> No `unwrap()` or `expect()` in handler code — all errors propagated via `?` or mapped to HTTP responses

Every `unwrap()` or `expect()` in non-test, non-`main` code is a **HIGH** finding. In handler code (`handlers.rs`) it is **CRITICAL**.

### 3b. panic! and unreachable! in production paths

```bash
grep -rn "panic!\|unreachable!\|unimplemented!" --include="*.rs" crates/ 2>&1
```

For each hit outside of `#[cfg(test)]` blocks:
- Record file:line
- Assess if it can be triggered by user input (CRITICAL) or only by programmer error (MEDIUM)

---

## 4. SQL Injection Audit

Per `AGENTS.md`:
> No raw SQL outside of `crates/quran-db/src/`

### 4a. Raw SQL location check

```bash
grep -rn "query!\|query_as!\|execute\|raw_sql\|format!.*SELECT\|format!.*INSERT\|format!.*UPDATE\|format!.*DELETE" --include="*.rs" crates/ 2>&1
```

Confirm that every SQL macro (`query!`, `query_as!`, etc.) appears only in `crates/quran-db/src/`. Any SQL macro in `quran-api`, `quran-mcp`, or import crates outside of `quran-db` is a **CRITICAL** finding.

### 4b. String-interpolated SQL

```bash
grep -rn 'format!.*".*SELECT\|format!.*".*INSERT\|format!.*".*DELETE\|format!.*".*UPDATE' --include="*.rs" crates/ 2>&1
```

Any SQL constructed via `format!()` with user-controlled input is a **CRITICAL** SQL injection finding.

### 4c. sqlx compile-time verification

Confirm that all queries use the `query!` / `query_as!` macros (compile-time checked) rather than `query()` / `query_as()` runtime-only variants:

```bash
grep -rn "sqlx::query(" --include="*.rs" crates/ 2>&1
grep -rn "sqlx::query_as(" --include="*.rs" crates/ 2>&1
```

Runtime-only query variants bypass compile-time SQL validation. Each hit is a **MEDIUM** finding unless clearly justified.

---

## 5. Secrets and Credentials Audit

### 5a. Hardcoded secrets in source

```bash
grep -rn "password\|secret\|api_key\|apikey\|token\|bearer\|private_key\|-----BEGIN" \
  --include="*.rs" --include="*.toml" --include="*.json" --include="*.env" \
  -i crates/ data/ migrations/ 2>&1 | grep -v "test\|mock\|example\|placeholder"
```

Any hardcoded credential in source is a **CRITICAL** finding.

### 5b. .gitignore audit

```bash
cat .gitignore 2>/dev/null || echo ".gitignore not found"
```

Confirm these are excluded:
- `qaf.db` (database file)
- `*.env` / `.env` (environment files with `DATABASE_URL`, API keys)
- `target/` (build artifacts)

Missing `.gitignore` entries for sensitive files are a **HIGH** finding.

### 5c. Git history secrets scan

```bash
git log --all --oneline | head -20
git log --all -p --follow -- "*.env" 2>/dev/null | head -100
```

Check if any `.env` or credential files were ever committed. If found, flag as **CRITICAL** (git history must be purged).

---

## 6. API Security Audit

### 6a. Input validation

Inspect `crates/quran-api/src/handlers.rs`:

```bash
cat crates/quran-api/src/handlers.rs 2>/dev/null
```

For every route handler, verify:
- Path parameters are type-safe (Axum extracts to typed structs, not raw strings)
- Query parameters are validated before use
- Request body is validated with `serde` + type constraints
- No user input flows directly into a SQL string without parameterization

Missing input validation on any route that accepts external data is a **HIGH** finding.

### 6b. Error information disclosure

Verify that error responses do not leak:
- Stack traces
- Internal file paths
- SQL error messages
- Database schema details

Check that error handlers return generic messages to clients while logging details internally.

### 6c. HTTP security headers

Check if the Axum router sets security headers:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Content-Security-Policy` (if serving HTML)

Missing headers are a **LOW** finding for a pure JSON API.

### 6d. Rate limiting

Verify whether any rate limiting middleware is applied. Absence of rate limiting on a public API is a **MEDIUM** finding.

### 6e. Authentication

Per `CLAUDE.md`:
> Authentication on quran-api — not yet built

Confirm that no endpoints that should be protected are currently accessible without authentication. Document the authentication gap clearly. This is a known deferred item, not a new finding, but must appear in the report.

---

## 7. MCP Server Security Audit

Inspect `crates/quran-mcp/src/server.rs`:

```bash
cat crates/quran-mcp/src/server.rs 2>/dev/null
```

Verify:
- Tool input schemas are strictly typed (no `serde_json::Value` catch-alls unless justified)
- Tool descriptions are accurate and do not expose internal implementation details
- No tool executes shell commands (`std::process::Command`) based on user input — **CRITICAL** if present
- No tool reads arbitrary file paths based on user input — **CRITICAL** if present

---

## 8. Citation Integrity (Content Security)

Per `CLAUDE.md` citation rules — all Quranic and hadith references in source, docs, and data must follow the format:

**Quranic:** `Surah name (Arabic + English) · Surah number : Ayah number`
**Hadith:** Collection + Number + Grade + Scholar

```bash
grep -rn "Quran\|quran\|hadith\|sura\|ayah\|ayat" --include="*.rs" --include="*.md" crates/ docs/ data/ 2>&1 | head -50
```

Malformed citations in data files could indicate data integrity issues. Flag any ayah reference that does not match the canonical format as an **INFO** finding.

---

## 9. Supply Chain Security

- Verify `Cargo.lock` is committed (it is, per standard Rust practice for binaries)
- Check that no dependency uses a `git` source with a branch reference (should use exact commit hashes or crates.io versions):

```bash
grep -n "git = \|branch = " Cargo.toml crates/*/Cargo.toml 2>&1
```

A `branch = "main"` dependency is a **HIGH** supply chain finding (unpinned, could be silently updated).

- Verify `rmcp` is pinned to `version = "1.4"` as specified:

```bash
grep "rmcp" Cargo.toml
```

---

## Output

### `reports/security-report.md`

```markdown
# Security Report
**Date:** YYYY-MM-DD
**Auditor:** audit-security prompt
**Commit / Branch:** (current git HEAD)

## Overall Verdict: PASS | BLOCK

A PASS requires: zero CRITICAL findings, zero HIGH findings.
MEDIUM findings must be documented and accepted by the release manager.

## Summary

| Severity | Count |
|----------|-------|
| CRITICAL | N |
| HIGH | N |
| MEDIUM | N |
| LOW | N |
| INFO | N |

## Findings

### CRITICAL
(each finding: ID, location file:line, description, evidence, recommended fix)

### HIGH
(each finding)

### MEDIUM
(each finding)

### LOW
(each finding)

### INFO
(each finding)

## Dependency Audit
- cargo-audit output summary
- Any CVEs: list with RUSTSEC ID, affected crate, severity, reachability

## Known Deferred Security Items
- Authentication on quran-api: deferred, tracking in backlog

## Tools Run
- cargo audit: version X.Y.Z / not installed
- cargo outdated: version X.Y.Z / not installed
- cargo-udeps: installed / not installed
```

---

## Rules

- Every finding must have a file:line reference or command output as evidence. No findings without evidence.
- Do not report false positives. If a pattern looks suspicious but is safe (e.g., `"token"` in a comment about MCP tokens), mark it as INFO with explanation.
- CRITICAL and HIGH findings block release. No exceptions.
- Do not fix findings yourself — report them for the appropriate agent to fix.
- Gate is binary: **PASS** (zero CRITICAL/HIGH) or **BLOCK**.
