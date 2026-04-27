#!/usr/bin/env python3
"""
PardusDB MCP Server

Model Context Protocol server for PardusDB vector database.
Enables AI agents to perform vector similarity search and manage vector data.
"""

import asyncio
import os
import sys
from pathlib import Path
from typing import Any, Optional

try:
    from mcp.server import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import Tool, TextContent
except ImportError:
    print("Error: mcp package not found. Install with: pip install mcp", file=sys.stderr)
    sys.exit(1)


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
            "str": "TEXT",
            "string": "TEXT",
            "int": "INTEGER",
            "integer": "INTEGER",
            "float": "FLOAT",
            "bool": "BOOLEAN",
            "text": "TEXT",
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

        id_match = result.split("id=")[-1].split(",")[0].split(")")[0] if "id=" in result else "unknown"

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

            if "id=" in result:
                id_match = result.split("id=")[-1].split(",")[0].split(")")[0]
                results.append(id_match)

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


# ==================== Tool Definitions ====================

TOOLS = [
    Tool(
        name="pardusdb_create_database",
        description="Create a new PardusDB database file at the specified path",
        inputSchema={
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path for the new .pardus database file (e.g., 'data/vectors.pardus')",
                },
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
                "path": {
                    "type": "string",
                    "description": "Path to the existing .pardus database file",
                },
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
                "name": {
                    "type": "string",
                    "description": "Name of the table",
                },
                "vector_dim": {
                    "type": "number",
                    "description": "Dimension of the vectors (e.g., 768 for sentence transformers)",
                },
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
                "vector": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "The embedding vector (array of floats)",
                },
                "metadata": {
                    "type": "object",
                    "description": "Optional metadata to store with the vector",
                },
                "table": {
                    "type": "string",
                    "description": "Table name (uses current table if not specified)",
                },
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
                    "items": {
                        "type": "array",
                        "items": {"type": "number"},
                    },
                    "description": "Array of embedding vectors",
                },
                "metadata_list": {
                    "type": "array",
                    "items": {"type": "object"},
                    "description": "Optional array of metadata objects (one per vector)",
                },
                "table": {
                    "type": "string",
                    "description": "Table name (uses current table if not specified)",
                },
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
                "query_vector": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "The query embedding vector",
                },
                "k": {
                    "type": "number",
                    "description": "Number of results to return (default: 10)",
                },
                "table": {
                    "type": "string",
                    "description": "Table name (uses current table if not specified)",
                },
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
                "sql": {
                    "type": "string",
                    "description": "SQL command to execute",
                },
            },
            "required": ["sql"],
        },
    ),
    Tool(
        name="pardusdb_list_tables",
        description="List all tables in the current database",
        inputSchema={
            "type": "object",
            "properties": {},
        },
    ),
    Tool(
        name="pardusdb_use_table",
        description="Set the current table for subsequent operations",
        inputSchema={
            "type": "object",
            "properties": {
                "table": {
                    "type": "string",
                    "description": "Name of the table to use",
                },
            },
            "required": ["table"],
        },
    ),
    Tool(
        name="pardusdb_status",
        description="Get the current status of the database connection",
        inputSchema={
            "type": "object",
            "properties": {},
        },
    ),
]


# ==================== Server Setup ====================

server = Server("pardusdb-mcp", "0.4.0")


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