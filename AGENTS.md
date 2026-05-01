# AGENTS.md

## Repo Shape

- Root is a single Rust crate, not a Cargo workspace. Core API is `src/lib.rs`; CLI entrypoint is `src/main.rs`.
- Sidecars live outside the Rust crate: `mcp/src/server.py`, `sdk/python/`, and `sdk/typescript/pardusdb/`.
- `pardusdb` with no args starts an in-memory REPL. `pardusdb <path>` opens or creates a file-backed DB and reads commands from `stdin` until `quit`.
- The MCP server and both SDKs shell out to the `pardusdb` binary and parse its text output. Treat CLI output and meta-command behavior as compatibility-sensitive.

## Commands

- Rust build: `cargo build --release`
- Full Rust tests: `cargo test`
- Focused Rust integration tests: `cargo test --test database_test`, `cargo test --test concurrent_test`, `cargo test --test sql_parser_test`, `cargo test --test bug_dimension_mismatch`
- Example app: `cargo run --example simple_rag --release`
- GPU paths require `--features gpu`
- Python SDK checks: `cd sdk/python && pip install -e ".[dev]" && pytest && ruff check && mypy pardusdb`
- TypeScript SDK checks: `cd sdk/typescript/pardusdb && npm install && npm test && npm run build`
- There is no repo-local CI workflow or task runner to tell you what to run; verify the package you touched directly.

## Behavior Gotchas

- The `pardus` helper script is the persistent default-DB entrypoint: with no args it auto-creates and opens `~/.pardus/pardus-rag.db`. The raw `pardusdb` binary does not do that; no-path mode is in-memory only.
- `Database::open(path)` creates the file if it does not exist. File-backed flows often rely on that implicit create behavior.
- SQL vectors use square brackets, e.g. `[0.1, 0.2]`. Similarity syntax is `WHERE embedding SIMILARITY [..]`, and results are distance-ascending.
- `mcp/src/server.py` defaults embedding-based text tools to dimension `384` and model `all-MiniLM-L6-v2`.
- If `sentence-transformers` is unavailable, MCP text import/search code falls back to zero vectors instead of hard-failing.
- `sdk/typescript/pardusdb/package.json` defines `npm run lint`, but does not declare `eslint` in `devDependencies`; do not assume lint works in a clean checkout.
- **macOS venv isolation**: `install-macos.sh` installs all Python packages (mcp, sentence-transformers, pypdf, python-docx, openpyxl, xlrd) inside `~/.pardus/mcp/venv/`. The MCP server runs inside this venv, so all dependencies must be installed there — global pip packages are not visible.
- **macOS Python 3.10+**: `install-macos.sh` auto-detects Python < 3.10 (common on macOS 26 with Python 3.9) and offers to install `python@3.13` via Homebrew. The installer uses `brew_prefix/opt/python@3.13/bin/python3.13` directly (not `python3` symlink).

## Release Notes

- Version strings that affect shipped artifacts are duplicated in `Cargo.toml`, `mcp/src/server.py`, `sdk/python/pyproject.toml`, `sdk/typescript/pardusdb/package.json`, `setup.sh`, `install.sh`, and `install-macos.sh`.
- `README.md` and `INSTALL.md` also hardcode the current version and versioned binary filenames; update them in the same release change.
- **Binary naming convention**: Precompiled binaries in `bin/` must follow the format `pardus-v{VERSION}-{platform}-{arch}` where:
  - `VERSION` is the semantic version (e.g., `0.4.17`)
  - `platform` is lowercase OS name (e.g., `linux`, `darwin`)
  - `arch` is the architecture (e.g., `x86_64`, `arm64`)
  - Examples: `pardus-v0.4.17-linux-x86_64`, `pardus-v0.4.17-darwin-arm64`
- When compiling a new release:
  1. Run `cargo build --release`
  2. Copy the binary: `cp target/release/pardusdb bin/pardus-v{VERSION}-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)`
  3. Update version numbers in all files listed above
  4. Commit and push before running installers
- **Installer binary requirements**:
  - `install.sh` requires `bin/pardus-v{VERSION}-linux-x86_64`. If missing, fails with "Binario precompilado no encontrado".
  - `install-macos.sh` requires `bin/pardus-v{VERSION}-darwin-arm64`. If missing, fails with "Compila en tu Mac con: cargo build --release".
