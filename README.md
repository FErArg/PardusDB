# PardusDB

**A fast, SQLite-like embedded vector database with graph-based approximate nearest neighbor search**

[![Version](https://img.shields.io/badge/version-0.4.17-blue.svg)](ssh://git@svr1.serinfo.com.es:6022/home/git/pardus-rag.git)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.85-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python-3.10+-blue.svg)](https://python.org/)

PardusDB is designed for developers building local AI applications — RAG pipelines, semantic search, recommendation systems, or any project that needs lightweight, persistent vector storage without external dependencies.

While [Pardus AI](https://pardusai.org/) gives non-technical users a powerful no-code platform to ask questions of their CSV, JSON, and PDF data in plain English, PardusDB gives developers the same speed and privacy in an embeddable, fully open-source vector database.

## Contributors

- [FErArg](https://ferarg.com) — *Individual contributor*
- **Deepseek** — *AI research and development*
- **Miramax** — *AI research and development*


## Features

- **Single-file storage** — Everything lives in one `.pardus` file, just like SQLite
- **Multiple tables** — Store different vector dimensions and metadata in the same database
- **Familiar SQL-like syntax** — CREATE, INSERT, SELECT, UPDATE, DELETE feel natural
- **UNIQUE constraints** — O(1) duplicate detection using HashSet
- **GROUP BY with aggregates** — O(n) hash aggregation with COUNT, SUM, AVG, MIN, MAX
- **JOINs** — O(n+m) hash join algorithm for INNER, LEFT, RIGHT joins
- **Fast vector similarity search** — Graph-based approximate nearest neighbor search
- **Thread-safe** — Safe concurrent reads in multi-threaded applications
- **Full transactions** — BEGIN/COMMIT/ROLLBACK for atomic operations
- **Optional GPU acceleration** — For large batch inserts and queries
- **Python MCP server**
- **Import documents from disk** — PDF, CSV, DOCX, XLSX, XLS, JSON, JSONL, MD, TXT with auto-embeddings and parent-child tracking
- **Optional dependency installers** — Install document parsing libraries (pypdf, python-docx, openpyxl, xlrd) and sentence-transformers for auto-embeddings via setup.sh/install.sh
- **Database health checks** — Verify integrity, detect orphans, check dimensions

## Installation

Two installers are provided. Both install the binary, helper script, MCP server, Python SDK, and config — the only difference is how the binary is obtained.

### Option 1: setup.sh — Build from source (requires Rust)

```bash
git clone ssh://git@svr1.serinfo.com.es:6022/home/git/pardus-rag.git
cd pardusdb
./setup.sh --install
```

Compiles `pardusdb` from Rust source with `cargo build --release`. Use this if you want the latest code or have modified the source. Rust is installed automatically if missing.

### Option 2: install.sh — Use precompiled binary (no Rust)

```bash
git clone ssh://git@svr1.serinfo.com.es:6022/home/git/pardus-rag.git
cd pardusdb
./install.sh --install
```

Copies the precompiled binary from `bin/pardus-v0.4.17-linux-x86_64` to `~/.local/bin/pardusdb`. No Rust compilation — faster but requires a pre-existing binary in the repo.

### Option 3: install-macos.sh — macOS with venv-based MCP (auto-installs Python 3.10+ if needed)

```bash
git clone ssh://git@svr1.serinfo.com.es:6022/home/git/pardus-rag.git
cd pardusdb
./install-macos.sh --install
```

Requires the precompiled macOS binary `bin/pardus-v0.4.17-darwin-arm64` in the repo. If not present, compile on your Mac with `cargo build --release` and copy to that path. Installs the MCP server inside a Python virtual environment (`~/.pardus/mcp/venv/`). **If Python < 3.10 is detected, automatically offers to install Python 3.13 via Homebrew.**

| | setup.sh | install.sh | install-macos.sh |
|---|---|---|---|
| Requires Rust | Yes (auto-installed) | No | No |
| Requires Python 3.10+ | No | No | **Yes (auto-installed via Homebrew)** |
| Compiles source | Yes | No | **Only if macOS binary missing** |
| Binary from | `bin/pardus-v*-{platform}-{arch}` | `bin/pardus-v*-linux-x86_64` | `bin/pardus-v*-darwin-arm64` |
| MCP installation | global pip | global pip | virtual environment |
| macOS compatibility | Partial | Partial | **Recommended** |
| Speed | ~1-3 min | <1 sec | <1 sec + Python install if needed |

See [INSTALL.md](INSTALL.md) for detailed instructions.

## Quick Start

### Using the Helper (Recommended)

The `pardus` helper automatically manages the default database at `~/.pardus/pardus-rag.db`:

```bash
pardus                    # Opens database, creates if missing
pardus mi.db              # Open specific file
```

### Using the REPL

```bash
pardus
```

```
╔═══════════════════════════════════════════════════════════════╗
║                    PardusDB REPL                      ║
║          Vector Database with SQL Interface           ║
╚═══════════════════════════════════════════════════════════════╝

pardusdb [~/.pardus/pardus-rag.db]> CREATE TABLE docs (embedding VECTOR(768), content TEXT);
Table 'docs' created

pardusdb [~/.pardus/pardus-rag.db]> INSERT INTO docs (embedding, content)
VALUES ([0.1, 0.2, 0.3, ...], 'Hello World');
Inserted row with id=1

pardusdb [~/.pardus/pardus-rag.db]> SELECT * FROM docs
WHERE embedding SIMILARITY [0.1, 0.2, 0.3, ...] LIMIT 5;

Found 1 similar rows:
  id=1, distance=0.0000, values=[Vector([...]), Text("Hello World")]

pardusdb [~/.pardus/pardus-rag.db]> quit
Saved to: ~/.pardus/pardus-rag.db
Goodbye!
```

## SQL Syntax

### Data Types

| Type | Description | Example |
|------|-------------|---------|
| `VECTOR(n)` | n-dimensional float vector | `VECTOR(768)` |
| `TEXT` | UTF-8 string | `'hello world'` |
| `INTEGER` | 64-bit integer | `42` |
| `FLOAT` | 64-bit float | `3.14` |
| `BOOLEAN` | true/false | `true` |

### Basic Operations

```sql
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    embedding VECTOR(768),
    title TEXT,
    category TEXT,
    score FLOAT
);

INSERT INTO documents (embedding, title, category, score)
VALUES ([0.1, 0.2, ...], 'Introduction to Rust', 'tutorial', 0.95);

SELECT * FROM documents WHERE category = 'tutorial' LIMIT 10;

UPDATE documents SET score = 0.99 WHERE id = 1;

DELETE FROM documents WHERE id = 1;
```

### Vector Similarity Search

```sql
SELECT * FROM documents
WHERE embedding SIMILARITY [0.12, 0.24, ...]
LIMIT 10;
```

Results are automatically ordered by distance (closest first).

### UNIQUE Constraint

```sql
CREATE TABLE users (
    embedding VECTOR(128),
    id INTEGER PRIMARY KEY,
    email TEXT UNIQUE
);

-- This will fail - duplicate email
INSERT INTO users (embedding, id, email) VALUES ([0.1, ...], 1, 'test@example.com');
-- Error: Duplicate value for UNIQUE column 'email'
```

### GROUP BY with Aggregates

```sql
SELECT category, COUNT(*), AVG(score), SUM(amount)
FROM sales
GROUP BY category;

SELECT category, SUM(amount) as total
FROM sales
GROUP BY category
HAVING SUM(amount) > 1000;
```

### JOINs

```sql
SELECT * FROM orders
INNER JOIN users ON orders.user_id = users.id;

SELECT users.email, orders.product
FROM users
LEFT JOIN orders ON users.id = orders.user_id;
```

## REPL Commands

| Command | Description |
|---------|-------------|
| `.create <file>` | Create and open a new database |
| `.open <file>` | Open an existing database |
| `.save` | Force save current database |
| `.tables` | List tables |
| `.clear` | Clear screen |
| `help` | Show help |
| `quit` | Exit (auto-saves if file open) |

## MCP Server for AI Agents

PardusDB includes an MCP server that allows AI agents (OpenCode, Claude Desktop, etc.) to interact with the database using natural language.

### Tools Available

| Tool | Description |
|------|-------------|
| `pardusdb_create_database` | Create a new database file |
| `pardusdb_open_database` | Open an existing database |
| `pardusdb_create_table` | Create a new table |
| `pardusdb_insert_vector` | Insert a single vector |
| `pardusdb_batch_insert` | Batch insert multiple vectors |
| `pardusdb_search_similar` | Search by vector similarity |
| `pardusdb_execute_sql` | Execute raw SQL |
| `pardusdb_list_tables` | List all tables |
| `pardusdb_use_table` | Set active table |
| `pardusdb_status` | Show connection status |
| `pardusdb_import_text` | Import documents from a directory (PDF, CSV, DOCX, XLSX, JSON, JSONL, MD, TXT) with auto-embeddings |
| `pardusdb_health_check` | Run integrity checks on tables and data |
| `pardusdb_get_schema` | Show table schema and structure |
| `pardusdb_import_status` | View or manage import history |

### OpenCode Configuration

Add to your `opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "pardusdb": {
      "type": "local",
      "command": ["python3", "/home/${USER}/.pardus/mcp/server.py"],
      "enabled": true
    }
  }
}
```

Adjust the path to match your installation. Tools are automatically available to the LLM.

## SDKs

### Python SDK

```bash
pip install -e sdk/python
```

```python
from pardusdb import PardusDB

client = PardusDB()
client.create_table("docs", vector_dim=768, metadata_schema={"content": "TEXT"})
client.insert("docs", [0.1, 0.2, ...], {"content": "Hello"})
results = client.search("docs", [0.1, 0.2, ...], k=10)
```



## Benchmarks

For detailed benchmarks, see [BENCHMARKS.md](BENCHMARKS.md).

### Performance Summary (Apple Silicon M-series)

| Operation | Time |
|-----------|------|
| Single insert | ~160 µs/doc |
| Batch insert (1,000 docs) | ~6 ms |
| Query (k=10) | ~3 µs |

### Speed Comparison

| vs Neo4j | PardusDB Advantage |
|----------|-------------------|
| Insert | **1983x faster** |
| Search | **431x faster** |

| vs HelixDB | PardusDB Advantage |
|------------|-------------------|
| Insert | **200x faster** |
| Search | **62x faster** |

| Batch Size | Speedup vs Individual |
|------------|----------------------|
| 100 | 45x |
| 500 | 149x |
| 1000 | **220x** |

## Examples

### Rust

```bash
cargo run --example simple_rag --release
```

### Python

```bash
cd examples/python
pip install requests
python simple_rag.py
```

## Why We Built PardusDB

The Pardus AI team built PardusDB because we believe private, local-first AI tools should be accessible to everyone — from individual developers to large teams.

PardusDB gives you the low-level building block for fast, private vector search, while [Pardus AI](https://pardusai.org/) delivers the high-level no-code experience for analysts, marketers, and business users who just want answers from their data.

If you enjoy working with PardusDB, we'd love for you to try [Pardus AI](https://pardusai.org/) — upload your spreadsheets or documents and ask questions in plain English. Free tier available, no credit card required.

## License

MIT License — use it freely in personal and commercial projects.

---

⭐ Star us on GitHub if you find this useful!
🚀 Building something cool with PardusDB? Share it with us on X or Discord — we'd love to hear from you.

**Pardus AI** — https://pardusai.org/
