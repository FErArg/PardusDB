"""
Model Context Windows Database
  sourced from litellm model_prices_and_context_window.json
  Updated: 2026-05-06
"""

MODEL_CONTEXT_BUNDLED = {
    # OpenAI (447 models)
    "openai/gpt-5.4": 1050000,
    "openai/gpt-5.4-2026-03-05": 1050000,
    "openai/gpt-5.4-mini": 1050000,
    "openai/gpt-5.5": 1050000,
    "openai/gpt-5.5-2026-04-23": 1050000,
    "openai/gpt-5.5-pro": 1050000,
    "openai/gpt-4.1": 1047576,
    "openai/gpt-4.1-mini": 1047576,
    "openai/gpt-4.1-nano": 1047576,
    "openai/gpt-4o": 128000,
    "openai/gpt-4o-mini": 128000,
    "openai/gpt-4o-2024-08-06": 128000,
    "openai/gpt-4o-2024-11-20": 128000,
    "openai/gpt-4-turbo": 128000,
    "openai/gpt-3.5-turbo": 16385,
    "openai/gpt-realtime": 32000,
    "azure/gpt-5.4": 1050000,
    "azure/gpt-5.5": 1050000,
    "azure/gpt-4.1": 1047576,
    "azure/gpt-4o": 128000,
    "azure/gpt-4o-mini": 128000,
    "azure/gpt-4-turbo": 128000,

    # Anthropic (251 models)
    "anthropic/claude-opus-4-6-v1": 1000000,
    "anthropic/claude-opus-4-7": 1000000,
    "anthropic/claude-sonnet-4-6": 1000000,
    "anthropic/claude-sonnet-4-20250514-v1:0": 1000000,
    "anthropic/claude-3-5-sonnet-20241022-v2:0": 1000000,
    "anthropic/claude-3-5-sonnet-20240620-v1:0": 1000000,
    "anthropic/claude-haiku-4-5-20251001-v1:0": 1000000,
    "anthropic/claude-opus-4-5-20251101-v1:0": 1000000,
    "claude-3-5-sonnet-20240620-v1:0": 1000000,
    "claude-3-5-sonnet-20241022-v2:0": 1000000,
    "claude-opus-4-6": 1000000,
    "claude-opus-4-7": 1000000,
    "claude-sonnet-4-6": 1000000,
    "claude-sonnet-4-20250514": 1000000,
    "claude-3-haiku-20240307-v1:0": 200000,
    "claude-3-opus-20240229-v1:0": 200000,
    "claude-3-sonnet-20240229-v1:0": 200000,
    "claude-instant-v1": 200000,

    # Google/Gemini (52 models)
    "gemini/gemini-exp-1206": 2097152,
    "gemini/gemini-exp-1114": 1048576,
    "gemini/gemini-2.0-flash": 1048576,
    "gemini/gemini-2.0-flash-001": 1048576,
    "gemini/gemini-2.0-flash-lite": 1048576,
    "gemini/gemini-2.5-flash": 1048576,
    "gemini/gemini-2.5-flash-lite": 1048576,
    "gemini/gemini-2.5-pro": 1048576,
    "gemini/gemini-2.5-pro-preview-tts": 1048576,
    "gemini/gemini-3-pro-preview": 1048576,
    "gemini/gemini-3.1-pro-preview": 1048576,
    "gemini/gemini-3.1-flash-lite-preview": 1048576,
    "gemini/gemini-flash-latest": 1048576,
    "gemini/gemini-flash-lite-latest": 1048576,
    "gemini/gemini-pro-latest": 1048576,
    "gemini/gemini-1.5-flash": 8192,
    "gemini/gemini-gemma-2-27b-it": 8192,
    "gemini/gemini-gemma-2-9b-it": 8192,
    "gemini/gemma-3-27b-it": 131072,

    # DeepSeek (30 models)
    "deepseek/deepseek-v3.2": 163840,
    "deepseek/deepseek-chat": 131072,
    "deepseek/deepseek-reasoner": 131072,
    "deepseek/deepseek-r1": 65536,
    "deepseek/deepseek-v3": 65536,
    "deepseek/deepseek-coder": 128000,

    # MiniMax (14 models)
    "minimax/MiniMax-M2.1": 1000000,
    "minimax/MiniMax-M2.1-lightning": 1000000,
    "minimax/MiniMax-M2.5": 1000000,
    "minimax/MiniMax-M2.5-lightning": 1000000,
    "minimax/MiniMax-M2": 200000,

    # Qwen/Alibaba (90 models)
    "qwen/qwen3.5-flash-02-23": 1000000,
    "qwen/qwen3.5-plus-02-15": 1000000,
    "qwen/qwen3-coder-plus": 997952,
    "qwen/qwen3-coder": 262100,
    "qwen/qwen3-235b-a22b-2507": 262144,
    "qwen/qwen3-235b-a22b-thinking-2507": 262144,
    "qwen/qwen3.5-35b-a3b": 262144,
    "qwen/qwen3.5-27b": 262144,
    "qwen/qwen3.5-122b-a10b": 262144,
    "qwen/qwen3.5-397b-a17b": 262144,
    "qwen/qwen2.5-coder-32b-instruct": 32768,
    "qwen/qwen-vl-plus": 8192,
    "qwen/qwen3-235b-a22b-2507": 262144,
    "alibaba/qwen": 8192,

    # Z.ai (25 models)
    "zai/glm-4.7": 204800,
    "zai/glm-4.6": 202800,
    "zai/glm-5": 202752,
    "zai/glm-4.5": 131072,
    "zai/glm-4.5v": 65536,
    "openrouter/z-ai/glm-4.7": 200000,
    "openrouter/z-ai/glm-4.6": 202800,
    "openrouter/z-ai/glm-5": 202752,
    "vertex_ai/zai-org/glm-4.7": 200000,
    "vertex_ai/zai-org/glm-5": 200000,
}

