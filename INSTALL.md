# Installation Guide

Complete installation and configuration guide for PardusDB.

## Requirements

- **Rust** (for building from source) — install from https://rustup.rs/
- **Node.js 18+** (for MCP server and TypeScript SDK) — install from https://nodejs.org/
- **Python 3.10+** (for Python SDK) — install from https://python.org/
- **npm** (comes with Node.js)

## Quick Install

```bash
git clone https://github.com/pardus-ai/pardusdb
cd pardusdb
./setup.sh --install
```

This installs:
- Binary `pardusdb` → `~/.local/bin/pardusdb`
- Helper script `pardus` → `~/.local/bin/pardus`
- Default database directory → `~/.local/share/pardus/`
- MCP server → `~/.local/share/pardus/mcp/`
- Python SDK
- TypeScript SDK

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
pardus                    # Opens ~/.local/share/pardus/data.pardus (creates if missing)
pardus mi.db              # Opens specific file
```

### Using the Binary Directly

```bash
pardusdb                          # In-memory session (no persistence)
pardusdb ~/.local/share/pardus/mi.db   # Open specific file
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

### Configuring MCP in OpenCode

Add this to your OpenCode configuration file (`~/.config/opencode/opencode.jsonc` or `./opencode.jsonc`):

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "pardusdb": {
      "type": "local",
      "command": ["node", "/home/YOUR_USER/.local/share/pardus/mcp/dist/index.js"],
      "enabled": true
    }
  }
}
```

Adjust the path to match your installation.

### Using MCP in Claude Desktop

Add to `~/.claude.json`:

```json
{
  "mcpServers": {
    "pardusdb": {
      "command": "node",
      "args": ["/home/YOUR_USER/.local/share/pardus/mcp/dist/index.js"]
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

---

## Backup

PardusDB stores all data in a single `.pardus` file. To backup:

```bash
# Simple copy (while database is not in use)
cp ~/.local/share/pardus/data.pardus ~/.local/share/pardus/data-backup-$(date +%Y%m%d).pardus

# Or with the REPL
pardus
> .save
> quit
cp ~/.local/share/pardus/data.pardus /path/to/backup/
```

### Restore

```bash
cp /path/to/backup/data.pardus ~/.local/share/pardus/data.pardus
pardus
```

---

## Uninstall

```bash
cd /path/to/pardusdb
./setup.sh --uninstall
```

This removes:
- `~/.local/bin/pardusdb`
- `~/.local/bin/pardus`
- `~/.local/share/pardus/` (including all database files)
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

### TypeScript SDK

```bash
cd /path/to/pardusdb/sdk/typescript/pardusdb
npm install
npm run build
```

```typescript
import { PardusDB } from 'pardusdb';

const client = new PardusDB();
await client.createTable('docs', 768, { content: 'TEXT' });
await client.insert('docs', [0.1, 0.2], { content: 'Hello' });
const results = await client.search('docs', [0.1, 0.2], 10);
```

See `sdk/typescript/pardusdb/README.md` for full documentation.

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
pardusdb_create_database with path="~/.local/share/pardus/mydb.pardus"
```

### "Node version too old"

PardusDB requires Node.js 18+. Update:
```bash
# Using nvm (recommended)
nvm install 18
nvm use 18
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
| Data directory | `~/.local/share/pardus/` |
| Default DB | `~/.local/share/pardus/data.pardus` |
| MCP server | `~/.local/share/pardus/mcp/` |
| Python SDK | (installed via pip) |
| TypeScript SDK | `sdk/typescript/pardusdb/` |