#!/bin/bash
# PardusDB MCP Server Launcher
# Supports: virtual env activation, env vars, custom Python

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_SERVER="${SCRIPT_DIR}/src/server.py"
VENV_DIR="${HOME}/.pardus/mcp/venv"
PYTHON_CMD="${PARDUSDB_PYTHON:-python3}"

# Use venv Python if available (macOS style)
if [ -d "$VENV_DIR" ] && [ -f "${VENV_DIR}/bin/python" ]; then
    PYTHON_CMD="${VENV_DIR}/bin/python"
fi

# Verify server exists
if [ ! -f "$MCP_SERVER" ]; then
    echo "Error: MCP server not found at $MCP_SERVER" >&2
    exit 1
fi

# Execute
exec "$PYTHON_CMD" "$MCP_SERVER" "$@"