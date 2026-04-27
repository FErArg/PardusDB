# Changelog

All notable changes to PardusDB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.7] - 2026-04-28

### Added

- **Project initialization tool**: New `pardusdb_init_project` MCP tool that creates a new `.pardus` database, auto-detects embedding dimension via `sentence-transformers`, creates a default table with schema, inserts a test vector, and runs a search verification. Returns a health summary of the setup.
- **Database test tool**: New `pardusdb_test_database` MCP tool that runs a quick insert-and-search test on an existing database to verify read/write operations work correctly, with optional cleanup of test records.

## [0.4.6] - 2026-04-28

### Fixed

- **MCP read operations always recreated demo table**: Removed `demo_operations()` from `run_with_file()` in `src/main.rs`, which was called on every file open. Since the MCP server spawns a new `pardusdb` process per operation, every read query (SELECT, SHOW TABLES) would recreate the "documents" table and insert 100 demo rows, polluting the database.

## [0.4.5] - 2026-04-28

### Added

- **Document dependency installer**: New `install_document_dependencies()` function in `setup.sh` and `install.sh` that installs pypdf, python-docx, openpyxl, and xlrd for full document import support.
- **sentence-transformers installer**: New `install_sentence_transformers()` function in both installers for automatic embeddings during document import. Installs the `all-MiniLM-L6-v2` model (~80MB).
- **Optional dependencies**: Both functions are optional — users choose whether to install heavy dependencies like sentence-transformers.

## [0.4.4] - 2026-04-28

### Fixed

- **SQL string escape in import**: Fixed `SyntaxError` when importing documents with single quotes or newlines in content. Replaced nested escaped quotes with a clean `content_esc` variable that properly escapes `'` → `''` and strips newlines.

## [0.4.3] - 2026-04-28

### Added

- **Document import tool**: New MCP tool `pardusdb_import_text` scans a directory and imports documents (PDF, CSV, DOCX, XLSX, JSON, JSONL, MD, TXT) with automatic embeddings via `sentence-transformers` (all-MiniLM-L6-v2). Falls back to zero vectors if the library is not installed.
- **Parent-child tracking**: Multi-page/file documents create one parent entry plus individual child fragments linked via `parent_doc_id`, with `chunk_index`, `total_chunks`, `page`, and `title`.
- **Deduplication**: Import skips files already imported (detected via SHA256 hash and `__import_log__` table).
- **Import history**: Internal `__import_log__` table tracks all import operations with file hash, size, timestamp, and status. View or reset with `pardusdb_import_status`.
- **Health check tool**: `pardusdb_health_check` verifies table integrity, orphan detection, schema validation, dimension consistency, and duplicate tracking.
- **Schema introspection**: `pardusdb_get_schema` shows table columns, types, and statistics.
- **Auto-create table**: The import tool creates the target table automatically if it doesn't exist.
- **File size limit**: Configurable `max_file_size_mb` (default 50) prevents oversized file imports.
- **Progress reporting**: Import progress printed to stderr every 5 files during large imports.
- **Granular error handling**: Individual file errors don't abort the entire import batch.
- **Multi-format parsing**: Built-in parsers for TXT, MD, CSV, JSON, JSONL, plus optional `pypdf` (PDF), `python-docx` (DOCX), and `openpyxl` (XLSX).

## [0.4.1] - 2026-04-28

### Fixed

- **OpenCode config extension**: Renamed `opencode.jsonc` → `opencode.json` in installers and documentation to match the actual file format used by OpenCode.
- **pip install mcp fallback**: Added `--break-system-packages` fallback for systems where pip blocks global installations (Debian/Ubuntu with Python 3.11+ externally managed environments).

### Changed

- Updated README.md and INSTALL.md to reference `opencode.json` instead of `opencode.jsonc`.

## [0.4.0] - 2026-04-28

### Changed

- **MCP server migrated from TypeScript/Node.js to Python**: The MCP server is now implemented in Python (`mcp/src/server.py`) instead of TypeScript/Node.js. This eliminates Node.js as a prerequisite — only Python 3.10+ is required for the MCP server.
- **Simplified installation**: Installers no longer use `npm` — the MCP server is a single Python file that runs directly.

### Added

- **Python MCP dependency**: Optional `mcp` package added to Python SDK for running the MCP server: `pip install pardusdb[mcp]`

### Removed

