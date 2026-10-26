"""
Pytest configuration and shared fixtures for PardusDB tests
"""

import pytest
import subprocess
import tempfile
from pathlib import Path

PARDUSDB = "pardusdb"


@pytest.fixture(scope="session")
def pardusdb_binary():
    """Verify pardusdb binary is available"""
    try:
        subprocess.run([PARDUSDB, "--version"], capture_output=True, timeout=5)
        return PARDUSDB
    except FileNotFoundError:
        pytest.skip("pardusdb binary not found in PATH")


@pytest.fixture(scope="session")
def temp_workspace():
    """Create a temporary workspace for tests"""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield tmpdir


def run_pardus(command, db_path=None, timeout=30):
    """Helper function to run pardusdb commands"""
    args = [PARDUSDB]
    if db_path:
        args.append(db_path)
    try:
        proc = subprocess.run(
            args,
            input=f"{command}\nquit\n".encode(),
            capture_output=True,
            timeout=timeout
        )
        return proc.stdout.decode() + proc.stderr.decode()
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    except FileNotFoundError:
        return "BINARY_NOT_FOUND"
