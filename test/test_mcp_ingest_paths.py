"""Tests for MCP ingestion path resolution."""

import importlib.util
import sys
import types
from pathlib import Path

import pytest


def load_server_module():
    root = Path(__file__).resolve().parents[1]
    server_path = root / "mcp" / "src" / "server.py"

    mcp_module = types.ModuleType("mcp")
    mcp_server_module = types.ModuleType("mcp.server")
    mcp_stdio_module = types.ModuleType("mcp.server.stdio")
    mcp_types_module = types.ModuleType("mcp.types")

    class FakeServer:
        def __init__(self, *args, **kwargs):
            pass

        def list_tools(self):
            def decorator(func):
                return func

            return decorator

        def call_tool(self):
            def decorator(func):
                return func

            return decorator

        async def run(self, *args, **kwargs):
            return None

    class FakeTool:
        def __init__(self, *args, **kwargs):
            self.args = args
            self.kwargs = kwargs

    class FakeTextContent:
        def __init__(self, *args, **kwargs):
            self.args = args
            self.kwargs = kwargs

    async def fake_stdio_server(*args, **kwargs):
        return None

    mcp_server_module.Server = FakeServer
    mcp_stdio_module.stdio_server = fake_stdio_server
    mcp_types_module.Tool = FakeTool
    mcp_types_module.TextContent = FakeTextContent

    old_modules = {
        name: sys.modules.get(name)
        for name in ("mcp", "mcp.server", "mcp.server.stdio", "mcp.types")
    }
    sys.modules.update(
        {
            "mcp": mcp_module,
            "mcp.server": mcp_server_module,
            "mcp.server.stdio": mcp_stdio_module,
            "mcp.types": mcp_types_module,
        }
    )

    try:
        spec = importlib.util.spec_from_file_location("pardusdb_mcp_server_test", server_path)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        return module
    finally:
        for name, old_module in old_modules.items():
            if old_module is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = old_module


@pytest.fixture(scope="module")
def server_module():
    return load_server_module()


def test_relative_file_is_found_recursively(server_module, tmp_path):
    doc = tmp_path / "docs" / "manual.md"
    doc.parent.mkdir()
    doc.write_text("hello", encoding="utf-8")

    resolved = server_module._resolve_ingest_path("manual.md", project_root=tmp_path)

    assert resolved == doc.resolve()


def test_relative_directory_is_found_recursively(server_module, tmp_path):
    docs = tmp_path / "nested" / "docs"
    docs.mkdir(parents=True)

    resolved = server_module._resolve_ingest_path("docs", expect_dir=True, project_root=tmp_path)

    assert resolved == docs.resolve()


def test_relative_path_cannot_escape_project_root(server_module, tmp_path):
    with pytest.raises(ValueError, match="Path traversal not allowed"):
        server_module._resolve_ingest_path("../secret.md", project_root=tmp_path)


def test_symlink_escape_is_blocked_for_relative_path(server_module, tmp_path):
    outside = tmp_path.parent / "outside-ingest-target.md"
    outside.write_text("secret", encoding="utf-8")
    link = tmp_path / "link.md"
    link.symlink_to(outside)

    with pytest.raises(ValueError, match="escapes project root"):
        server_module._resolve_ingest_path("link.md", project_root=tmp_path)


def test_multiple_relative_matches_require_specific_path(server_module, tmp_path):
    first = tmp_path / "a" / "manual.md"
    second = tmp_path / "b" / "manual.md"
    first.parent.mkdir()
    second.parent.mkdir()
    first.write_text("one", encoding="utf-8")
    second.write_text("two", encoding="utf-8")

    with pytest.raises(ValueError, match="Multiple matches"):
        server_module._resolve_ingest_path("manual.md", project_root=tmp_path)


def test_absolute_file_may_be_outside_project_root(server_module, tmp_path):
    project_root = tmp_path / "project"
    project_root.mkdir()
    outside = tmp_path / "outside.md"
    outside.write_text("allowed", encoding="utf-8")

    resolved = server_module._resolve_ingest_path(str(outside), project_root=project_root)

    assert resolved == outside.resolve()
