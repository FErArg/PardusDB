#!/usr/bin/env python3
"""
PardusDB Security and Functionality Tests
"""

import subprocess
import tempfile
import os
import sys
from pathlib import Path

PARDUSDB = "pardusdb"
TIMEOUT = 30

def run_cmd(cmd, db_path=None):
    """Execute pardusdb command"""
    args = [PARDUSDB]
    if db_path:
        args.append(db_path)
    try:
        proc = subprocess.run(
            args,
            input=f"{cmd}\nquit\n".encode(),
            capture_output=True,
            timeout=TIMEOUT
        )
        return proc.stdout.decode() + proc.stderr.decode()
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    except FileNotFoundError:
        return "BINARY_NOT_FOUND"

def test_create_and_open():
    """Test database create and open"""
    print("TEST: Create and Open Database")
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Path(tmpdir) / "test.pardus"
        result = run_cmd(f"CREATE TABLE docs (embedding VECTOR(384), content TEXT);", str(db))
        if "created" in result.lower():
            print("  ✓ CREATE TABLE works")
        else:
            print(f"  ✗ CREATE TABLE failed: {result[:100]}")
        
        result = run_cmd(".tables", str(db))
        if "docs" in result:
            print("  ✓ .tables shows created table")
        else:
            print(f"  ✗ .tables failed: {result[:100]}")

def test_sql_injection():
    """Test SQL injection prevention"""
    print("\nTEST: SQL Injection Prevention")
    payloads = [
        "' OR '1'='1",
        "'; DROP TABLE docs; --",
        "1' UNION SELECT * FROM users--",
        "admin'--",
    ]
    
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Path(tmpdir) / "test.pardus"
        run_cmd(f"CREATE TABLE docs (embedding VECTOR(384), content TEXT);", str(db))
        
        for payload in payloads:
            result = run_cmd(f"SELECT * FROM docs WHERE content = '{payload}';", str(db))
            if "Error" in result or "Invalid" in result:
                print(f"  ✓ Injection blocked: {payload[:30]}")
            else:
                print(f"  ✗ Possible injection: {payload[:30]}")

def test_path_traversal():
    """Test path traversal prevention"""
    print("\nTEST: Path Traversal Prevention")
    dangerous_paths = [
        "../../../etc/passwd",
        "/tmp/../../../etc/passwd",
        "../../../../../../etc/passwd",
    ]
    
    for path in dangerous_paths:
        result = run_cmd(f"CREATE TABLE t (embedding VECTOR(384));", path)
        if "Error" in result or "not found" in result.lower():
            print(f"  ✓ Path blocked: {path[:40]}")
        else:
            print(f"  ✗ Possible traversal: {path[:40]}")

def test_vector_dimension_enforcement():
    """Test that vector dimension is enforced"""
    print("\nTEST: Vector Dimension Enforcement")
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Path(tmpdir) / "test.pardus"
        run_cmd(f"CREATE TABLE docs (embedding VECTOR(384), content TEXT);", str(db))
        
        result = run_cmd("INSERT INTO docs (embedding, content) VALUES ([0.1, 0.2], 'test');", str(db))
        if "dimension mismatch" in result.lower():
            print("  ✓ Dimension mismatch detected (expected 384, got 3)")
        else:
            print(f"  ✗ No dimension check: {result[:100]}")

def test_large_file_handling():
    """Test handling of oversized input"""
    print("\nTEST: Large Vector/Query Handling")
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Path(tmpdir) / "test.pardus"
        run_cmd(f"CREATE TABLE docs (embedding VECTOR(384), content TEXT);", str(db))
        
        # Create very large vector
        large_vec = ", ".join(["0.1"] * 10000)
        result = run_cmd(f"INSERT INTO docs (embedding, content) VALUES ([{large_vec}], 'test');", str(db))
        if "Error" in result or "dimension" in result.lower():
            print("  ✓ Large vector rejected")
        else:
            print(f"  ? Large vector accepted (check manually)")

def test_health_check():
    """Test health check command"""
    print("\nTEST: Health Check")
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Path(tmpdir) / "test.pardus"
        run_cmd(f"CREATE TABLE docs (embedding VECTOR(384), content TEXT);", str(db))
        run_cmd(f"INSERT INTO docs (embedding, content) VALUES ([{', '.join(['0.1']*384)}], 'test');", str(db))
        
        result = run_cmd("HEALTH CHECK;", str(db))
        if "ok" in result.lower() or "healthy" in result.lower() or "Error" not in result:
            print("  ✓ Health check executed")
        else:
            print(f"  ? Health check result: {result[:100]}")

def main():
    print("=" * 60)
    print("PardusDB Security and Functionality Tests")
    print("=" * 60)
    
    test_create_and_open()
    test_sql_injection()
    test_path_traversal()
    test_vector_dimension_enforcement()
    test_large_file_handling()
    test_health_check()
    
    print("\n" + "=" * 60)
    print("Tests completed")
    print("=" * 60)

if __name__ == "__main__":
    main()
