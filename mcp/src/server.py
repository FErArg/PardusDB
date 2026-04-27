#!/usr/bin/env python3
"""
PardusDB MCP Server

Model Context Protocol server for PardusDB vector database.
Enables AI agents to perform vector similarity search, manage vector data,
import documents, and perform health checks.
"""

import asyncio
import csv
import hashlib
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional

try:
    from mcp.server import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import Tool, TextContent
except ImportError:
    print("Error: mcp package not found. Install with: pip install mcp", file=sys.stderr)
    sys.exit(1)


MAX_FILE_SIZE_MB = 50
DEFAULT_VECTOR_DIM = 384
EMBEDDER_MODEL = "all-MiniLM-L6-v2"


# ==================== Optional Dependencies ====================

try:
    from sentence_transformers import SentenceTransformer
    _embedder = SentenceTransformer(EMBEDDER_MODEL)
    HAS_EMBEDDER = True
except ImportError:
    _embedder = None
    HAS_EMBEDDER = False

try:
    import pypdf
    PDF_AVAILABLE = True
except ImportError:
    PDF_AVAILABLE = False

try:
    import docx
    DOCX_AVAILABLE = True
except ImportError:
    DOCX_AVAILABLE = False

try:
    import openpyxl
    XLSX_AVAILABLE = True
except ImportError:
    XLSX_AVAILABLE = False


# ==================== Types ====================

