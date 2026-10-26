# Installation Guide

Complete installation and configuration guide for PardusDB.

## Requirements

- **Rust** (for building from source) — install from https://rustup.rs/
- **Python 3.10+** (for Python SDK and MCP server) — install from https://python.org/

## Quick Install

Multiple installers are provided. Use the macOS-specific scripts on macOS so the MCP Python package is installed inside a compatible virtual environment.

### Components Installed by Both

| Component | Path |
|-----------|------|
| `pardusdb` binary | `~/.local/bin/pardusdb` |
| `pardus` helper script | `~/.local/bin/pardus` |
| MCP server | `~/.pardus/mcp/` |
| Config file | `~/.config/pardus/config.toml` |
| Data directory | `~/.pardus/` |
| Python SDK | (pip-installed from source) |

The `pardus` helper manages the default database at `~/.pardus/pardus-rag.db`. Some installers create it during installation; `setup-macos.sh` only creates `~/.pardus/` and defers database creation until the first `pardus` run.

---

### Option 1: setup.sh — Build from source on Linux (requires Rust)

```bash
git clone https://github.com/FErArg/pardus-rag-ng
cd pardusdb
./setup.sh --install
```

**What it does:**
1. Checks for Rust/Cargo (installs via rustup.rs if missing)
2. Builds from source for the current platform
3. Compiles `pardusdb` with `cargo build --release`
4. Saves the compiled binary to `bin/pardus-v0.4.33-{platform}-{arch}` for future use
5. Installs the binary, helper, MCP server, config, Python SDK
6. Creates default database

**Use this when:** You want the latest source code on Linux, have modified Rust files, or are setting up a Linux development environment. **On macOS, use `setup-macos.sh` instead.**

---

### Option 2: install.sh — Use precompiled binary (no Rust)

```bash
git clone https://github.com/FErArg/pardus-rag-ng
cd pardusdb
./install.sh --install
```

**What it does:**
1. Copies the precompiled binary from `bin/pardus-v0.4.33-linux-x86_64` to `~/.local/bin/pardusdb`
2. Installs the helper, MCP server, config, Python SDK
3. Creates default database

**Important:** `install.sh` does **not** compile anything. It needs a pre-existing binary at `bin/pardus-v0.4.33-linux-x86_64`. If you modified Rust source code, run `cargo build --release` first or use `setup.sh`.

**Use this when:** You just want to install quickly, don't have Rust, or are deploying from a release tarball.

---

### Option 3: install-macos.sh — macOS with virtual environment (auto-installs Python 3.10+ if needed)

```bash
git clone https://github.com/FErArg/pardus-rag-ng
cd pardusdb
./install-macos.sh --install
```

**What it does:**
1. Checks for Python 3.10+ (required by the `mcp` Python package)
2. If Python < 3.10 is detected and Homebrew is available: offers to install Python 3.13 via `brew install python@3.13`
3. If Python < 3.10 and no Homebrew: shows instructions and exits
4. Requires `bin/pardus-v0.4.33-darwin-arm64` in the repo — if missing, **use `./setup-macos.sh --install` instead** (compiles on your Mac)
5. Copies the precompiled binary to `~/.local/bin/pardusdb`
6. Installs the helper, MCP server, config
7. Creates a Python virtual environment at `~/.pardus/mcp/venv/`
8. Installs the `mcp` package inside the venv
9. Generates a wrapper script `~/.pardus/mcp/run_pardusdb_mcp.sh`
10. Creates default database

**Why a virtual environment?** Isolates the MCP Python package and all its dependencies (including `sentence-transformers` for embeddings) from system packages. Works alongside macOS system Python 3.9 without conflicts.

**MCP in OpenCode:** The wrapper script is used as the command in `opencode.json`:
```json
{
  "mcp": {
    "pardusdb": {
      "type": "local",
      "command": ["$HOME/.pardus/mcp/run_pardusdb_mcp.sh"],
      "enabled": true
    }
  }
}
```

