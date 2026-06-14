# Qaf (قاف)

Word-level Quranic data access layer — Rust workspace.

Qaf provides a SQLite-backed data layer, a REST API, and an MCP server for AI assistant integration, with Khalaf ʿan Ḥamzah as the primary recitation target.

---

## Crates

| Crate | Purpose |
|---|---|
| `quran-db` | SQLite data layer — all models, queries, and migrations |
| `quran-api` | REST API (Axum, port 3000) |
| `quran-mcp` | MCP server (rmcp 1.4, stdio) — 13 tools for AI integration |
| `quran-import` | CLI — QAC morphology import, structural and recitation seed |
| `quran-tafsir-import` | CLI — tafsir import via quran.com API |
| `qaf-core` | Shared types (`Surah`, `Ayah`, `Page`, `Juz`, `QafError`) |

---

## Quick Start

**Prerequisites:** Rust stable, `sqlx-cli`

```bash
# Install sqlx-cli if needed
cargo install sqlx-cli --no-default-features --features sqlite

# Clone and enter
git clone https://github.com/elliyeen/qaf.git && cd qaf

# Apply all migrations
sqlx migrate run --database-url sqlite:qaf.db

# Run the REST API (port 3000)
DATABASE_URL=sqlite:qaf.db PORT=3000 cargo run -p quran-api

# Run the MCP server (stdio)
DATABASE_URL=sqlite:qaf.db cargo run -p quran-mcp
```

---

## REST API

Base URL: `http://localhost:3000`

### Structural

| Method | Route | Description |
|---|---|---|
| `GET` | `/health` | Version, build, and environment |
| `GET` | `/surah/:num` | Surah metadata and ayahs (1–114) |
| `GET` | `/page/:num` | Page metadata and ayahs on that page (1–604) |
| `GET` | `/juz/:num` | Juz metadata, page range, and ayah range (1–30) |

### Lexical

| Method | Route | Description |
|---|---|---|
| `GET` | `/word/:surah/:ayah/:pos` | Single word with morphology |
| `GET` | `/surah/:num/words` | All words in a surah |
| `GET` | `/root/:root` | All words sharing a root |
| `GET` | `/morphology/:word_id` | Morphology for a word |
| `GET` | `/ontology/:root` | Semantic domain for a root |
| `GET` | `/search?q=&field=` | Search by `root`, `lemma`, or `arabic` |

### Tadabbur (Contemplation Layer)

| Method | Route | Description |
|---|---|---|
| `GET` | `/tadabbur/:surah/:ayah` | Full composite page — words, morphology, roots, reflections, themes, cross-references, irab |
| `GET/POST` | `/tadabbur/:surah/:ayah/reflect` | List or create reflections |
| `PUT/DELETE` | `/tadabbur/:surah/:ayah/reflect/:id` | Update or delete a reflection |
| `GET/POST` | `/tadabbur/:surah/:ayah/themes` | List or tag themes |
| `GET/POST` | `/tadabbur/:surah/:ayah/xref` | List or create cross-references |
| `GET/POST` | `/tadabbur/:surah/:ayah/translations` | List or add translations |
| `GET/POST` | `/tadabbur/:surah/:ayah/irab` | List or add grammatical analysis |
| `GET` | `/irab/:word_id` | Irab for a word by id |
| `PUT/DELETE` | `/irab/id/:id` | Update or delete an irab record |
| `POST` | `/word/:surah/:ayah/:pos/irab` | Add irab by word coordinate |

---

## MCP Server

Connect to Claude Desktop by adding to your config:

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

**Available tools:** `get_word`, `search_root`, `get_morphology`, `get_ontology`, `get_ayah_words`, `get_tadabbur_page`, `get_reflections`, `get_cross_refs`, `search_words`, `add_reflection`, `get_word_irab`, `get_ayah_irab`, `add_irab`

---

## Database

- **Engine:** SQLite (`qaf.db`, not committed to git)
- **Migrations:** `migrations/` — 10 files, applied in order
- **Tests:** `sqlite::memory:` with migrations applied on startup

### Schema Overview

| Migration | Tables Added |
|---|---|
| `0001_initial` | `words`, `morphology`, `ontology` |
| `0002_add_bare_columns` | `words.lemma_bare` (diacritic-insensitive search) |
| `0003_tadabbur` | `reflections`, `themes`, `ayah_themes`, `cross_references` |
| `0004_translations` | `translations` |
| `0005_hadith_cross_refs` | `hadith_cross_references` |
| `0006_irab` | `word_irab` |
| `0007_structure` | `juz`, `pages`, `surahs`, `ayahs` |
| `0008_word_tokens` | `word_segments` |
| `0009_khalaf` | `recitations`, `recitation_texts`, `tajweed_spans`, `tajweed_rule_colors` |
| `0010_audio` | `audio_files`, `audio_segments` |

Apply:
```bash
sqlx migrate run --database-url sqlite:qaf.db
```

---

## Development

### Build

```bash
cargo build --workspace
```

### Test

```bash
cargo test --workspace
cargo test -p quran-db   # data layer only
cargo test -p quran-api  # API + integration tests
```

### Lint

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

### Import Data

```bash
# Seed structural data (surahs, ayahs, juz, pages)
DATABASE_URL=sqlite:qaf.db cargo run -p quran-import --bin seed-structure

# Import QAC morphology
DATABASE_URL=sqlite:qaf.db cargo run -p quran-import --bin quran-import -- --qac data/import/quran-morphology.txt

# Import tafsir (Ibn Kathīr, English)
DATABASE_URL=sqlite:qaf.db cargo run -p quran-tafsir-import -- --tafsir-id 169
```

---

## What Is Not Built Yet

| Feature | Notes |
|---|---|
| Full corpus import | ~77,000 words — pipeline exists, data load not run |
| Tafsir data load | Client exists; quran.com API call not run |
| Audio routes | Schema exists; requires R2 and reciter files |
| Bookmark / reading position | Requires auth |
| Auth | Cloudflare Access is the intended gate |
| Cloudflare D1 | Local SQLite works; D1 wiring not started |
| Vector / semantic search | Not started |

---

## Citation Standard

### Quranic References
**Surah name (Arabic + English) · Surah number : Ayah number**
Example: Sūrat al-Fātiḥah (الفاتحة) · 1:1

### Hadith References
Every hadith citation must include:
1. **Collection** — e.g. Ṣaḥīḥ al-Bukhārī, Sunan Abī Dāwūd
2. **Number** — hadith number in that collection
3. **Grade** — ṣaḥīḥ / ḥasan / ḍaʿīf, and which scholar graded it

---

## Standard

> "Indeed, Allah loves that when any of you does a job, he does it with itqān (perfection)."
> — al-Bayhaqī, Shuʿab al-Īmān no. 5312, graded ḥasan by al-Albānī (al-Silsilah al-Ṣaḥīḥah no. 1113)

---

## License

MIT — see [LICENSE](LICENSE)
