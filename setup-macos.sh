#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="0.4.33"
BINARY_NAME="pardusdb"
HELPER_NAME="pardus"
INSTALL_DIR="$HOME/.local/bin"
PARDUS_HOME="$HOME/.pardus"
CONFIG_DIR="$HOME/.config/pardus"
DATA_DIR="$PARDUS_HOME"
MCP_DIR="$PARDUS_HOME/mcp"
BIN_OUT_DIR="$SCRIPT_DIR/bin"
PYTHON_BIN="python3"

show_help() {
    cat << EOF
PardusDB v${VERSION} - macOS source installer

USAGE:
    ./setup-macos.sh [OPTION]

OPTIONS:
    --install     Build and install PardusDB for macOS (default)
    --uninstall   Remove installed PardusDB files, including ~/.pardus/
    --help        Show this help

INSTALLATION:
    Builds the Rust binary from source, installs the pardusdb binary,
    the 'pardus' helper, the MCP server in a Python virtual environment,
    config files, and the Python SDK.

    Install paths:
      - Binary:     ~/.local/bin/pardusdb
      - Helper:     ~/.local/bin/pardus
      - Databases:  ~/.pardus/
      - MCP Server: ~/.pardus/mcp/

EOF
}

detect_shell() {
    case "$(basename "$SHELL")" in
        bash) SHELL_RC="$HOME/.bashrc" ;;
        zsh)  SHELL_RC="$HOME/.zshrc" ;;
        fish) SHELL_RC="$HOME/.config/fish/config.fish" ;;
        *)    SHELL_RC="" ;;
    esac
}

ensure_macos() {
    if [ "$(uname -s)" != "Darwin" ]; then
        echo "ERROR: setup-macos.sh is only for macOS. Use ./setup.sh on other systems."
        exit 1
    fi
}

version_ge() {
    python3 - "$1" "$2" << 'PY'
import sys

def parse(version):
    return tuple(int(part) for part in version.split(".")[:3])

sys.exit(0 if parse(sys.argv[1]) >= parse(sys.argv[2]) else 1)
PY
}

install_rust() {
    echo ""
    echo "Rust not found. Installing Rust..."
    echo ""

    if command -v curl &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    else
        echo "ERROR: curl not found. Install Rust manually from https://rustup.rs/"
        exit 1
    fi

    if [ -f "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
        echo "Rust installed successfully."
    else
        echo "ERROR: Rust installation failed."
        exit 1
    fi
}

select_python() {
    if ! command -v python3 &> /dev/null; then
        echo "ERROR: python3 not found. Install Python 3.10+ and retry."
        exit 1
    fi

    local py_version
    py_version=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>/dev/null || echo "0.0")

    if version_ge "$py_version" "3.10"; then
        PYTHON_BIN="python3"
        return
    fi

    echo "ERROR: Python $py_version detected. The MCP Python package requires Python 3.10+."
    echo ""

    if ! command -v brew &> /dev/null; then
        echo "Homebrew not found. Install Python 3.10+ manually, then retry."
        echo "Recommended: https://brew.sh/ then brew install python@3.13"
        exit 1
    fi

    local brew_prefix
    brew_prefix=$(brew --prefix)
    local brew_python="$brew_prefix/opt/python@3.13/bin/python3.13"

    if [ ! -f "$brew_python" ]; then
        echo "Options for Python 3.10+ on macOS:"
        echo "  1) Install Python 3.13 with Homebrew: brew install python@3.13"
        echo "  2) Exit and install Python manually"
        echo ""
        read -p "  Option [1/2]: " py_option

        if [ "$py_option" = "1" ]; then
            echo "Installing python@3.13 with Homebrew..."
            brew install python@3.13
        else
            echo "Installation cancelled. Install Python 3.10+ and retry."
            exit 1
        fi
    fi

    if [ ! -f "$brew_python" ]; then
        echo "ERROR: $brew_python not found after Python installation."
        exit 1
    fi

    local new_py_version
    new_py_version=$("$brew_python" -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>/dev/null || echo "0.0")
    if ! version_ge "$new_py_version" "3.10"; then
        echo "ERROR: Python $new_py_version is still below 3.10."
        exit 1
    fi

    PYTHON_BIN="$brew_python"
    echo "Using Python $new_py_version at $PYTHON_BIN"
}

