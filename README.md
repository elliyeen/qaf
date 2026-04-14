# Qaf (قاف)

Word-level Quranic data access layer — Rust workspace.

```
quran-db   SQLite queries, models, migrations
quran-api  Axum REST API
quran-mcp  MCP server (rmcp 1.4 · stdio transport)
```

## Quick Start

```bash
# 1. Apply migrations
sqlx migrate run --database-url sqlite:qaf.db

# 2. Start the REST API
DATABASE_URL=sqlite:qaf.db cargo run -p quran-api

# 3. Start the MCP server
DATABASE_URL=sqlite:qaf.db cargo run -p quran-mcp
```

## REST Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/word/:surah/:ayah/:pos` | Single word |
| GET | `/root/:root` | All words sharing a root |
| GET | `/morphology/:word_id` | POS + features |
| GET | `/search?q=…&field=root\|lemma` | Search |
| GET | `/surah/:num/words` | All words in a surah |
| GET | `/ontology/:root` | Semantic domain + derivatives |
| GET | `/health` | `{ "status": "ok" }` |

## MCP Tools

| Tool | Description |
|------|-------------|
| `get_word(surah, ayah, position)` | Fetch a single Quranic word with full morphological data |
| `search_root(root_arabic)` | Find all words in the Quran sharing a given Arabic root |
| `get_morphology(word_id)` | Get part-of-speech and grammatical features for a word |
| `get_ontology(root_arabic)` | Get semantic domain, derivatives, and scholar notes for a root |

## Configure in Claude Desktop

```json
{
  "mcpServers": {
    "quran": {
      "command": "/path/to/quran-mcp",
      "env": { "DATABASE_URL": "sqlite:/path/to/qaf.db" }
    }
  }
}
```
