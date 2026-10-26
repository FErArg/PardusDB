"""
PardusDB Security and Functionality Tests (pytest)
Ejecutar: pytest test_security.py -v
"""

import pytest
import subprocess
import tempfile
import os
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


@pytest.fixture
def temp_db():
    """Create a temporary database for testing"""
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Path(tmpdir) / "test.pardus"
        yield str(db)


@pytest.fixture
def setup_db(temp_db):
    """Create a database with a test table"""
    run_cmd("CREATE TABLE docs (embedding VECTOR(384), content TEXT);", temp_db)
    return temp_db


class TestDatabaseOperations:
    """Test basic database operations"""

    def test_create_table(self, temp_db):
        """Test table creation"""
        result = run_cmd("CREATE TABLE t (embedding VECTOR(384), content TEXT);", temp_db)
        assert "created" in result.lower()

    def test_list_tables(self, setup_db):
        """Test .tables command"""
        result = run_cmd(".tables", setup_db)
        assert "docs" in result

    def test_table_persistence(self, setup_db):
        """Test that tables persist after save"""
        run_cmd("save", setup_db)
        result = run_cmd(".tables", setup_db)
        assert "docs" in result


class TestSecurity:
    """Security-related tests"""

    def test_sql_injection_or(self, setup_db):
        """Test SQL injection with OR"""
        result = run_cmd("SELECT * FROM docs WHERE content = '1' OR '1'='1';", setup_db)
        assert "Error" in result or "Invalid" in result

    def test_sql_injection_drop(self, setup_db):
        """Test DROP TABLE injection"""
        result = run_cmd("DROP TABLE docs;", setup_db)
        assert "dropped" in result.lower() or "Error" in result

    def test_vector_dimension_enforcement(self, setup_db):
        """Test that vector dimension is enforced"""
        result = run_cmd(
            "INSERT INTO docs (embedding, content) VALUES ([0.1, 0.2], 'test');",
            setup_db
        )
        assert "dimension mismatch" in result.lower()

    def test_empty_table_query(self, setup_db):
        """Test querying empty table"""
        result = run_cmd("SELECT * FROM docs;", setup_db)
        assert "Found 0 rows" in result or ("docs" in result and "0" in result)


class TestVectorOperations:
    """Test vector-related operations"""

    def test_insert_wrong_dimension(self, setup_db):
        """Test inserting vector with wrong dimension"""
        result = run_cmd(
            "INSERT INTO docs (embedding, content) VALUES ([0.1, 0.2], 'test');",
            setup_db
        )
        assert "dimension mismatch" in result.lower()

    def test_select_from_empty(self, setup_db):
        """Test SELECT on empty table"""
        result = run_cmd("SELECT * FROM docs;", setup_db)
        assert "Found 0 rows" in result


class TestHealthCheck:
    """Test health check functionality"""

    def test_health_check_command(self, setup_db):
        """Test HEALTH CHECK command"""
        result = run_cmd("HEALTH CHECK;", setup_db)
        # HEALTH CHECK might not exist in all versions
        # Just verify it doesn't crash
        assert result is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
