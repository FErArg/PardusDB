# PardusDB MCP Server — Jan AI Integration

[Jan](https://jan.ai) is a desktop AI application that supports the Model Context Protocol (MCP), allowing you to connect external tools and data sources. This guide explains how to integrate the PardusDB MCP server with Jan to enable vector similarity search, document ingestion, and database management through your AI conversations.

## Overview

Once configured, Jan will be able to use these PardusDB tools:

- **Database management**: Create/open databases and tables
- **Vector operations**: Insert vectors, batch insert, similarity search
- **Text search**: Semantic search with automatic embedding generation
- **Document ingestion**: Import PDFs, DOCX, TXT, MD, CSV, JSON, JSONL, XLSX, XLS with automatic chunking and embedding
- **Joplin integration**: Ingest notes directly from Joplin
- **Health checks**: Verify database integrity and fix issues
- **Token tracking**: Monitor token savings from semantic search

## Prerequisites

1. **pardusdb binary in PATH** — Install it via `cargo build --release` and ensure `pardusdb` is accessible, or use the installer script.
2. **Python 3.8+** — The MCP server is a Python application.
3. **Python packages** — Install the required dependencies:

```bash
pip install mcp sentence-transformers pypdf python-docx openpyxl xlrd
```

> **Note**: `sentence-transformers` is optional but required for text-to-vector embedding (without it, searches will return zero vectors). The other packages are only needed if you plan to import their respective file types.

## Jan Configuration

### Step 1: Enable Experimental Features

1. Open Jan → **Settings** → **General** → **Advanced**
2. Enable experimental features

### Step 2: Add the MCP Server

1. Go to **Settings** → **MCP Servers**
2. Click **+ Add MCP Server** in the top right corner
3. Configure the server:

| Field | Value |
|-------|-------|
| Server Name | `pardusdb` |
| Transport | `STDIO` |
| Command | `python3` |
| Args | Full path to `server.py` (see below) |
| Env | (leave empty) |

**Args path** depends on your installation:

| Installation | Args |
|-------------|------|
| Source clone | `<ruta-al-clon>/pardus-rag-ng/mcp/src/server.py` |
| Installer (Linux) | `~/.pardus/mcp/src/server.py` |
| Installer (macOS) | `~/.pardus/mcp/run_pardusdb_mcp.sh` |

For macOS with the installer, you can also use the launcher script:

| Field | Value |
|-------|-------|
| Command | `~/.pardus/mcp/run_pardusdb_mcp.sh` |
| Args | (leave empty) |

### Step 3: Enable the Server

Toggle the server ON. It will show a green indicator when active.

### Step 4: Verify Connection

1. Start a new conversation
2. Select a model with tool-calling enabled (e.g., Claude 3.7+, GPT-4o, or a local model with tools enabled)
3. Ask: "Show me the PardusDB status"
4. Jan should respond with the database status, confirming the connection works

## Usage Examples

### Create a Database and Table

```
"Create a database called 'knowledge' and a table with 384-dimensional vectors"
```

Jan will call `pardusdb_create_database` and `pardusdb_create_table`.

### Import Documents

```
"Import all PDFs from ~/Documents/papers into a table called 'research'"
```

Jan will call `pardusdb_import_text` with the directory path and table name.

### Semantic Search

```
"Find documents about machine learning optimization techniques"
```

Jan will call `pardusdb_search_text` with the query. PardusDB generates the embedding automatically and returns the most similar chunks.

### Ingest a Joplin Note

1. First fetch the note using Joplin tools
2. Then ask: "Ingest this note into my 'notes' table"

Jan will call `pardusdb_ingest_joplin` with the note content, title, and tags.

### Check Database Health

```
"Run a health check on the knowledge database"
```

Jan will call `pardusdb_health_check` to verify tables, detect orphan records, and check embedding consistency.

## Server Discovery

When Jan starts the MCP server, it runs from the current working directory. The server automatically:

- Looks for `database.pardus` in the CWD
- Creates it if not found (in-memory mode until saved)
- Verifies integrity on startup

For persistent databases, explicitly open one:

```
"Open the database at ~/mydb.pardus"
```

## Tool Reference

| Tool | Description |
|------|-------------|
| `pardusdb_create_database` | Create a new `.pardus` file |
| `pardusdb_open_database` | Open an existing database |
| `pardusdb_create_table` | Create a vector table with optional metadata columns |
| `pardusdb_insert_vector` | Insert a single vector with metadata |
| `pardusdb_batch_insert` | Insert multiple vectors efficiently |
| `pardusdb_search_similar` | Vector similarity search (requires query vector) |
| `pardusdb_search_text` | Semantic text search (auto-generates embedding) |
| `pardusdb_execute_sql` | Run raw SQL commands |
| `pardusdb_list_tables` | List all tables |
| `pardusdb_use_table` | Set active table for subsequent operations |
| `pardusdb_status` | Show current database and table |
| `pardusdb_import_text` | Bulk import directory of documents |
| `pardusdb_ingest_chunked` | Single document with sentence-aware chunking |
| `pardusdb_ingest_joplin` | Ingest Joplin note with metadata |
| `pardusdb_ingest_async` | Large file ingestion with job tracking |
| `pardusdb_ingest_status` | Poll async job progress |
| `pardusdb_health_check` | Integrity verification and repair |
| `pardusdb_get_schema` | Table schema and row count |
| `pardusdb_import_status` | View/reset import history |
| `pardusdb_get_stats` | Token savings statistics |
| `pardusdb_set_model` | Set LLM model for accurate token tracking |
| `pardusdb_reset_stats` | Reset session statistics |
| `pardusdb_refresh_model_context` | Update model context window cache |

## Troubleshooting

### Server won't connect

1. Verify Python and path: `python3 -c "import mcp; print('OK')"`
2. Check the `pardusdb` binary is in PATH: `which pardusdb`
3. Restart Jan completely
4. Verify the server path in Jan settings is correct

### Model won't use tools

- Ensure the model has tool calling enabled in **Model Capabilities**
- Some open-source models do not support tool calling well — try Claude or GPT-4

### Text search returns zero vectors

- Install `sentence-transformers`: `pip install sentence-transformers`
- Restart Jan after installation
- Verify the embedder loads without errors in the MCP server logs

### Import fails silently

- Check the import history: "Show me import status"
- Verify file permissions on the target directory
- Large files (>50MB) should use `pardusdb_ingest_async`

## Further Reading

- [Jan MCP Documentation](https://www.jan.ai/docs/desktop/mcp)
- [Jan MCP Server Settings](https://www.jan.ai/docs/desktop/integrations/mcp-servers)
- [PardusDB Documentation](./README.md)
- [Available MCP Servers](https://mcp.so)