**Use this when:** You are on macOS, want a fast setup without Rust, and need a clean MCP installation.

---

### Option 4: setup-macos.sh — macOS build from source with virtual environment

```bash
git clone https://github.com/FErArg/pardus-rag-ng
cd pardusdb
./setup-macos.sh --install
```

**What it does:**
1. Verifies the system is macOS
2. Checks for Rust/Cargo and installs Rust via rustup.rs if missing
3. Checks for Python 3.10+ for the `mcp` Python package
4. If Python < 3.10 is detected and Homebrew is available: offers to install Python 3.13 via `brew install python@3.13`
5. Compiles `pardusdb` with `cargo build --release`
6. Saves the compiled binary to `bin/pardus-v0.4.33-darwin-arm64`
7. Installs the binary, helper, config, and Python SDK
8. Creates a Python virtual environment at `~/.pardus/mcp/venv/`
9. Installs and verifies the `mcp` package inside that venv
10. Installs document import dependencies in the same venv
11. Creates `~/.pardus/` but does not initialize `~/.pardus/pardus-rag.db`; the helper creates it on first `pardus` use

**Use this when:** You are on macOS and need to build from the current source tree, or when no precompiled macOS binary exists.

---

### Comparison

| | setup.sh | install.sh | setup-macos.sh | install-macos.sh |
|---|---|---|---|---|
| Requires Rust | Yes | No | Yes | No |
| Requires Python 3.10+ for MCP | No | No | **Yes (auto-installed via Homebrew)** | **Yes (auto-installed via Homebrew)** |
| Compiles source | Yes | No | Yes | No |
| Takes binary from | source build | `bin/pardus-v*-linux-x86_64` | source build | `bin/pardus-v*-darwin-arm64` |
| Writes binary to `bin/` | Yes | No | Yes | No |
| MCP installation | global pip | global pip | **virtual environment** | **virtual environment** |
| Linux | Yes | Yes | Not supported | Not supported |
| macOS (Apple Silicon) | Not recommended | No | **Yes (recommended)** | Yes (if binary exists) |
| Speed | ~1-3 min | <1 second | ~1-3 min + Python setup | <1 second + Python setup |
| MCP server, SDK, config | Same | Same | Same | Same |

---

## Cross-Platform Compatibility

PardusDB supports Linux and macOS. The installer automatically detects your platform.

### Linux

The installer uses XDG Base Directory Specification:
- Data: `~/.pardus/`
- Config: `~/.config/pardus/`

### macOS

Same structure as Linux:
- Data: `~/.pardus/`
- Config: `~/.config/pardus/`

On macOS, ensure `~/.local/bin` is in your PATH (zsh is default on recent macOS versions):

```bash
echo 'export PATH="$PATH:$HOME/.local/bin"' >> ~/.zshrc
source ~/.zshrc
```

