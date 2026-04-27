# AGENTS.md

Guide for AI agents working with the PardusDB codebase.

## Project Overview

PardusDB is a fast, SQLite-like embedded vector database written in Rust. It provides:
- Single-file storage (`.pardus` files) in `~/.local/share/pardus/`
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
│   ├── database.rs        # High-level Database struct
│   ├── db.rs             # Low-level VectorDB struct
│   ├── table.rs          # Table with rows + HNSW graph
│   ├── graph.rs          # HNSW implementation
│   ├── parser.rs         # SQL parser (recursive descent)
│   ├── schema.rs         # Column, Row, Value types
│   ├── distance.rs       # Distance metrics (Cosine, DotProduct, Euclidean)
│   ├── node.rs           # Graph node + Candidate
│   ├── concurrent.rs      # Thread-safe ConcurrentDatabase
│   ├── prepared.rs        # Prepared statements
│   ├── error.rs          # MarsError enum
│   ├── storage.rs         # Persistence (MARS format)
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
```

### Version Bump

All these files must be updated together when bumping version:
- `Cargo.toml` (line 3)
- `mcp/package.json` (line 3)
- `mcp/src/index.ts` (line 611)
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

## MCP Server Integration

The MCP server spawns the `pardusdb` binary as a subprocess for each operation. Key points:

- Binary must be in PATH or the MCP server code must be updated with the full path
- MCP server uses stdio for communication (JSON-RPC over stdin/stdout)
- Tools are prefixed with `pardusdb_` (e.g., `pardusdb_create_database`, `pardusdb_search_similar`)

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

### Slow inserts

Use batch inserts instead of individual:
```rust
conn.insert_batch_direct("table", vectors, metadata)?;
```

## Resources

- [GitHub](https://github.com/pardus-ai/pardusdb)
- [Pardus AI](https://pardusai.org/)
- [Rust Docs](https://doc.rust-lang.org/)
- [MCP Protocol](https://modelcontextprotocol.io/)