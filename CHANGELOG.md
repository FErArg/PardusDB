# Changelog

All notable changes to PardusDB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.18] - 2026-05-01

### Fixed

- **IndentationError in `get_table_schema`** (`mcp/src/server.py`): Fixed 4-space over-indent on line that sets `schema["row_count"]` inside the try block. The error caused the MCP server to crash with "unexpected indent" when calling any tool that accessed table metadata.

## [0.4.17] - 2026-05-01

### Changed

- **Bumped version** to 0.4.17 across Cargo.toml, setup.sh, install.sh, install-macos.sh, SDKs, and MCP server.

### Security

- **Path traversal fix in MCP server**: Added symlink resolution and directory boundary validation in `handle_import_text` to prevent reading files outside the target directory via malicious symlinks.

### Fixed

- **Regex match verification in MCP server**: Added `parse_count_from_result()` helper function to safely handle regex operations that could previously crash on malformed output. Fixed 6+ locations where `m.group()` was called without verifying `m` was not None.

### Performance

- **Python SDK `_execute()` optimization**: Eliminated unnecessary temporary file creation by piping commands directly to subprocess stdin. Removed unused `tempfile` and `json` imports.
- **Python SDK `insert_batch()` optimization**: Rewrote to generate single multi-VALUES SQL statement instead of N individual inserts, reducing N subprocess calls to 1.
- **Rust `graph.rs` optimization**: Changed `to_prune` from `Vec` to `HashSet` for O(1) membership testing instead of O(n) linear search.

### Added

- **New binary**: `bin/pardus-v0.4.17` (Linux x86_64).

## [0.4.16] - 2026-05-01

### Changed

- **Bumped version** to 0.4.16 across Cargo.toml, setup.sh, install.sh, install-macos.sh, SDKs, and MCP server.
- **Updated repository URL**: Changed from `https://github.com/pardus-ai/pardusdb` to `https://github.com/FErArg/PardusDB` across all documentation and configuration files.

### Fixed

- **skill.md reescrito**: Corregida información desactualizada con:
  - URL correcta del repositorio
  - Comandos REPL correctos (`.create`, `.open`, `.save`, `.tables`, `.clear`, `.cls`)
  - Alias documentados (`quit` / `exit` / `q`)
  - API Rust correcta (`execute`, `query`)
  - Python SDK documentado (`create_table`, `insert`, `search`)
  - 15 herramientas MCP completas incluyendo `pardusdb_import_status`

### Added

- **`pardusdb_import_status`**: Nueva herramienta MCP para ver y reiniciar historial de importaciones.
- **Nuevo binario compilado**: `bin/pardus-v0.4.16` (Linux x86_64).

## [0.4.15] - 2026-04-28

### Changed

- **Bumped version** to 0.4.15 across Cargo.toml, setup.sh, install.sh, SDKs, and MCP server.
- **Platform-specific binaries**: Binaries are now suffixed with platform and architecture. Linux x86_64: `bin/pardus-v{VERSION}-linux-x86_64`, macOS ARM64: `bin/pardus-v{VERSION}-darwin-arm64`. `setup.sh` saves with the correct suffix for the current platform.
- **install.sh**: Now searches for `bin/pardus-v{VERSION}-linux-x86_64` (Linux precompiled binary).
- **install-macos.sh**: Now searches for `bin/pardus-v{VERSION}-darwin-arm64` (macOS precompiled binary). Shows a clear error if the macOS binary is not found: "Compile on your Mac with cargo build --release and copy to bin/pardus-v{VERSION}-darwin-arm64".

### Fixed

- **import_text auto-creates table**: `handle_import_text` now unconditionally calls `ensure_import_table()` before importing, instead of using a fragile SELECT-based existence check. `CREATE TABLE IF NOT EXISTS` is idempotent, so this is safe even if the table already exists.
- **sentence-transformers inside venv**: On macOS, `sentence-transformers` is now installed inside `~/.pardus/mcp/venv/` instead of globally. The MCP server runs inside the venv, so dependencies must be available there for embeddings to work.

## [0.4.14] - 2026-04-28

### Changed

- **Bumped version** to 0.4.14 across Cargo.toml, setup.sh, install.sh, SDKs, and MCP server.
- **MCP package verification**: `install_mcp()` now verifies the `mcp` Python package was installed correctly, displaying OK/fallo status like other dependencies.
- **Repository cleanup**: Added `.gitignore` (target/, *.pyc, __pycache__/, *.pardus). Removed orphaned files: nodesource_setup.sh, test_mcp_tools.py, unrelated docs (CELEX PDF, BOE summary), accidentally committed --version binary and __pycache__ bytecode.
- **Docs**: Revised README.md and INSTALL.md with clearer setup.sh vs install.sh comparison tables.
- **macOS installer**: Replaced `install-macos.sh` with a venv-based MCP installation. Now uses the precompiled binary from `bin/pardus-v0.4.14` (no Rust compilation). Installs MCP inside `~/.pardus/mcp/venv/` with a wrapper script `run_mcp.sh` for OpenCode integration. Removed Rust as a prerequisite — only Python 3 and pip are required.
- **macOS Python 3.10+ auto-install**: `install-macos.sh` now detects if Python < 3.10 is in use (common on macOS 26 which ships 3.9) and offers to install Python 3.13 via Homebrew automatically. If Homebrew is not available, displays clear installation instructions and exits rather than silently failing the MCP install.