- **Node.js dependency**: Node.js 18+ is no longer required for basic PardusDB installation (only needed for TypeScript SDK)
- **npm**: No longer used during installation

## [0.3.0] - 2026-04-28

### Added

- **OpenCode MCP auto-configuration**: Both `setup.sh` and `install.sh` now ask if you want to configure PardusDB MCP for OpenCode. If you answer yes, the installer automatically adds the MCP server entry to `~/.config/opencode/opencode.jsonc` with the correct user path (`/home/${USER}/.pardus/mcp/dist/index.js`).
- **OpenCode skill installation**: Installers automatically copy `skill/skill.md` to `~/.config/opencode/skills/pardusdb.md`, making PardusDB knowledge available to OpenCode agents.
- **Precompiled binary**: `bin/pardus-v0.3.0` is included with each release for use with `install.sh` (fast installation without Rust compilation).

## [0.2.2] - 2026-04-27

### Changed

- **Default database name**: `data.pardus` → `pardus-rag.db` (more descriptive name)
- **MCP path placeholder**: `YOUR_USER` → `${USER}` in docs for clarity

## [0.2.1] - 2026-04-27

### Changed

- **Data directory**: `~/.local/share/pardus/` → `~/.pardus/` (XDG-compliant, simpler path)
- **Config directory**: New `~/.config/pardus/` for configuration files
- **Platform detection**: `setup.sh` now detects Linux vs macOS
- **Shell detection**: `setup.sh` auto-detects bash/zsh/fish for PATH instructions
- **Auto-create database**: Default database `~/.pardus/pardus-rag.db` is now created automatically after installation completes
- **Config file**: Installer creates `~/.config/pardus/config.toml` with default settings

### Added

- **install.sh**: New lightweight installer that uses precompiled binaries from `bin/pardus-vX.X.X`, skips Rust compilation
- **Versioned binaries**: After compilation, `setup.sh` saves the binary to `bin/pardus-v0.2.1` for use with `install.sh`
- **Rust auto-install**: `setup.sh` automatically installs Rust via `rustup` if `cargo` is not found

## [0.2.0] - 2026-04-27

### Added

- **Helper script `pardus`**: New convenience script that auto-creates the default database at `~/.local/share/pardus/data.pardus` if it doesn't exist when run without arguments.
- **Python SDK**: Full Python SDK installed via `pip install -e sdk/python`. Provides a Python client class to interact with PardusDB programmatically.
- **TypeScript SDK**: TypeScript SDK at `sdk/typescript/pardusdb/` with npm installation. Includes type definitions and full API coverage.
- **MCP Server**: Model Context Protocol server at `mcp/` for integration with AI agents (OpenCode, Claude Desktop, etc.). Exposes 10 tools for database operations.
- **Unified installation**: `setup.sh` now installs all components (binary, helper, MCP server, Python SDK, TypeScript SDK) in a single command.
- **Collaborators section**: README now acknowledges project contributors (FErArg, Deepseek, Miramax, Kimi).

### Changed

- **Binary location**: Binaries now always install to `~/.local/bin/` (no longer tries `/usr/local/bin` with sudo fallback). This follows the XDG Base Directory Specification.
- **Data directory**: Database files default to `~/.local/share/pardus/` instead of the current working directory.
- **Version bump**: All packages bumped from `0.1.0` to `0.2.0` to reflect the new SDK and MCP features.

### Deprecated

- **In-memory REPL**: Running `pardusdb` without arguments now automatically opens or creates the default database instead of starting an empty in-memory session.

### Removed

- **Sudo-based installation**: The installer no longer attempts to use sudo for system-wide installation.

## [0.1.0] - 2026-04-26

### Added

- Initial release of PardusDB
- HNSW-based vector similarity search
- SQL-like query syntax (CREATE, INSERT, SELECT, UPDATE, DELETE)
- Single-file storage (.pardus files)
- Multiple tables per database
- UNIQUE constraints with O(1) duplicate detection
- GROUP BY with aggregate functions (COUNT, SUM, AVG, MIN, MAX)
- JOIN support (INNER, LEFT, RIGHT) with O(n+m) hash join algorithm
- Thread-safe concurrent reads via `RwLock`
- REPL interface with `.create`, `.open`, `.save`, `.tables` commands
- Optional GPU acceleration via wgpu
- Benchmarks vs Neo4j and HelixDB