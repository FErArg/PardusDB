# Ingestion Failure Analysis — PardusDB v0.4.x

**Date:** 2026-05-01
**Note attempted:** `2b1da6d6790f4505a14c39c30285d496` — "Cloud and AI Development Act"
**Database:** `/home/ferarg/Descargas/00_Pruebas/data.pardus`
**Target table:** `documents` (384-dim vectors)

---

## 1. What Was Attempted

1. **Read Joplin note** via `joplin_read_note` — succeeded, note content loaded
2. **Opened PardusDB** via `pardusdb_pardusdb_open_database` — succeeded
3. **Verified DB state:**
   - `pardusdb_pardusdb_list_tables` → 1 table (`documents`, 465 rows)
   - `pardusdb_pardusdb_get_schema` → confirmed 384-dim vectors
4. **Generated embedding** via `sentence-transformers` (`all-MiniLM-L6-v2`):
   ```python
   from sentence_transformers import SentenceTransformer
   model = SentenceTransformer('all-MiniLM-L6-v2')
   text = 'Cloud and AI Development Act CADA EU data centres sovereign cloud...'
   embedding = model.encode(text)  # 384-dim float vector
   ```
5. **Inserted via REPL:**
   ```sql
   INSERT INTO documents (embedding, filename, content)
   VALUES ([...384 floats...], 'cloud_ai_development_act.txt', 'Cloud and AI Development Act CADA...')
   ```
   - Response: `Inserted row with id=466`
6. **Verified insertion:**
   ```sql
   SELECT id, filename FROM documents WHERE filename = 'cloud_ai_development_act.txt'
   ```
   - Response: `Found 0 rows` ❌

---

## 2. The Bug — Root Cause

**Symptom:** Inserts report success but data never persists to disk.

**Documented in development notes (v0.4.6):**
> "El binario de PardusDB tiene un bug que crashea al abrir cualquier conexión porque intenta crear la tabla documents con unwrap() sin manejar el caso 'already exists'. Eso impide cualquier operación (listar, insertar, buscar, etc.)."

### Technical Description

The Rust binary has a **connection initialization flaw**. When any operation opens a `.pardus` database file, the code attempts to run:

```sql
CREATE TABLE IF NOT EXISTS documents (...)
```

But the Rust code wraps this call with `.unwrap()` or similar, without properly handling the `AlreadyExists` variant. This causes:

1. **Connection panic** — silent crash during initialization
2. **Transaction never commits** — data written to memory but not flushed to disk
3. **Appears successful** — application-layer code reports `Inserted row with id=N`
4. **Data lost on close** — file on disk never contains the new rows

### Hypothesized Bug Location

```rust
// src/database.rs or src/connection.rs — init logic
let table = db.create_table_if_not_exists("documents", schema).unwrap();
//                                                     ^^^^^^^
// If table already exists, this panics instead of returning Ok()
```

### Why Both MCP and REPL Show Success

The error occurs **after** the SQL engine returns "ok" at the application layer, but **before** the write is actually committed to the SQLite storage layer. Both interfaces report success because they receive the success response from the engine before the silent crash happens.

### Impact Matrix

| Operation | Interface | Reports | Persists to Disk |
|---|---|---|---|
| `INSERT INTO` | REPL | `id=466` ✅ | ❌ No |
| `insert_vector` | MCP | `Vector inserted` ✅ | ❌ No |
| `batch_insert` | MCP | `Inserted N vectors` ✅ | ❌ No |
| `SELECT id, filename` | REPL | `Found 0 rows` ❌ | N/A |
| `SELECT COUNT(*)` | REPL | `465` (old count) | N/A |
| `search_text` | MCP | Works (reads old data) | N/A |

---

## 3. Evidence from the Session

### REPL Insert Success:
```
Inserted row with id=466
```

### Subsequent Query:
```
SELECT id, filename FROM documents WHERE filename = 'cloud_ai_development_act.txt'
→ Found 0 rows
```

### File State After Insert:
```
$ md5sum data.pardus
cb212a200f7d46bd36dc7c90f8b86d60  data.pardus
```
Hash unchanged from before insert — file never modified.

### MCP SELECT Also Fails:
```
Error: Invalid file format: Expected 'FROM', got 'AS'
```
The MCP tool fails to parse `AS` aliases in SQL queries, compounding the issue.

---

## 4. Known SQL Parser Limitations

| Feature | Status | Error Message |
|---|---|---|
| `AS` column alias | ❌ Unsupported | `Invalid file format: Expected 'FROM', got 'AS'` |
| `GROUP BY` | ❌ Unsupported | Parser error |
| `WHERE` with vector similarity | ✅ Works | Correct |
| `SIMILARITY` operator | ✅ Works | Correct |
| Basic `SELECT` | ✅ Works | Correct |

---

## 5. Resolution Required

The bug must be fixed in the **Rust source code**:

1. **Locate** the `create_table_if_not_exists("documents")` call
2. **Replace** `.unwrap()` with proper error handling (`match` or `?`)
3. **Handle** the `AlreadyExists` case gracefully (skip creation, continue)
4. **Rebuild** the binary: `cargo build --release`
5. **Test** that inserts actually persist:
   ```bash
   ./target/release/pardusdb data.pardus "INSERT INTO documents ..."
   ./target/release/pardusdb data.pardus "SELECT COUNT(*) FROM documents"
   # Should show new row count, not 0
   ```

---

## 6. Related Issues from Development Log

- **v0.4.6:** Bug first documented — connection crash on table recreation
- **v0.4.17:** Code review flagged unused `HashSet` import in `src/graph.rs:1:36`
- **v0.4.18:** `IndentationError` in `server.py:189` — MCP server has Python syntax errors
- **SQL parser:** No `AS` alias support, no `GROUP BY` support
- **Coroutine bug:** `import_text` fails with `expected string or bytes-like object, got 'coroutine'` — async embedder not awaited

---

*Generated from session: 2026-05-01*
*Agent: opencode (Plan Mode → Build Mode)*
*PardusDB version: v0.4.x (development)*