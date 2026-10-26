"""
Fetch and cache model context from litellm GitHub repository.
"""

import json
import time
import urllib.request
from pathlib import Path
from typing import Tuple

MODEL_CONTEXT_URL = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json"
CACHE_FILE = Path.home() / ".pardus" / "mcp_model_context.json"
CACHE_STALE_WARN_DAYS = 7


def fetch_model_context() -> Tuple[dict, bool]:
    """
    Fetch model context from litellm GitHub.

    Returns:
        Tuple of (data_dict, was_fetched).
        was_fetched is True if fresh data was obtained, False if using fallback.
        If fetch fails and no cache exists, returns ({}, False).
    """
    try:
        with urllib.request.urlopen(MODEL_CONTEXT_URL, timeout=15) as response:
            data = json.loads(response.read().decode())

        if isinstance(data, dict) and "data" in data:
            data = data["data"]

        if not isinstance(data, dict):
            data = {}

        CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
        with open(CACHE_FILE, 'w') as f:
            json.dump(data, f, indent=2)

        return data, True

    except Exception as e:
        if CACHE_FILE.exists():
            try:
                with open(CACHE_FILE) as f:
                    return json.load(f), False
            except Exception:
                pass

        return {}, False


def get_cached_context() -> Tuple[dict, int]:
    """
    Get cached model context with age.

    Returns:
        Tuple of (data_dict, cache_age_days).
        cache_age_days is 0 if no cache exists.
    """
    if not CACHE_FILE.exists():
        return {}, 0

    age = time.time() - CACHE_FILE.stat().st_mtime
    age_days = int(age / (24 * 60 * 60))

    try:
        with open(CACHE_FILE) as f:
            return json.load(f), age_days
    except Exception:
        return {}, 0


def is_cache_stale() -> bool:
    """Check if the cache is older than the stale warning threshold."""
    _, age_days = get_cached_context()
    return age_days >= CACHE_STALE_WARN_DAYS


def get_cache_age_days() -> int:
    """Get the age of the cache in days."""
    _, age_days = get_cached_context()
    return age_days