**Recommended installer on macOS:** If you have Rust installed (or don't mind installing it), use `./setup-macos.sh --install`. It compiles the binary for your Mac and installs MCP in a Python 3.10+ virtual environment. If you prefer not to install Rust, use `install-macos.sh` but you must first ensure `bin/pardus-v0.4.33-darwin-arm64` exists in the repo.

---

## After Installation

### Add to PATH (if not already)

Bash:
```bash
echo 'export PATH="$PATH:$HOME/.local/bin"' >> ~/.bashrc
source ~/.bashrc
```

Zsh:
```bash
echo 'export PATH="$PATH:$HOME/.local/bin"' >> ~/.zshrc
source ~/.zshrc
```

Fish:
```bash
fish_add_path "$HOME/.local/bin"
```

### Verify Installation

```bash
pardusdb --version
```

---

## Usage

### Using the Helper (Recommended)

The `pardus` helper automatically manages the default database:

```bash
pardus                    # Opens ~/.pardus/pardus-rag.db (creates if missing)
pardus mi.db              # Opens specific file
```

### Using the Binary Directly

```bash
pardusdb                          # In-memory session (no persistence)
pardusdb ~/.pardus/mi.db          # Open specific file
```

### REPL Commands

| Command | Description |
|---------|-------------|
| `.create <file>` | Create and open a new database |
| `.open <file>` | Open an existing database |
| `.save` | Force save current database |
| `.tables` | List all tables |
| `.clear` | Clear screen |
| `help` | Show help |
| `quit` | Exit (auto-saves if file open) |

### SQL Quick Reference

```sql
-- Create a table
CREATE TABLE docs (embedding VECTOR(768), content TEXT, score FLOAT);

-- Insert data
INSERT INTO docs (embedding, content, score)
VALUES ([0.1, 0.2, ...], 'Hello World', 0.95);

-- Search by similarity
SELECT * FROM docs WHERE embedding SIMILARITY [0.1, 0.2, ...] LIMIT 10;

-- Filtered search
SELECT * FROM docs WHERE content LIKE '%hello%' LIMIT 10;

-- Update
UPDATE docs SET score = 0.99 WHERE id = 1;

-- Delete
DELETE FROM docs WHERE id = 1;
```

---

## Configuration

PardusDB uses a configuration file at `~/.config/pardus/config.toml`:

```toml
[database]
default_path = "~/.pardus/pardus-rag.db"

[logging]
level = "info"
```

This file is created automatically by the installer.

---

## MCP Server for AI Agents

PardusDB includes an MCP (Model Context Protocol) server that allows AI agents to interact with the database.

### MCP Tools Available

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

### MCP Document Ingestion Paths

`pardusdb_import_text`, `pardusdb_ingest_chunked`, and `pardusdb_ingest_async` resolve paths safely for project-local ingestion. Absolute paths are accepted as explicit paths, including locations outside the project. Relative paths are always confined to the MCP process project root; if they do not exist directly under that root, PardusDB searches recursively inside the root and uses the single matching file or directory. Escapes such as `../` and symlinks pointing outside the root are rejected. If multiple matches exist, provide a more specific relative path.

### Configuring MCP in OpenCode

Add this to your OpenCode configuration file (`~/.config/opencode/opencode.json` or `./opencode.json`):

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "pardusdb": {
      "type": "local",
      "command": ["/home/${USER}/.pardus/mcp/run_pardusdb_mcp.sh"],
      "enabled": true
    }
  }
}
```

Adjust the path to match your installation.

### OpenCode MCP Auto-Configuration

During installation, `setup.sh`, `install.sh`, `setup-macos.sh`, and `install-macos.sh` can ask if you want to configure PardusDB MCP for OpenCode automatically. If you answer yes, the installer adds the MCP server entry to your OpenCode config.

After installation completes, restart OpenCode to load the new MCP tools.

### Using MCP in Claude Desktop

Add to `~/.claude.json`:

```json
{
  "mcpServers": {
    "pardusdb": {
      "command": "python3",
      "args": ["/home/${USER}/.pardus/mcp/server.py"]
    }
  }
}
```

### Example Usage

Once configured, you can use natural language like:

```
Create a table called articles with 384-dimensional embeddings and a title text column.
```

The agent will use `pardusdb_create_table` automatically.

### Document Import with `pardusdb_import_text`

Imports files from a directory and stores them as vector-searchable documents with automatic embeddings.
Multi-page files (PDF) and multi-paragraph files (DOCX) create one parent document plus one child fragment per page/paragraph.
Tabular files (CSV, XLSX) create one parent plus one child per row.

**Supported formats:** PDF, CSV, DOCX, XLSX, XLS, JSON, JSONL, MD, TXT

**Path resolution:** Relative import paths are project-local. For example, `manual.pdf` can be found anywhere under the current MCP project root if it has a unique match. Use an absolute path when intentionally ingesting a file outside the project root.

**Parent-child structure:** Each imported file creates a parent record (`page=0`, `parent_doc_id=NULL`) plus one child record per page/paragraph/row, linked via `parent_doc_id`. Additional fields track the relationship: `chunk_index`, `total_chunks`, `filename`, `title`, and `doc_path`.

**Optional dependencies for specific formats:**

```bash
pip install sentence-transformers  # automatic embeddings (recommended, all-MiniLM-L6-v2, 384-dim)
pip install pypdf                  # PDF support
pip install python-docx            # DOCX support
pip install openpyxl               # XLSX support
pip install xlrd                   # XLS (Excel 97-2003) support
```

If `sentence-transformers` is not installed, vectors are stored as zeros. If a format library is missing, files of that type are skipped with a warning.

**Optional dependency installers:**

The setup/install scripts include optional steps to install these dependencies interactively:
- `install_document_dependencies()` — Installs pypdf, python-docx, openpyxl, xlrd for document parsing
- `install_sentence_transformers()` — Installs sentence-transformers for automatic embeddings (model `all-MiniLM-L6-v2`, ~80MB, requires confirmation interactiva)

**Usage examples:**

```
Import all documents from /home/user/project/docs into a table called articles.
Import only PDF and TXT files from /home/user/docs, table name: documents.
Import from /data, table: knowledge_base, max file size: 100MB.
```

---

## Backup

PardusDB stores all data in a single `.pardus` file. To backup:

```bash
# Simple copy (while database is not in use)
cp ~/.pardus/pardus-rag.db ~/.pardus/data-backup-$(date +%Y%m%d).pardus