check_prerequisites() {
    echo "==================================="
    echo "   PardusDB v${VERSION} macOS Installer"
    echo "==================================="
    echo ""

    ensure_macos

    if ! command -v cargo &> /dev/null; then
        install_rust
    fi

    export PATH="$HOME/.cargo/bin:$PATH"

    if ! command -v cargo &> /dev/null; then
        echo "ERROR: cargo not found after Rust setup."
        exit 1
    fi

    select_python

    echo "Prerequisites verified."
    echo ""
}

build_binary() {
    echo "[1/10] Building Rust binary (release mode)..."

    export PATH="$HOME/.cargo/bin:$PATH"
    cargo build --release

    if [ ! -f "target/release/$BINARY_NAME" ]; then
        echo "ERROR: Rust build did not produce target/release/$BINARY_NAME"
        exit 1
    fi

    mkdir -p "$BIN_OUT_DIR"
    local binary_out="$BIN_OUT_DIR/pardus-v${VERSION}-darwin-$(uname -m)"
    cp "target/release/$BINARY_NAME" "$binary_out"

    echo "Binary built successfully."
    echo "  Binary saved to: $binary_out"
}

install_binary() {
    echo "[2/10] Installing binary..."

    mkdir -p "$INSTALL_DIR"
    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    if [[ ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
        echo "Binary installed at: $INSTALL_DIR/$BINARY_NAME (already in PATH)"
    else
        echo "Binary installed at: $INSTALL_DIR/$BINARY_NAME"
        echo "  Add '$INSTALL_DIR' to PATH if needed:"
        detect_shell
        if [ -n "$SHELL_RC" ]; then
            echo "  echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> $SHELL_RC"
            echo "  source $SHELL_RC"
        else
            echo "  echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.bashrc"
        fi
    fi
}

create_helper() {
    echo "[3/10] Creating 'pardus' helper..."

    cat > "$INSTALL_DIR/$HELPER_NAME" << 'HELPER_SCRIPT'
#!/bin/bash
DB_DIR="$HOME/.pardus"
DEFAULT_DB="$DB_DIR/pardus-rag.db"

mkdir -p "$DB_DIR"

if [ $# -eq 0 ]; then
    if [ ! -f "$DEFAULT_DB" ]; then
        mkdir -p "$DB_DIR"
        echo "Creating default database: $DEFAULT_DB"
        echo ".create $DEFAULT_DB" | pardusdb > /dev/null 2>&1
    fi
    exec pardusdb "$DEFAULT_DB"
else
    exec pardusdb "$@"
fi
HELPER_SCRIPT

    chmod +x "$INSTALL_DIR/$HELPER_NAME"
    echo "Helper installed at: $INSTALL_DIR/$HELPER_NAME"
}

create_config() {
    echo "[4/10] Creating config file..."

    mkdir -p "$CONFIG_DIR"

    cat > "$CONFIG_DIR/config.toml" << 'CONFIG_EOF'
# PardusDB Configuration File

[database]
default_path = "~/.pardus/pardus-rag.db"

[logging]
level = "info"
CONFIG_EOF

    echo "  Config: $CONFIG_DIR/config.toml"
}

pip_install_or_fail() {
    local label="$1"
    shift
    local log_file
    log_file=$(mktemp)

    if "$MCP_DIR/venv/bin/python" -m pip install "$@" > "$log_file" 2>&1; then
        rm -f "$log_file"
        return 0
    fi

    echo "ERROR: Failed to install $label in MCP virtual environment."
    echo "pip output:"
    sed 's/^/  /' "$log_file"
    rm -f "$log_file"
    exit 1
}

pip_install_optional() {
    local label="$1"
    shift
    local log_file
    log_file=$(mktemp)

    if "$MCP_DIR/venv/bin/python" -m pip install "$@" > "$log_file" 2>&1; then
        rm -f "$log_file"
        return 0
    fi

    echo "  WARNING: Could not install $label."
    echo "  Last pip output lines:"
    tail -n 20 "$log_file" | sed 's/^/    /'
    rm -f "$log_file"
    return 1
}

install_mcp() {
    echo "[5/10] Installing MCP server (Python venv)..."

    if [ ! -f "$SCRIPT_DIR/mcp/src/server.py" ]; then
        echo "ERROR: mcp/src/server.py not found."
        exit 1
    fi

    mkdir -p "$MCP_DIR/src"
    cp "$SCRIPT_DIR/mcp/src/"*.py "$MCP_DIR/src/"

    echo "  Creating virtual environment with $PYTHON_BIN..."
    rm -rf "$MCP_DIR/venv"
    "$PYTHON_BIN" -m venv "$MCP_DIR/venv"

    pip_install_or_fail "pip upgrade" --upgrade pip
    pip_install_or_fail "mcp" mcp

    if "$MCP_DIR/venv/bin/python" -c "from mcp.server import Server" 2>/dev/null; then
        echo "  - mcp (Python package): OK"
    else
        echo "ERROR: mcp installed but cannot import 'mcp.server.Server' from the venv."
        exit 1
    fi

    cat > "$MCP_DIR/run_mcp.sh" << 'WRAPPER_EOF'
#!/bin/bash
exec "$HOME/.pardus/mcp/venv/bin/python" "$HOME/.pardus/mcp/src/server.py" "$@"
WRAPPER_EOF
    chmod +x "$MCP_DIR/run_mcp.sh"

    echo "  MCP server installed at: $MCP_DIR/src/server.py"
    echo "  Wrapper: $MCP_DIR/run_mcp.sh"
}

install_document_dependencies() {
    echo "[6/10] Installing document import dependencies..."

    pip_install_optional "markitdown[all]" 'markitdown[all]' || pip_install_optional "markitdown" markitdown || true

    md_state=$("$MCP_DIR/venv/bin/python" -c "from markitdown import MarkItDown; print('OK')" 2>/dev/null || echo "not installed")
    echo "  - markitdown (venv): $md_state"
}

install_sentence_transformers() {
    echo "[7/10] Installing sentence-transformers for automatic embeddings..."

    echo -n "  Install sentence-transformers (recommended, ~80MB)? (s/N): "
    read -r answer
    if [ "$answer" != "s" ] && [ "$answer" != "S" ]; then
        echo "  Skipped. MCP will use zero vectors for automatic embeddings."
        return
    fi

    pip_install_or_fail "sentence-transformers" sentence-transformers
    echo "  - sentence-transformers: OK"
}

configure_opencode() {
    echo "[8/10] Configuring OpenCode..."

    local REAL_USER
    REAL_USER=$(logname 2>/dev/null || echo "$USER")

    echo -n "  Configure PardusDB MCP for OpenCode? (s/N): "
    read -r answer
    if [ "$answer" != "s" ] && [ "$answer" != "S" ]; then
        echo "  Skipped."
        return
    fi

    local OPCODE_CONFIG_DIR="$HOME/.config/opencode"
    local OPCODE_CONFIG="$OPCODE_CONFIG_DIR/opencode.json"
    local OPCODE_SKILLS_DIR="$HOME/.config/opencode/skills"
    local SKILL_SOURCE="$SCRIPT_DIR/skill/skill.md"
    local MCP_PATH="$MCP_DIR/run_mcp.sh"

    if [ -f "$SKILL_SOURCE" ]; then
        mkdir -p "$OPCODE_SKILLS_DIR"
        cp "$SKILL_SOURCE" "$OPCODE_SKILLS_DIR/pardusdb.md"
        echo "  Skill copied to: $OPCODE_SKILLS_DIR/pardusdb.md"
    fi

    if [ -f "$OPCODE_CONFIG" ]; then
        if python3 -c "
import json
with open('$OPCODE_CONFIG') as f:
    cfg = json.load(f)
exit(0 if 'pardusdb' in cfg.get('mcp', {}) else 1)
" 2>/dev/null; then
            echo "  Entry 'pardusdb' already exists in $OPCODE_CONFIG"
            echo "  Skipped."
            return
        fi

        python3 -c "
import json
with open('$OPCODE_CONFIG') as f:
    cfg = json.load(f)
if 'mcp' not in cfg:
    cfg['mcp'] = {}
cfg['mcp']['pardusdb'] = {
    'type': 'local',
    'command': ['$MCP_PATH'],
    'enabled': True
}
with open('$OPCODE_CONFIG', 'w') as f:
    json.dump(cfg, f, indent=2)
    f.write('\\n')
" 2>/dev/null && echo "  MCP configured in: $OPCODE_CONFIG" || echo "  ERROR: Could not update $OPCODE_CONFIG"
    else
        mkdir -p "$OPCODE_CONFIG_DIR"
        cat > "$OPCODE_CONFIG" << JSONEOF
{
  "\$schema": "https://opencode.ai/config.json",
  "mcp": {
    "pardusdb": {
      "type": "local",
      "command": ["$MCP_PATH"],
      "enabled": true
    }
  }
}
JSONEOF
        chown "$REAL_USER" "$OPCODE_CONFIG" 2>/dev/null || true
        echo "  Created: $OPCODE_CONFIG"
    fi

    echo "  Restart OpenCode for the changes to take effect."
}

install_python_sdk() {
    echo "[9/10] Installing Python SDK..."

    if [ ! -d "$SCRIPT_DIR/sdk/python" ]; then
        echo "  WARNING: sdk/python/ not found, skipping Python SDK"
        return
    fi

    if "$PYTHON_BIN" -m pip install -e "$SCRIPT_DIR/sdk/python" --quiet 2>/dev/null; then
        "$PYTHON_BIN" -c "import pardusdb" 2>/dev/null && echo "  Python SDK installed" || echo "  WARNING: Python SDK not importable"
    else
        echo "  WARNING: Could not install Python SDK with $PYTHON_BIN"
    fi
}

create_data_dir() {
    echo "[10/10] Creating data directory..."

    mkdir -p "$DATA_DIR"
    echo "  Data directory: $DATA_DIR/"
    echo "  Default database: $DATA_DIR/pardus-rag.db (created on first 'pardus' use)"
}

verify_installation() {
    echo ""
    echo "==================================="
    echo "   Installation Completed!"
    echo "==================================="
    echo ""
    echo "Installed files:"
    echo "  - $INSTALL_DIR/pardusdb    (main binary)"
    echo "  - $INSTALL_DIR/pardus      (helper, creates default DB)"
    echo "  - $MCP_DIR/                (MCP server and venv)"
    echo "  - $CONFIG_DIR/config.toml  (config)"
    echo ""
    echo "Quick usage:"
    echo "  pardus                    # Open default DB"
    echo "  pardusdb                  # Direct binary (in-memory)"
    echo "  pardusdb my.db            # Open/create specific file"
    echo ""
    echo "MCP dependencies:"
    mcp_state=$("$MCP_DIR/venv/bin/python" -c "from mcp.server import Server; print('OK')" 2>/dev/null || echo "not installed")
    md_state=$("$MCP_DIR/venv/bin/python" -c "from markitdown import MarkItDown; print('OK')" 2>/dev/null || echo "not installed")
    st_state=$("$MCP_DIR/venv/bin/python" -c "from sentence_transformers import SentenceTransformer; print('OK')" 2>/dev/null || echo "not installed")
    echo "  - mcp (venv): $mcp_state"
    echo "  - markitdown (venv): $md_state"
    echo "  - sentence-transformers (venv): $st_state"
    echo ""
    echo "For OpenCode MCP setup, see INSTALL.md"
    echo ""
}

do_install() {
    check_prerequisites
    build_binary
    install_binary
    create_helper
    create_config
    install_mcp
    install_document_dependencies
    install_sentence_transformers
    configure_opencode
    install_python_sdk
    create_data_dir
    verify_installation
}

do_uninstall() {
    ensure_macos
    echo "==================================="
    echo "   PardusDB v${VERSION} - Uninstall"
    echo "==================================="
    echo ""

    local removed=0

    if [ -f "$INSTALL_DIR/pardusdb" ]; then
        rm -f "$INSTALL_DIR/pardusdb"
        echo "  Removed: $INSTALL_DIR/pardusdb"
        removed=1
    fi

    if [ -f "$INSTALL_DIR/$HELPER_NAME" ]; then
        rm -f "$INSTALL_DIR/$HELPER_NAME"
        echo "  Removed: $INSTALL_DIR/$HELPER_NAME"
        removed=1
    fi

    if [ -d "$PARDUS_HOME" ]; then
        rm -rf "$PARDUS_HOME"
        echo "  Removed: $PARDUS_HOME/ (databases and MCP stats)"
        removed=1
    fi

    if [ -d "$CONFIG_DIR" ]; then
        rm -rf "$CONFIG_DIR"
        echo "  Removed: $CONFIG_DIR/"
        removed=1
    fi

    if [ $removed -eq 0 ]; then
        echo "No PardusDB installation found."
    else
        echo ""
        echo "Uninstall completed."
    fi
}

case "${1:-}" in
    --install|"")
        do_install
        ;;
    --uninstall)
        do_uninstall
        ;;
    --help|-h)
        show_help
        ;;
    *)
        echo "Unknown option: $1"
        echo "Use --help to see available options."
        exit 1
        ;;
esac