class PardusDBClient:
    def __init__(self) -> None:
        self.db_path: Optional[str] = None
        self.current_table: Optional[str] = None

    async def execute(self, command: str) -> str:
        db_arg = [self.db_path] if self.db_path else []
        proc = await asyncio.create_subprocess_exec(
            "pardusdb",
            *db_arg,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate(input=f"{command}\nquit\n".encode())
        return (stdout + stderr).decode()

    def set_db_path(self, db_path: Optional[str]) -> None:
        self.db_path = db_path

    def get_db_path(self) -> Optional[str]:
        return self.db_path

    def set_current_table(self, table_name: Optional[str]) -> None:
        self.current_table = table_name

    def get_current_table(self) -> Optional[str]:
        return self.current_table


db_client = PardusDBClient()


# ==================== Helper Functions ====================

def file_hash(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def generate_embedding(text: str, dim: int) -> list[float]:
    if not text or not HAS_EMBEDDER:
        return [0.0] * dim
    try:
        vec = _embedder.encode(text, convert_to_numpy=True)
        if vec is None:
            return [0.0] * dim
        vec = vec.tolist() if hasattr(vec, 'tolist') else list(vec)
        if len(vec) != dim:
            return [0.0] * dim
        return vec
    except Exception:
        return [0.0] * dim


def get_table_schema(table: str) -> dict[str, Any]:
    schema = {
        "exists": False,
        "columns": [],
        "row_count": 0,
        "vector_dim": None,
    }
    try:
        result = db_client.execute(f"SHOW TABLES")
        if table in result:
            schema["exists"] = True
        count_result = db_client.execute(f"SELECT COUNT(*) FROM {table}")
        try:
            import re
            m = re.search(r"Count:\s*(\d+)", count_result)
            if m:
                schema["row_count"] = int(m.group(1))
        except Exception:
            pass
    except Exception:
        pass
    return schema


def ensure_import_table(table: str, dim: int) -> None:
    create_sql = f"""CREATE TABLE IF NOT EXISTS {table} (
        embedding VECTOR({dim}),
        filename TEXT,
        content TEXT,
        page INT,
        file_type TEXT,
        parent_doc_id INT,
        doc_path TEXT,
        chunk_index INT,
        total_chunks INT,
        title TEXT
    )"""
    db_client.execute(create_sql)
    db_client.execute(f"""CREATE TABLE IF NOT EXISTS __import_log__ (
        id INTEGER PRIMARY KEY,
        table_name TEXT,
        doc_path TEXT,
        filename TEXT,
        file_size INT,
        file_hash TEXT,
        imported_at TEXT,
        total_parents INT,
        total_children INT,
        status TEXT
    )""")


def log_import(
    table: str,
    doc_path: str,
    filename: str,
    file_size: int,
    file_hash: str,
    total_parents: int,
    total_children: int,
    status: str,
) -> None:
    now = datetime.now(timezone.utc).isoformat()
    sql = f"""INSERT INTO __import_log__
        (table_name, doc_path, filename, file_size, file_hash, imported_at, total_parents, total_children, status)
        VALUES ('{table}', '{doc_path}', '{filename}', {file_size}, '{file_hash}', '{now}', {total_parents}, {total_children}, '{status}')"""
    try:
        db_client.execute(sql)
    except Exception:
        pass


def is_already_imported(table: str, file_hash: str) -> bool:
    sql = f"SELECT COUNT(*) FROM __import_log__ WHERE table_name = '{table}' AND file_hash = '{file_hash}'"
    try:
        result = db_client.execute(sql)
        import re
        m = re.search(r"Count:\s*(\d+)", result)
        if m and int(m.group(1)) > 0:
            return True
    except Exception:
        pass
    return False


def parse_id_from_result(result: str) -> Optional[int]:
    import re
    m = re.search(r"id=(\d+)", result)
    if m:
        return int(m.group(1))
    return None


# ==================== File Parsers ====================

def parse_txt(path: str) -> dict[str, Any]:
    content = Path(path).read_text(encoding="utf-8", errors="replace")
    return {
        "title": Path(path).stem,
        "content": content,
        "pages": [{"content": content, "page": 0}],
    }


def parse_md(path: str) -> dict[str, Any]:
    content = Path(path).read_text(encoding="utf-8", errors="replace")
    title = Path(path).stem
    return {
        "title": title,
        "content": content,
        "pages": [{"content": content, "page": 0}],
    }


def parse_csv(path: str) -> dict[str, Any]:
    rows = []
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            reader = csv.DictReader(f)
            for i, row in enumerate(reader):
                rows.append({"content": json.dumps(row), "page": i + 1})
    except Exception:
        content = Path(path).read_text(encoding="utf-8", errors="replace")
        return {
            "title": Path(path).stem,
            "content": content,
            "pages": [{"content": content, "page": 0}],
        }
    full_content = Path(path).read_text(encoding="utf-8", errors="replace")
    return {
        "title": Path(path).stem,
        "content": full_content,
        "pages": rows if rows else [{"content": full_content, "page": 0}],
    }


def parse_pdf(path: str) -> dict[str, Any]:
    if not PDF_AVAILABLE:
        raise ImportError("pypdf not installed. Install with: pip install pypdf")
    reader = pypdf.PdfReader(path)
    title = Path(path).stem
    try:
        if reader.metadata and reader.metadata.get("/Title"):
            title = reader.metadata.get("/Title", title)
    except Exception:
        pass
    pages = []
    for i, page in enumerate(reader.pages):
        try:
            text = page.extract_text()
        except Exception:
            text = ""
        if text.strip():
            pages.append({"content": text, "page": i + 1})
    full_content = "\n\n".join(p["content"] for p in pages)
    return {
        "title": title,
        "content": full_content,
        "pages": pages,
    }


def parse_docx(path: str) -> dict[str, Any]:
    if not DOCX_AVAILABLE:
        raise ImportError("python-docx not installed. Install with: pip install python-docx")
    doc = docx.Document(path)
    title = Path(path).stem
    try:
        if doc.core_properties.title:
            title = doc.core_properties.title
    except Exception:
        pass
    paragraphs = []
    for i, para in enumerate(doc.paragraphs):
        text = para.text.strip()
        if text:
            paragraphs.append({"content": text, "page": i + 1})
    full_content = "\n\n".join(p["content"] for p in paragraphs)
    return {
        "title": title,
        "content": full_content,
        "pages": paragraphs if paragraphs else [{"content": full_content, "page": 0}],
    }


def parse_xlsx(path: str) -> dict[str, Any]:
    if not XLSX_AVAILABLE:
        raise ImportError("openpyxl not installed. Install with: pip install openpyxl")
    wb = openpyxl.load_workbook(path, data_only=True)
    title = Path(path).stem
    if wb.sheetnames:
        title = wb.sheetnames[0]
    rows = []
    for sheet_name in wb.sheetnames:
        ws = wb[sheet_name]
        for i, row in enumerate(ws.iter_rows(values_only=True), start=1):
            row_data = {str(cell.column_letter): str(cell.value) if cell.value is not None else "" for cell in row}
            if any(v for v in row_data.values()):
                rows.append({"content": json.dumps(row_data), "page": i})
    full_content = "\n".join(r["content"] for r in rows)
    return {
        "title": title,
        "content": full_content,
        "pages": rows if rows else [{"content": "", "page": 0}],
    }


def parse_json(path: str) -> dict[str, Any]:
    content = Path(path).read_text(encoding="utf-8", errors="replace")
    try:
        data = json.loads(content)
    except Exception:
        return {
            "title": Path(path).stem,
            "content": content,
            "pages": [{"content": content, "page": 0}],
        }
    pages = []
    if isinstance(data, list):
        for i, item in enumerate(data):
            if isinstance(item, dict):
                text = item.get("text", json.dumps(item))
                pages.append({"content": text, "page": i + 1})
            else:
                pages.append({"content": str(item), "page": i + 1})
    elif isinstance(data, dict):
        text = data.get("text", data.get("content", json.dumps(data)))
        pages.append({"content": text, "page": 0})
    full_content = "\n".join(p["content"] for p in pages)
    return {
        "title": Path(path).stem,
        "content": full_content,
        "pages": pages if pages else [{"content": content, "page": 0}],
    }


def parse_jsonl(path: str) -> dict[str, Any]:
    lines = Path(path).read_text(encoding="utf-8", errors="replace").strip().split("\n")
    pages = []
    for i, line in enumerate(lines, start=1):
        try:
            item = json.loads(line)
        except Exception:
            item = {"text": line}
        if isinstance(item, dict):
            text = item.get("text", item.get("content", json.dumps(item)))
        else:
            text = str(item)
        pages.append({"content": text, "page": i})
    full_content = "\n".join(p["content"] for p in pages)
    return {
        "title": Path(path).stem,
        "content": full_content,
        "pages": pages,
    }


PARSERS = {
    ".txt": parse_txt,
    ".md": parse_md,
    ".csv": parse_csv,
    ".pdf": parse_pdf,
    ".docx": parse_docx,
    ".xlsx": parse_xlsx,
    ".json": parse_json,
    ".jsonl": parse_jsonl,
}

SUPPORTED_EXTENSIONS = list(PARSERS.keys())


# ==================== Tool Handlers ====================

async def handle_create_database(args: dict[str, Any]) -> dict[str, Any]:
    db_path = args.get("path")
    if not db_path:
        return {"content": [{"type": "text", "text": "Error: Database path is required"}], "isError": True}
    try:
        parent = Path(db_path).parent
        if parent and not parent.exists():
            parent.mkdir(parents=True, exist_ok=True)
        db_client.set_db_path(db_path)
        await db_client.execute(f".create {db_path}")
        return {"content": [{"type": "text", "text": f"Database created successfully at: {db_path}"}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error creating database: {e}"}], "isError": True}


async def handle_open_database(args: dict[str, Any]) -> dict[str, Any]:
    db_path = args.get("path")
    if not db_path:
        return {"content": [{"type": "text", "text": "Error: Database path is required"}], "isError": True}
    if not Path(db_path).exists():
        return {"content": [{"type": "text", "text": f"Error: Database file not found: {db_path}"}], "isError": True}
    try:
        db_client.set_db_path(db_path)
        await db_client.execute(f".open {db_path}")
        return {"content": [{"type": "text", "text": f"Database opened successfully: {db_path}"}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error opening database: {e}"}], "isError": True}


async def handle_create_table(args: dict[str, Any]) -> dict[str, Any]:
    name = args.get("name")
    vector_dim = args.get("vector_dim")
    metadata_schema = args.get("metadata_schema")
    if not name or not vector_dim:
        return {"content": [{"type": "text", "text": "Error: Table name and vector_dim are required"}], "isError": True}
    try:
        columns = [f"embedding VECTOR({vector_dim})"]
        type_map = {
            "str": "TEXT", "string": "TEXT",
            "int": "INTEGER", "integer": "INTEGER",
            "float": "FLOAT", "bool": "BOOLEAN", "text": "TEXT",
        }
        if metadata_schema:
            for col_name, col_type in metadata_schema.items():
                sql_type = type_map.get(col_type.lower(), col_type.upper())
                columns.append(f"{col_name} {sql_type}")
        sql = f"CREATE TABLE IF NOT EXISTS {name} ({', '.join(columns)})"
        await db_client.execute(sql)
        db_client.set_current_table(name)
        return {"content": [{"type": "text", "text": f"Table '{name}' created successfully with {vector_dim}-dimensional vectors.\n\nSQL: {sql}"}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error creating table: {e}"}], "isError": True}


async def handle_insert_vector(args: dict[str, Any]) -> dict[str, Any]:
    vector = args.get("vector")
    metadata = args.get("metadata")
    table = args.get("table") or db_client.get_current_table()
    if not vector or not isinstance(vector, list):
        return {"content": [{"type": "text", "text": "Error: Vector array is required"}], "isError": True}
    if not table:
        return {"content": [{"type": "text", "text": "Error: No table specified. Use 'use_table' first or provide 'table' parameter."}], "isError": True}
    try:
        columns = ["embedding"]
        values = [f"[{', '.join(str(x) for x in vector)}]"]
        if metadata:
            for key, val in metadata.items():
                columns.append(key)
                if isinstance(val, str):
                    values.append(f"'{val}'")
                elif isinstance(val, bool):
                    values.append("true" if val else "false")
                else:
                    values.append(str(val))
        sql = f"INSERT INTO {table} ({', '.join(columns)}) VALUES ({', '.join(values)})"
        result = await db_client.execute(sql)
        id_match = parse_id_from_result(result) or "unknown"
        return {"content": [{"type": "text", "text": f"Vector inserted successfully with ID: {id_match}"}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error inserting vector: {e}"}], "isError": True}


async def handle_batch_insert(args: dict[str, Any]) -> dict[str, Any]:
    vectors = args.get("vectors")
    metadata_list = args.get("metadata_list")
    table = args.get("table") or db_client.get_current_table()
    if not vectors or not isinstance(vectors, list):
        return {"content": [{"type": "text", "text": "Error: Vectors array is required"}], "isError": True}
    if not table:
        return {"content": [{"type": "text", "text": "Error: No table specified"}], "isError": True}
    try:
        results = []
        for i, vector in enumerate(vectors):
            metadata = metadata_list[i] if metadata_list else None
            columns = ["embedding"]
            values = [f"[{', '.join(str(x) for x in vector)}]"]
            if metadata:
                for key, val in metadata.items():
                    columns.append(key)
                    if isinstance(val, str):
                        values.append(f"'{val}'")
                    elif isinstance(val, bool):
                        values.append("true" if val else "false")
                    else:
                        values.append(str(val))
            sql = f"INSERT INTO {table} ({', '.join(columns)}) VALUES ({', '.join(values)})"
            result = await db_client.execute(sql)
            vid = parse_id_from_result(result)
            if vid:
                results.append(str(vid))
        return {"content": [{"type": "text", "text": f"Batch insert completed. Inserted {len(results)} vectors with IDs: {', '.join(results)}"}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error during batch insert: {e}"}], "isError": True}


async def handle_search_similar(args: dict[str, Any]) -> dict[str, Any]:
    query_vector = args.get("query_vector")
    k = args.get("k", 10)
    table = args.get("table") or db_client.get_current_table()
    if not query_vector or not isinstance(query_vector, list):
        return {"content": [{"type": "text", "text": "Error: query_vector array is required"}], "isError": True}
    if not table:
        return {"content": [{"type": "text", "text": "Error: No table specified"}], "isError": True}
    try:
        vector_str = f"[{', '.join(str(x) for x in query_vector)}]"
        sql = f"SELECT * FROM {table} WHERE embedding SIMILARITY {vector_str} LIMIT {k}"
        result = await db_client.execute(sql)
        return {"content": [{"type": "text", "text": f"Search Results:\n\n{result}"}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error searching: {e}"}], "isError": True}


async def handle_execute_sql(args: dict[str, Any]) -> dict[str, Any]:
    sql = args.get("sql")
    if not sql:
        return {"content": [{"type": "text", "text": "Error: SQL query is required"}], "isError": True}
    try:
        result = await db_client.execute(sql)
        return {"content": [{"type": "text", "text": f"Query Result:\n\n{result}"}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error executing SQL: {e}"}], "isError": True}


async def handle_list_tables() -> dict[str, Any]:
    try:
        result = await db_client.execute("SHOW TABLES")
        return {"content": [{"type": "text", "text": f"Tables:\n\n{result}"}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error listing tables: {e}"}], "isError": True}


async def handle_use_table(args: dict[str, Any]) -> dict[str, Any]:
    table = args.get("table")
    if not table:
        return {"content": [{"type": "text", "text": "Error: Table name is required"}], "isError": True}
    db_client.set_current_table(table)
    return {"content": [{"type": "text", "text": f"Now using table: {table}"}]}


async def handle_get_status() -> dict[str, Any]:
    db_path = db_client.get_db_path()
    current_table = db_client.get_current_table()
    status = "PardusDB Status:\n\n"
    status += f"Database: {db_path or 'Not opened (in-memory)'}\n"
    status += f"Current Table: {current_table or 'None selected'}\n"
    if db_path and Path(db_path).exists():
        size = os.path.getsize(db_path)
        status += f"Database Size: {size / 1024:.2f} KB\n"
    return {"content": [{"type": "text", "text": status}]}


# ==================== Import Tool ====================

async def handle_import_text(args: dict[str, Any]) -> dict[str, Any]:
    dir_path = args.get("dir_path")
    table = args.get("table")
    file_patterns = args.get("file_patterns", SUPPORTED_EXTENSIONS)
    recursive = args.get("recursive", True)
    max_file_size_mb = args.get("max_file_size_mb", MAX_FILE_SIZE_MB)
    vector_dim = args.get("vector_dim", DEFAULT_VECTOR_DIM)

    if not dir_path or not table:
        return {"content": [{"type": "text", "text": "Error: dir_path and table are required"}], "isError": True}

    if not Path(dir_path).exists():
        return {"content": [{"type": "text", "text": f"Error: Directory not found: {dir_path}"}], "isError": True}

    try:
        ensure_import_table(table, vector_dim)
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error creating table: {e}"}], "isError": True}

    all_files = []
    base_path = Path(dir_path).resolve()
    pattern_set = set(file_patterns) if file_patterns else set(SUPPORTED_EXTENSIONS)

    try:
        if recursive:
            for ext in pattern_set:
                all_files.extend(base_path.rglob(f"*{ext}"))
        else:
            for ext in pattern_set:
                all_files.extend(base_path.glob(f"*{ext}"))
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error scanning directory: {e}"}], "isError": True}

    all_files = sorted(set(all_files))
    total_files = len(all_files)

    if total_files == 0:
        return {"content": [{"type": "text", "text": f"No supported files found in {dir_path} with patterns {file_patterns}"}]}

    stats = {"imported": 0, "skipped": 0, "errors": 0}
    error_details = []
    imported_files = []

    for i, file_path in enumerate(all_files):
        fpath = str(file_path)
        fsize = 0
        try:
            fsize = file_path.stat().st_size
        except Exception:
            pass

        if max_file_size_mb and fsize > max_file_size_mb * 1024 * 1024:
            stats["skipped"] += 1
            error_details.append(f"Skipped (too large): {file_path.name} ({fsize / 1024 / 1024:.1f} MB)")
            continue

        ext = file_path.suffix.lower()
        parser = PARSERS.get(ext)

        if not parser:
            stats["skipped"] += 1
            continue

        try:
            fhash = file_hash(fpath)
        except Exception:
            fhash = "unknown"

        if is_already_imported(table, fhash):
            stats["skipped"] += 1
            continue

        try:
            parsed = parser(fpath)
        except ImportError as e:
            stats["errors"] += 1
            error_details.append(f"{file_path.name}: {str(e)}")
            continue
        except Exception as e:
            stats["errors"] += 1
            error_details.append(f"{file_path.name}: {str(e)}")
            continue

        title = parsed.get("title", file_path.stem)
        content = parsed.get("content", "")
        pages = parsed.get("pages", [])
        total_chunks = len(pages) if pages else 1

        try:
            parent_embedding = generate_embedding(content, vector_dim)
            vec_str = f"[{', '.join(str(x) for x in parent_embedding)}]"
            title_esc = title.replace("'", "''")
            fpath_esc = fpath.replace("'", "''")
            fname_esc = file_path.name.replace("'", "''")
            content_esc = content[:5000].replace("'", "''").replace("\n", " ").replace("\r", " ")
            sql = (f"INSERT INTO {table} (embedding, filename, content, page, file_type, "
                   f"parent_doc_id, doc_path, chunk_index, total_chunks, title) "
                   f"VALUES ({vec_str}, '{fname_esc}', '{content_esc}', "
                   f"0, '{ext[1:]}', NULL, '{fpath_esc}', 0, {total_chunks}, '{title_esc}')")
            result = db_client.execute(sql)
            parent_id = parse_id_from_result(result)

            if parent_id is None:
                parent_id = -1

            for chunk_idx, page_data in enumerate(pages):
                chunk_content = page_data.get("content", "")
                page_num = page_data.get("page", 0)
                chunk_emb = generate_embedding(chunk_content, vector_dim)
                chunk_vec_str = f"[{', '.join(str(x) for x in chunk_emb)}]"
                chunk_esc = chunk_content[:5000].replace(chr(10), ' ').replace("'", "''")
                child_sql = (f"INSERT INTO {table} (embedding, filename, content, page, file_type, "
                             f"parent_doc_id, doc_path, chunk_index, total_chunks, title) "
                             f"VALUES ({chunk_vec_str}, '{fname_esc}', '{chunk_esc}', "
                             f"{page_num}, '{ext[1:]}', {parent_id}, '{fpath_esc}', {chunk_idx + 1}, {total_chunks}, '{title_esc}')")
                db_client.execute(child_sql)

            log_import(table, fpath, file_path.name, fsize, fhash, 1, total_chunks, "ok")
            stats["imported"] += 1
            imported_files.append(file_path.name)

        except Exception as e:
            stats["errors"] += 1
            error_details.append(f"{file_path.name}: {str(e)}")
            try:
                log_import(table, fpath, file_path.name, fsize, fhash, 0, 0, "error")
            except Exception:
                pass

        if (i + 1) % 5 == 0:
            print(f"[progress] Processed {i + 1}/{total_files} files...", file=sys.stderr, flush=True)

    embedder_info = f"yes ({EMBEDDER_MODEL})" if HAS_EMBEDDER else "no (install sentence-transformers)"
    summary = [
        f"Import completed:",
        f"  Files processed: {total_files}",
        f"  Imported: {stats['imported']}",
        f"  Skipped (already imported or too large): {stats['skipped']}",
        f"  Errors: {stats['errors']}",
        f"  Embeddings: {embedder_info}",
        "",
    ]
    if imported_files:
        summary.append("Imported files:")
        for fn in imported_files[:20]:
            summary.append(f"  - {fn}")
        if len(imported_files) > 20:
            summary.append(f"  ... and {len(imported_files) - 20} more")
    if error_details:
        summary.append("")
        summary.append("Issues:")
        for err in error_details[:20]:
            summary.append(f"  ! {err}")
    return {"content": [{"type": "text", "text": "\n".join(summary)}]}


# ==================== Health Check Tool ====================

async def handle_health_check(args: dict[str, Any]) -> dict[str, Any]:
    target_table = args.get("table")
    repair = args.get("repair", False)

    db_path = db_client.get_db_path()
    if not db_path:
        return {"content": [{"type": "text", "text": "Error: No database opened. Use open_database first."}], "isError": True}

    if not Path(db_path).exists():
        return {"content": [{"type": "text", "text": f"Error: Database file not found: {db_path}"}], "isError": True}

    db_size = os.path.getsize(db_path)
    report = [f"Database Health Report", f"Database: {db_path}", f"Size: {db_size / 1024:.2f} KB", ""]

    try:
        tables_result = db_client.execute("SHOW TABLES")
        import re
        table_names = re.findall(r"Table '(\w+)'", tables_result)
        if not table_names:
            report.append("No tables found.")
            return {"content": [{"type": "text", "text": "\n".join(report)}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error listing tables: {e}"}], "isError": True}

    tables_to_check = [target_table] if target_table else [t for t in table_names if t != "__import_log__"]

    for tbl in tables_to_check:
        if tbl not in table_names:
            report.append(f"Table: {tbl}")
            report.append("  ❌ Table does not exist")
            report.append("")
            continue

        report.append(f"Table: {tbl}")
        has_warnings = False

        try:
            count_result = db_client.execute(f"SELECT COUNT(*) FROM {tbl}")
            m = re.search(r"Count:\s*(\d+)", count_result)
            total_rows = int(m.group(1)) if m else 0
            report.append(f"  Total records: {total_rows}")
        except Exception as e:
            report.append(f"  ⚠️  Could not count rows: {e}")

        try:
            orphans_result = db_client.execute(
                f"SELECT COUNT(*) FROM {tbl} WHERE parent_doc_id IS NOT NULL "
                f"AND parent_doc_id NOT IN (SELECT id FROM {tbl} WHERE parent_doc_id IS NULL)"
            )
            m = re.search(r"Count:\s*(\d+)", orphans_result)
            orphan_count = int(m.group(1)) if m else 0
            if orphan_count > 0:
                report.append(f"  ❌ Orphan children found: {orphan_count}")
                has_warnings = True
            else:
                report.append(f"  ✅ No orphan children")
        except Exception as e:
            report.append(f"  ⚠️  Could not check orphans: {e}")

        try:
            dup_result = db_client.execute(
                f"SELECT doc_path, COUNT(*) as cnt FROM {tbl} "
                f"WHERE parent_doc_id IS NULL AND doc_path IS NOT NULL "
                f"GROUP BY doc_path HAVING cnt > 1"
            )
            if "id=" in dup_result or dup_result.strip():
                report.append(f"  ⚠️  Duplicate parent documents found")
                has_warnings = True
            else:
                report.append(f"  ✅ No duplicate parent documents")
        except Exception:
            pass

        try:
            zero_result = db_client.execute(
                f"SELECT COUNT(*) FROM {tbl} WHERE embedding = '[{', '.join(['0.0'] * DEFAULT_VECTOR_DIM)}]'"
            )
            m = re.search(r"Count:\s*(\d+)", zero_result)
            zero_count = int(m.group(1)) if m else 0
            if zero_count > 0:
                report.append(f"  ⚠️  Records with zero vectors: {zero_count}")
                has_warnings = True
            else:
                report.append(f"  ✅ All records have valid embeddings")
        except Exception:
            pass

        if not has_warnings:
            report.append(f"  ✅ Health: PASS")
        else:
            report.append(f"  ⚠️  Health: WARNINGS")
        report.append("")

    try:
        if "__import_log__" in table_names:
            log_count = db_client.execute("SELECT COUNT(*) FROM __import_log__")
            m = re.search(r"Count:\s*(\d+)", log_count)
            log_rows = int(m.group(1)) if m else 0
            report.append(f"Import Log: {log_rows} entries")
            ok_count_result = db_client.execute("SELECT COUNT(*) FROM __import_log__ WHERE status = 'ok'")
            m2 = re.search(r"Count:\s*(\d+)", ok_count_result)
            ok_rows = int(m2.group(1)) if m2 else 0
            report.append(f"  Successful imports: {ok_rows}")
        else:
            report.append("Import Log: not found")
    except Exception as e:
        report.append(f"⚠️  Could not check import log: {e}")

    return {"content": [{"type": "text", "text": "\n".join(report)}]}


# ==================== Schema Tool ====================

async def handle_get_schema(args: dict[str, Any]) -> dict[str, Any]:
    table = args.get("table")
    if not table:
        return {"content": [{"type": "text", "text": "Error: Table name is required"}], "isError": True}

    try:
        tables_result = db_client.execute("SHOW TABLES")
        if table not in tables_result:
            return {"content": [{"type": "text", "text": f"Table '{table}' does not exist"}], "isError": True}

        count_result = db_client.execute(f"SELECT COUNT(*) FROM {table}")
        import re
        m = re.search(r"Count:\s*(\d+)", count_result)
        row_count = int(m.group(1)) if m else 0

        output = [f"Schema for table: {table}", f"Total rows: {row_count}", ""]

        output.append("Note: Use raw SQL to inspect column types, as SHOW TABLES does not expose schema details.")
        output.append("Known columns for import tables: embedding, filename, content, page, file_type, parent_doc_id, doc_path, chunk_index, total_chunks, title")

        return {"content": [{"type": "text", "text": "\n".join(output)}]}
    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error getting schema: {e}"}], "isError": True}


# ==================== Import Status Tool ====================

async def handle_import_status(args: dict[str, Any]) -> dict[str, Any]:
    action = args.get("action", "list")
    table = args.get("table")
    doc_path = args.get("doc_path")

    try:
        tables_result = db_client.execute("SHOW TABLES")
        if "__import_log__" not in tables_result:
            return {"content": [{"type": "text", "text": "No import history found (import_log table does not exist)"}]}

        if action == "list":
            where_clauses = []
            if table:
                where_clauses.append(f"table_name = '{table}'")
            if doc_path:
                where_clauses.append(f"doc_path = '{doc_path}'")
            where = " AND ".join(where_clauses) if where_clauses else "1=1"
            sql = f"SELECT filename, table_name, file_size, imported_at, total_parents, total_children, status FROM __import_log__ WHERE {where} ORDER BY imported_at DESC LIMIT 50"
            result = db_client.execute(sql)
            return {"content": [{"type": "text", "text": f"Import History:\n\n{result}"}]}

        elif action == "reset":
            if not table and not doc_path:
                return {"content": [{"type": "text", "text": "Error: Provide table or doc_path to reset"}], "isError": True}
            where_clauses = []
            if table:
                where_clauses.append(f"table_name = '{table}'")
            if doc_path:
                where_clauses.append(f"doc_path = '{doc_path}'")
            where = " AND ".join(where_clauses)
            sql = f"DELETE FROM __import_log__ WHERE {where}"
            db_client.execute(sql)
            return {"content": [{"type": "text", "text": f"Import records reset for: {table or ''} {doc_path or ''}"}]}

        else:
            return {"content": [{"type": "text", "text": f"Unknown action: {action}. Use 'list' or 'reset'."}], "isError": True}

    except Exception as e:
        return {"content": [{"type": "text", "text": f"Error: {e}"}], "isError": True}


# ==================== Tool Definitions ====================

TOOLS = [
    Tool(
        name="pardusdb_create_database",
        description="Create a new PardusDB database file at the specified path",
        inputSchema={
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path for the new .pardus database file"},
            },
            "required": ["path"],
        },
    ),
    Tool(
        name="pardusdb_open_database",
        description="Open an existing PardusDB database file",
        inputSchema={
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the existing .pardus database file"},
            },
            "required": ["path"],
        },
    ),
    Tool(
        name="pardusdb_create_table",
        description="Create a new table for storing vectors with optional metadata columns",
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Name of the table"},
                "vector_dim": {"type": "number", "description": "Dimension of the vectors (e.g. 768)"},
                "metadata_schema": {
                    "type": "object",
                    "description": "Optional metadata columns: {column_name: type}. Types: str, int, float, bool",
                    "additionalProperties": {"type": "string"},
                },
            },
            "required": ["name", "vector_dim"],
        },
    ),
    Tool(
        name="pardusdb_insert_vector",
        description="Insert a single vector with optional metadata into a table",
        inputSchema={
            "type": "object",
            "properties": {
                "vector": {"type": "array", "items": {"type": "number"}, "description": "The embedding vector"},
                "metadata": {"type": "object", "description": "Optional metadata to store with the vector"},
                "table": {"type": "string", "description": "Table name (uses current table if not specified)"},
            },
            "required": ["vector"],
        },
    ),
    Tool(
        name="pardusdb_batch_insert",
        description="Insert multiple vectors efficiently in a batch",
        inputSchema={
            "type": "object",
            "properties": {
                "vectors": {
                    "type": "array",
                    "items": {"type": "array", "items": {"type": "number"}},
                    "description": "Array of embedding vectors",
                },
                "metadata_list": {"type": "array", "items": {"type": "object"}, "description": "Optional metadata per vector"},
                "table": {"type": "string", "description": "Table name (uses current table if not specified)"},
            },
            "required": ["vectors"],
        },
    ),
    Tool(
        name="pardusdb_search_similar",
        description="Search for vectors similar to a query vector using cosine similarity",
        inputSchema={
            "type": "object",
            "properties": {
                "query_vector": {"type": "array", "items": {"type": "number"}, "description": "The query embedding vector"},
                "k": {"type": "number", "description": "Number of results (default: 10)"},
                "table": {"type": "string", "description": "Table name (uses current table if not specified)"},
            },
            "required": ["query_vector"],
        },
    ),
    Tool(
        name="pardusdb_execute_sql",
        description="Execute raw SQL commands on the database",
        inputSchema={
            "type": "object",
            "properties": {
                "sql": {"type": "string", "description": "SQL command to execute"},
            },
            "required": ["sql"],
        },
    ),
    Tool(
        name="pardusdb_list_tables",
        description="List all tables in the current database",
        inputSchema={"type": "object", "properties": {}},
    ),
    Tool(
        name="pardusdb_use_table",
        description="Set the current table for subsequent operations",
        inputSchema={
            "type": "object",
            "properties": {
                "table": {"type": "string", "description": "Name of the table to use"},
            },
            "required": ["table"],
        },
    ),
    Tool(
        name="pardusdb_status",
        description="Get the current status of the database connection",
        inputSchema={"type": "object", "properties": {}},
    ),
    Tool(
        name="pardusdb_import_text",
        description="Import documents from a directory into a table. Scans recursively and imports PDF, CSV, DOCX, XLSX, JSON, JSONL, MD, and TXT files with automatic embeddings. Multi-page files create parent + child records with parent-child tracking. Skips already-imported files (by SHA256 hash).",
        inputSchema={
            "type": "object",
            "properties": {
                "dir_path": {"type": "string", "description": "Directory path to scan for documents"},
                "table": {"type": "string", "description": "Target table name (created if not exists)"},
                "file_patterns": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File extensions to include. Default: all supported (.pdf, .csv, .docx, .xlsx, .json, .jsonl, .md, .txt)",
                },
                "recursive": {"type": "boolean", "description": "Scan subdirectories recursively (default: true)"},
                "max_file_size_mb": {"type": "number", "description": f"Max file size in MB (default: {MAX_FILE_SIZE_MB})"},
                "vector_dim": {"type": "number", "description": f"Embedding dimension (default: {DEFAULT_VECTOR_DIM})"},
            },
            "required": ["dir_path", "table"],
        },
    ),
    Tool(
        name="pardusdb_health_check",
        description="Run integrity checks on the database. Verifies tables, detects orphan children, checks for duplicate parent documents, and validates embedding consistency.",
        inputSchema={
            "type": "object",
            "properties": {
                "table": {"type": "string", "description": "Specific table to check (checks all if omitted)"},
                "repair": {"type": "boolean", "description": "Attempt to fix issues (default: false)"},
            },
        },
    ),
    Tool(
        name="pardusdb_get_schema",
        description="Show the schema and statistics of a table",
        inputSchema={
            "type": "object",
            "properties": {
                "table": {"type": "string", "description": "Table name"},
            },
            "required": ["table"],
        },
    ),
    Tool(
        name="pardusdb_import_status",
        description="View or reset import history. Use 'list' to see imports, 'reset' to clear records for re-importing.",
        inputSchema={
            "type": "object",
            "properties": {
                "action": {"type": "string", "enum": ["list", "reset"], "description": "'list' to view history, 'reset' to clear records"},
                "table": {"type": "string", "description": "Filter by table name (optional)"},
                "doc_path": {"type": "string", "description": "Filter by file path (optional, for reset)"},
            },
            "required": ["action"],
        },
    ),
]


# ==================== Server Setup ====================

server = Server("pardusdb-mcp", "0.4.6")


@server.list_tools()
async def list_tools() -> list[Tool]:
    return TOOLS


@server.call_tool()
async def call_tool(name: str, args: dict[str, Any]) -> list[TextContent]:
    result: dict[str, Any]

    if name == "pardusdb_create_database":
        result = await handle_create_database(args)
    elif name == "pardusdb_open_database":
        result = await handle_open_database(args)
    elif name == "pardusdb_create_table":
        result = await handle_create_table(args)
    elif name == "pardusdb_insert_vector":
        result = await handle_insert_vector(args)
    elif name == "pardusdb_batch_insert":
        result = await handle_batch_insert(args)
    elif name == "pardusdb_search_similar":
        result = await handle_search_similar(args)
    elif name == "pardusdb_execute_sql":
        result = await handle_execute_sql(args)
    elif name == "pardusdb_list_tables":
        result = await handle_list_tables()
    elif name == "pardusdb_use_table":
        result = await handle_use_table(args)
    elif name == "pardusdb_status":
        result = await handle_get_status()
    elif name == "pardusdb_import_text":
        result = await handle_import_text(args)
    elif name == "pardusdb_health_check":
        result = await handle_health_check(args)
    elif name == "pardusdb_get_schema":
        result = await handle_get_schema(args)
    elif name == "pardusdb_import_status":
        result = await handle_import_status(args)
    else:
        result = {"content": [{"type": "text", "text": f"Unknown tool: {name}"}], "isError": True}

    is_error = result.pop("isError", False)
    return [TextContent(type="text", text=result["content"][0]["text"])]


async def main() -> None:
    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream, server.create_initialization_options())


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        sys.exit(0)
    except Exception as e:
        print(f"Fatal error: {e}", file=sys.stderr)
        sys.exit(1)