# Or with the REPL
pardus
> .save
> quit
cp ~/.pardus/pardus-rag.db /path/to/backup/
```

### Restore

```bash
cp /path/to/backup/data.pardus ~/.pardus/pardus-rag.db
pardus
```

---

## Uninstall

Installers support `--uninstall`. Run the matching installer from the repo root:

```bash
cd /path/to/pardusdb
./setup.sh --uninstall
# or: ./install.sh --uninstall
# macOS source install: ./setup-macos.sh --uninstall
# macOS prebuilt install: ./install-macos.sh --uninstall
```

This removes:
- `~/.local/bin/pardusdb`
- `~/.local/bin/pardus`
- `~/.pardus/` (including all database files)
- `~/.config/pardus/` (configuration)
- Python SDK (`pip uninstall pardusdb`)

---

## SDKs

### Python SDK

```bash
pip install -e /path/to/pardusdb/sdk/python
```

```python
from pardusdb import PardusDB

client = PardusDB()
client.create_table("docs", vector_dim=768, metadata_schema={"content": "TEXT"})
client.insert("docs", [0.1, 0.2, ...], {"content": "Hello"})
results = client.search("docs", [0.1, 0.2, ...], k=10)
```

See `sdk/python/README.md` for full documentation.



---

## Troubleshooting

### "command not found: pardus"

Make sure `~/.local/bin` is in your PATH:
```bash
export PATH="$PATH:$HOME/.local/bin"
```

### "Error: Database path is required"

When using the MCP server, make sure you open a database first:
```bash
pardusdb_create_database with path="~/.pardus/mydb.pardus"
```



### Python SDK import fails

```bash
pip install --upgrade pip
pip install -e /path/to/pardusdb/sdk/python
```

### MCP server not responding

Make sure the binary `pardusdb` is in your PATH and can be executed standalone:
```bash
which pardusdb
pardusdb --version
```

---

## File Locations Summary

| Component | Path |
|-----------|------|
| Binary | `~/.local/bin/pardusdb` |
| Helper script | `~/.local/bin/pardus` |
| Data directory | `~/.pardus/` |
| Default DB | `~/.pardus/pardus-rag.db` |
| MCP server | `~/.pardus/mcp/` |
| Config file | `~/.config/pardus/config.toml` |
| Python SDK | (installed via pip) |
