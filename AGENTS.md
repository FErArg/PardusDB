# AGENTS.md

Guide for AI agents working with the PardusDB codebase.

## Project Overview

PardusDB is a fast, SQLite-like embedded vector database written in Rust. It provides:
- Single-file storage (`.pardus` files) in `~/.pardus/`
- SQL-like query syntax
- HNSW-based vector similarity search
- MCP server for AI agent integration
- Python and TypeScript SDKs

## Key Conventions

### Directory Structure

```
pardus-rag/
├── src/                  # Rust source code
│   ├── main.rs           # REPL entry point
│   ├── lib.rs            # Public API exports
│   ├── database.rs       # High-level Database struct
│   ├── db.rs             # Low-level VectorDB struct
│   ├── table.rs          # Table with rows + HNSW graph
│   ├── graph.rs          # HNSW implementation
│   ├── parser.rs         # SQL parser (recursive descent)
│   ├── schema.rs         # Column, Row, Value types
│   ├── distance.rs       # Distance metrics (Cosine, DotProduct, Euclidean)
│   ├── node.rs           # Graph node + Candidate
│   ├── concurrent.rs     # Thread-safe ConcurrentDatabase
│   ├── prepared.rs       # Prepared statements
│   ├── error.rs          # MarsError enum
│   ├── storage.rs        # Persistence (MARS format)
│   └── gpu.rs            # GPU acceleration (wgpu)
├── mcp/                  # MCP server (TypeScript)
├── sdk/
│   ├── python/           # Python SDK
│   └── typescript/       # TypeScript SDK
├── examples/
│   ├── simple_rag.rs
│   └── python/
└── setup.sh              # Installer
```

### Standard Agent Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `PARDUSDB_PATH` | `~/.pardus/` | Database storage directory |
| `PARDUSDB_BINARY` | `~/.local/bin/pardusdb` | Path to pardusdb binary |
| `PARDUSDB_MCP_SERVER` | `~/.pardus/mcp/server.py` | MCP server script path |
| `MCP_DB_PATH` | `~/.pardus/pardus-rag.db` | Default MCP database file |
| `OPENCODE_MCP_CONFIG` | `~/.config/opencode/opencode.json` | OpenCode MCP configuration |
| `EMBEDDER_MODEL` | `all-MiniLM-L6-v2` | SentenceTransformer model |
| `EMBEDDER_DIM` | `384` | Embedding dimension for the model |
| `EMBEDDER_CACHE` | `~/.cache/huggingface/hub/` | HuggingFace model cache |
| `DEFAULT_VECTOR_DIM` | `384` | Default vector dimension for new tables |
| `HNSW_M` | `16` | HNSW graph max connections per node |
| `HNSW_EF_CONSTRUCTION` | `200` | HNSW construction dynamic list size |
| `HNSW_EF_SEARCH` | `50` | HNSW search dynamic list size |

### Environment Setup

```bash
# Install binary
cargo build --release
cp target/release/pardusdb ~/.local/bin/

# Install MCP server dependencies
pip install mcp sentence-transformers pypdf python-docx openpyxl xlrd

# Install OpenCode MCP config
# Ensure ~/.config/opencode/opencode.json includes:
# {
#   "mcpServers": {
#     "pardusdb": {
#       "command": "python3",
#       "args": ["/home/user/.pardus/mcp/server.py"]
#     }
#   }
# }
```

### Rust Code Conventions

- **Error handling**: Use `MarsError` from `error.rs` with `thiserror`. Never use `unwrap()` on operations that could fail in production code.
- **Locks**: Always handle `RwLock` poisoning gracefully — use `.lock().unwrap_or_else(|e| e.into_inner())` instead of `.unwrap()`.
- **Unsafe**: Any `unsafe` blocks must have explicit comments explaining the safety invariants.
- **Naming**: Use `PascalCase` for types/traits, `snake_case` for functions/variables, `SCREAMING_SNAKE_CASE` for constants.

### Data Flow