## [0.4.13] - 2026-04-28

### Changed

- **Bumped version** to 0.4.13 across Cargo.toml, setup.sh, install.sh, SDKs, and MCP server.
- **Removed all Node.js references** from README.md, INSTALL.md, mcp/README.md, AGENTS.md. Node.js is not required to build or use PardusDB.

## [0.4.12] - 2026-04-28

### Fixed

- **Persistence bug (critical)**: `run_with_file()` previously opened the database and immediately saved/closed it without processing any commands. Every MCP call spawned a new process that opened the file, ran zero SQL commands, and exited without persisting. Now `run_with_file()` enters a REPL-like loop reading from stdin — processes SQL commands, calls `save()` on quit (`.quit`, `quit`, `exit`), ensuring all changes persist to disk. Data now survives close/reopen cycles.

### Changed

- `run_with_file()` now reads stdin line-by-line and executes commands until `quit`, matching REPL behavior with auto-save on exit.

## [0.4.11] - 2026-04-28

### Fixed

- **returncode validation**: `db_client.execute()` now checks `proc.returncode != 0` and prefixes errors with exit code, so failures no longer silently pass as success.
- **SQL injection hardening**: Added `sql_escape()` and `sql_safe_identifier()` helpers. Applied to all table names, column names, paths, and string values in INSERT/SELECT queries across all MCP tool handlers.
- **Embedder error propagation**: When `embedder.encode()` fails, the file now gets logged as `embedder_failed` status and the error is reported in the import summary instead of being silently swallowed.

### Security

- `table` and `column` identifiers now validated via `sql_safe_identifier()` (alphanumeric + underscore only)
- String values now escaped via `sql_escape()` (single-quote doubling)
- DB paths in `.create`/`.open` commands now quoted via `shlex.quote()`

## [0.4.10] - 2026-04-28

### Removed

- **Dead code cleanup**: Removed `demo_operations()` function and its `use std::time::Instant` import from `main.rs`. The function was never meant for production and had 8 `.unwrap()` calls that could panic. This eliminates any risk of future regression if the function were accidentally re-enabled.

## [0.4.9] - 2026-04-28

### Fixed

- **Critical bug**: `demo_operations()` was called on every `pardusdb <path>` invocation, attempting to `CREATE TABLE documents` without `IF NOT EXISTS`, causing all database operations to crash if the table already existed. Removed the automatic demo table creation from `run_with_file()`.

## [0.4.8] - 2026-04-28

### Added

- **Content hash dedup**: Duplicate documents are now detected not only by file hash but also by content hash. If the same content exists in different files, it will be skipped during import.

## [0.4.7] - 2026-04-28

### Fixed

- **Content truncation removed**: Document content was being truncated to 5000 characters before inserting. Now inserts the full content of each document.
- **Batch embeddings**: All page/chunk embeddings for a file are now generated in a single `embedder.encode()` batch call instead of one call per fragment. For a 50-page PDF: ~51 calls → 1 call.
- **Parent embedding dropped**: Parent records now get a zero vector. Only child records (actual page/paragraph chunks) receive real embeddings, making similarity searches more accurate.

### Changed

- Optimized import pipeline for multi-page documents (PDF, DOCX) — significant speedup and better vector quality.

## [0.4.6] - 2026-04-28

### Fixed

- **Coroutine bug in MCP execute()**: `PardusDBClient.execute()` was an `async def` method using `asyncio.create_subprocess_exec`, but called without `await` from 15+ synchronous helper functions. Every call returned an unresolved coroutine object instead of the SQL result string, causing `expected string or bytes-like object, got 'coroutine'` errors during document import. Fixed by switching to synchronous `subprocess.run()`.

### Added

- **XLS parser**: New `parse_xls()` function using `xlrd` for legacy Excel (.xls) support. Registered in `PARSERS`.
- **Lazy model loading**: `SentenceTransformer` model is now loaded on first embedding generation instead of at module import time.

### Changed

- **Model cache check in installers**: `install_sentence_transformers()` now checks `~/.cache/huggingface/hub/` for the cached model instead of forcing a download during installation.

## [0.4.5] - 2026-04-28

### Added

- **Document dependency installer**: New `install_document_dependencies()` function in `setup.sh` and `install.sh` that optionally installs pypdf (PDF), python-docx (DOCX), openpyxl (XLSX), and xlrd (XLS) for full document import support. Interactive prompt (s/N).
- **sentence-transformers installer**: New `install_sentence_transformers()` function in both installers for automatic embeddings during document import. Installs the `all-MiniLM-L6-v2` model (~80MB). Interactive prompt (s/N).
- **Optional dependencies**: Both functions are optional — users choose whether to install heavy dependencies like sentence-transformers.
- **Dependency verification**: Installation summary now checks and reports the status of each optional Python package (pypdf, docx, openpyxl, xlrd, sentence-transformers).

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