# Provider groups for model detection
MODEL_PROVIDERS = {
    "openai": ["openai/", "gpt-"],
    "anthropic": ["anthropic/", "claude"],
    "gemini": ["gemini/"],
    "deepseek": ["deepseek/"],
    "minimax": ["minimax/"],
    "qwen": ["qwen/"],
    "alibaba": ["qwen/"],
    "zai": ["z-ai", "zai/", "glm-"],
}

# Module-level cache for merged model context (bundled + cached/fetched)
_model_context_cache: dict = {}
_model_context_initialized: bool = False


def _ensure_initialized():
    """Ensure the model context cache is initialized."""
    global _model_context_cache, _model_context_initialized
    if not _model_context_initialized:
        _model_context_cache = dict(MODEL_CONTEXT_BUNDLED)
        _model_context_initialized = True


def refresh_model_context(verbose: bool = False) -> dict:
    """
    Manually refresh model context from litellm GitHub repository.

    Merges fetched data with bundled defaults (fetched takes precedence).

    Returns:
        Merged dictionary of all known models.
    """
    global _model_context_cache

    _ensure_initialized()

    try:
        from .model_fetch import fetch_model_context, get_cache_age_days

        data, was_fetched = fetch_model_context()

        if was_fetched:
            _model_context_cache = {**MODEL_CONTEXT_BUNDLED, **data}
            if verbose:
                print(f"Model context refreshed from litellm: {len(data)} models fetched, {len(_model_context_cache)} total")
        else:
            cached_data, age_days = get_cache_age_days()
            if cached_data:
                _model_context_cache = {**MODEL_CONTEXT_BUNDLED, **cached_data}
                if verbose:
                    print(f"Using cached model context (age: {age_days} days), {len(_model_context_cache)} total models")
            else:
                if verbose:
                    print("No cached model context available, using bundled defaults")
                _model_context_cache = dict(MODEL_CONTEXT_BUNDLED)

    except Exception as e:
        if verbose:
            print(f"Warning: Failed to refresh model context: {e}. Using cached data.")
        _ensure_initialized()

    return _model_context_cache


def get_context_window_for_model(model_name: str) -> int:
    """
    Get context window for a model by name.
    Falls back to common defaults if exact model not found.
    """
    _ensure_initialized()

    model_lower = model_name.lower()

    # Direct match in cache
    for key, ctx in _model_context_cache.items():
        if key.lower() == model_lower:
            return ctx

    # Partial match in cache
    for key, ctx in _model_context_cache.items():
        if key.lower() in model_lower or model_lower in key.lower():
            return ctx

    # Fallback based on provider patterns
    if "gpt-5" in model_lower:
        return 1050000
    if "gpt-4.1" in model_lower:
        return 1047576
    if "gpt-4o" in model_lower:
        return 128000
    if "claude" in model_lower:
        return 1000000
    if "gemini" in model_lower:
        return 1048576
    if "deepseek" in model_lower:
        return 131072
    if "minimax" in model_lower:
        return 1000000
    if "qwen" in model_lower or "qwq" in model_lower:
        return 262144
    if "glm-" in model_lower:
        return 200000

    # Default fallback
    return 128000


def detect_provider(model_name: str) -> str:
    """Detect provider from model name."""
    model_lower = model_name.lower()
    for provider, patterns in MODEL_PROVIDERS.items():
        if any(p in model_lower for p in patterns):
            return provider
    return "unknown"


# Default tokens per chunk (used for estimation)
DEFAULT_TOKENS_PER_CHUNK = 300
DEFAULT_AVG_CHUNK_SIZE_CHARS = 1200
