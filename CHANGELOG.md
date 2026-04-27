# Changelog

All notable changes to PardusDB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-04-27

### Changed

- **Data directory**: `~/.local/share/pardus/` → `~/.pardus/` (XDG-compliant, simpler path)
- **Config directory**: New `~/.config/pardus/` for configuration files
- **Platform detection**: `setup.sh` now detects Linux vs macOS
- **Shell detection**: `setup.sh` auto-detects bash/zsh/fish for PATH instructions
- **Auto-create database**: Default database `~/.pardus/data.pardus` is now created automatically after installation completes
- **Config file**: Installer creates `~/.config/pardus/config.toml` with default settings

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