# AGENTS.md - PardusDB

## Repo Shape

- Root crate is Rust package `pardusdb` (`Cargo.toml`, edition 2024). Library entrypoint is `src/lib.rs`; CLI/REPL entrypoint is `src/main.rs`.
- SQL parsing lives in `src/parser.rs`; command execution and file persistence live in `src/database.rs`; graph/vector internals are under `src/graph.rs`, `src/table.rs`, `src/distance.rs`, and `src/concurrent.rs`.
- MCP server is a separate Python stdio server in `mcp/src/server.py`; it shells out to `pardusdb`, so it needs the Rust binary in `PATH`.
- SDKs are separate packages: Python in `sdk/python`, TypeScript in `sdk/typescript/pardusdb`. Both SDK test suites also shell out to `pardusdb` from `PATH`.

## Core Commands

- Rust build: `cargo build`; release binary: `cargo build --release`.
- Rust tests: `cargo test`; focused Rust test: `cargo test test_name` or `cargo test --test database_test test_similarity_search`.
- Run CLI from source: `cargo run --` for in-memory REPL, or `cargo run -- path/to/db.pardus` to open/create and auto-save that file.
- GPU code is behind the optional feature: `cargo build --features gpu`.
- Python SDK setup/test: `pip install -e sdk/python`, then from `sdk/python`: `python -m pytest`; focused test: `python -m pytest tests/test_pardusdb.py::TestConnection::test_create_file_database`.
- TypeScript SDK commands from `sdk/typescript/pardusdb`: `npm install`, `npm run build`, `npm test`; focused Jest test: `npx jest src/index.test.ts -t "should create a table"`.
- Root security/CLI tests require installed `pardusdb` in `PATH`: `python3 test/security_test.py` without pytest, or `python -m pytest test/`.

## Runtime Gotchas

- `Database::open(path)` creates a new `.pardus` file if it does not exist; in-memory databases silently no-op on `save()`.
- CLI meta commands accept both dotted and plain forms for common commands (`.tables`/`tables`, `.save`/`save`, `quit`/`.quit`). With a file argument, EOF or `quit` triggers save.
- The `pardus` helper installed by scripts opens/creates `~/.pardus/pardus-rag.db`; raw `pardusdb` with no args starts an in-memory REPL unless `.open`/`.create` is used.
- `setup.sh --install` is the Linux/general source installer; use `setup-macos.sh --install` on macOS so MCP gets a Python 3.10+ venv and an installed `mcp` package. `install.sh` and `install-macos.sh` expect prebuilt binaries in `bin/`.
- Installer `--uninstall` paths remove installed files and `~/.pardus/`; do not run uninstall casually because that directory contains user databases and MCP stats.

## MCP Notes

- MCP default vector dimension is `384` (`all-MiniLM-L6-v2`). If `sentence-transformers` is missing or model load fails, automatic embeddings degrade to zero vectors.
- MCP auto-discovers `./database.pardus` in the current working directory; otherwise the first command creates/uses `database.pardus` there unless a tool opens another path.
- MCP document import supports `.txt`, `.md`, `.csv`, `.pdf`, `.docx`, `.xlsx`, `.xls`, `.json`, `.jsonl`; PDF/DOCX/XLSX/XLS support depends on optional Python packages (`pypdf`, `python-docx`, `openpyxl`, `xlrd`).
- MCP import/chunking writes converted Markdown under relative `./tmp/{uuid}/`; successful imports clean it up, errors can leave tmp directories for debugging.
- MCP tool handlers are all in one process with a singleton `db_client`; changing the active database/table affects later tool calls in that server session.

## Test/Tooling Notes

- There is no CI workflow in `.github/workflows` and no root task runner; use the package-native commands above.
- `sdk/python/pyproject.toml` defines strict mypy and Ruff settings but no scripts; invoke tools directly if needed (`ruff`, `mypy`) after installing `.[dev]`.
- TypeScript package has no lockfile in repo. Avoid committing a generated `package-lock.json` unless dependency locking is part of the task.
- `npm run lint` references ESLint but `package.json` does not list ESLint; expect it to fail until dependencies/config are added.