```
SQL string → parser::parse() → Command enum
  → Database::execute_command() → Table operations
    → Graph::insert/query → Distance::compute

Text query → MCP agent → pardusdb_search_text tool
  → generate_embedding() → SIMILARITY search → Results
```

## MCP Server Integration

The MCP server spawns the `pardusdb` binary as a subprocess for each operation. Key points:

- Binary must be in PATH or the MCP server code must be updated with the full path
- MCP server uses stdio for communication (JSON-RPC over stdin/stdout)
- Tools are prefixed with `pardusdb_` (e.g., `pardusdb_create_database`, `pardusdb_search_similar`)
- Use `pardusdb_search_text` for semantic search by text query (generates embedding internally)
- Use `pardusdb_search_similar` when you already have an embedding vector

### Available MCP Tools

| Tool | Purpose |
|------|---------|
| `pardusdb_create_database` | Create/open a database file |
| `pardusdb_open_database` | Open an existing database |
| `pardusdb_create_table` | Create a table with schema |
| `pardusdb_insert_vector` | Insert a single vector row |
| `pardusdb_batch_insert` | Insert multiple vector rows |
| `pardusdb_search_similar` | Search by embedding vector |
| `pardusdb_search_text` | Search by text query (generates embedding) |
| `pardusdb_execute_sql` | Execute raw SQL |
| `pardusdb_list_tables` | List all tables |
| `pardusdb_import_text` | Import documents from directory |
| `pardusdb_import_status` | Check import status/log |
| `pardusdb_get_schema` | Get table schema |
| `pardusdb_health_check` | Run health diagnostics |

## Common Tasks

### Adding a new SQL operator

1. Add to `ComparisonOp` enum in `parser.rs`
2. Add parsing logic in `parse_condition()`
3. Add evaluation logic in `table.rs:evaluate_condition()`
4. Add test in the `#[cfg(test)]` module

### Adding a new distance metric

1. Implement `Distance<T>` trait in `distance.rs`
2. Add type alias in `db.rs` (e.g., `CosineDB<f32> = VectorDB<f32, Cosine>`)
3. Update `Table` to use the desired distance in `table.rs`

### Modifying the HNSW graph

The graph is in `graph.rs`. Key methods:
- `insert()`: Add node, search candidates, prune neighbors, back-link
- `search()`: Greedy BFS from start node using max-heap
- `robust_prune()`: Geometric diversity pruning
- `query()`: Search + truncate to k results

## Troubleshooting

### "Vector dimension mismatch"

All vectors in a table must have the same dimension. Check:
- Table was created with correct `VECTOR(n)` dimension
- All inserted vectors have exactly `n` elements

### "Table not found"

- Table was created with `CREATE TABLE`
- Using correct table name (case-sensitive)
- Database connection is valid

### "Invalid number" in vector INSERT

Embedder may produce values in scientific notation (e.g., `-4.846e-33`).
Fixed in v0.4.13 — parser now supports `e`/`E` notation.

### Slow inserts

Use batch inserts instead of individual:
```rust
conn.insert_batch_direct("table", vectors, metadata)?;
```

## Version Bump

All these files must be updated together when bumping version:
- `Cargo.toml` (line 3)
- `mcp/package.json` (line 3)
- `mcp/src/index.ts` (line 611)
- `mcp/src/server.py` (line 1232 - `Server("pardusdb-mcp", "0.x.y")`)
- `sdk/python/pyproject.toml` (line 7)
- `sdk/typescript/pardusdb/package.json` (line 3)

### Testing

Run tests with:
```bash
cargo test
```

For specific modules:
```bash
cargo test --lib graph
cargo test --lib distance
```

### Building

```bash
cargo build --release
```

With GPU support:
```bash
cargo build --release --features gpu
```

## Resources

- [GitHub](https://github.com/pardus-ai/pardusdb)
- [Pardus AI](https://pardusai.org/)
- [Rust Docs](https://doc.rust-lang.org/)
- [MCP Protocol](https://modelcontextprotocol.io/)
