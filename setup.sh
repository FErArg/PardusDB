#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="0.2.2"
BINARY_NAME="pardusdb"
HELPER_NAME="pardus"
INSTALL_DIR="$HOME/.local/bin"
PARDUS_HOME="$HOME/.pardus"
CONFIG_DIR="$HOME/.config/pardus"
DATA_DIR="$PARDUS_HOME"
MCP_DIR="$PARDUS_HOME/mcp"
BIN_OUT_DIR="$SCRIPT_DIR/bin"

show_help() {
    cat << EOF
PardusDB v${VERSION} - Instalador

USO:
    ./setup.sh [OPCION]

OPCIONES:
    --install     Instalar PardusDB (por defecto)
    --uninstall   Desinstalar PardusDB completamente
    --help        Mostrar esta ayuda

INSTALACION:
    Instala el binario pardusdb, el helper 'pardus',
    el servidor MCP para agentes AI, y el SDK Python.

    Rutas de instalacion:
      - Binario:     ~/.local/bin/pardusdb
      - Helper:      ~/.local/bin/pardus
      - Datos BD:    ~/.pardus/
      - MCP Server:  ~/.pardus/mcp/

DESINSTALACION:
    Elimina todos los archivos instalados incluyendo
    las bases de datos almacenadas en ~/.pardus/

EOF
}

detect_platform() {
    case "$(uname -s)" in
        Linux*)  PLATFORM="linux" ;;
        Darwin*) PLATFORM="macos" ;;
        *)      PLATFORM="unknown" ;;
    esac
}

detect_shell() {
    case "$(basename "$SHELL")" in
        bash) SHELL_RC="$HOME/.bashrc" ;;
        zsh)  SHELL_RC="$HOME/.zshrc" ;;
        fish) SHELL_RC="$HOME/.config/fish/config.fish" ;;
        *)    SHELL_RC="" ;;
    esac
}

install_rust() {
    echo ""
    echo "Rust no encontrado. Instalando Rust..."
    echo ""

    if command -v curl &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    else
        echo "ERROR: curl no encontrado. No se puede instalar Rust automaticamente."
        echo "Por favor instala Rust manualmente desde https://rustup.rs/"
        exit 1
    fi

    if [ -f "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
        echo ""
        echo "Rust instalado correctamente."
    else
        echo "ERROR: La instalacion de Rust fallo."
        exit 1
    fi
}

check_prerequisites() {
    echo "==================================="
    echo "   PardusDB v${VERSION} Installer"
    echo "==================================="
    echo ""

    detect_platform
    echo "  Plataforma detectada: $PLATFORM"
    echo ""

    if ! command -v cargo &> /dev/null; then
        install_rust
    fi

    export PATH="$HOME/.cargo/bin:$PATH"

    local missing=()

    if ! command -v cargo &> /dev/null; then
        missing+=("Rust (cargo) - error critico tras instalacion")
    fi

    if ! command -v node &> /dev/null; then
        missing+=("Node.js (node) - instalar desde https://nodejs.org/")
    fi

    if ! command -v python3 &> /dev/null; then
        missing+=("Python 3 (python3) - instalar desde https://python.org/")
    fi

    if ! command -v npm &> /dev/null; then
        missing+=("npm - se instala con Node.js")
    fi

    if [ ${#missing[@]} -ne 0 ]; then
        echo "ERROR: Faltan prerrequisitos:"
        for item in "${missing[@]}"; do
            echo "  - $item"
        done
        echo ""
        echo "Por favor instale los prerrequisitos faltantes e intente de nuevo."
        exit 1
    fi

    local node_version=$(node -v | sed 's/v//' | cut -d. -f1)
    if [ "$node_version" -lt 18 ]; then
        echo "ERROR: Node.js 18+ requerido. Version actual: $(node -v)"
        exit 1
    fi

    echo "Prerrequisitos verificados."
    echo ""
}

build_binary() {
    echo "[1/7] Construyendo binario Rust (release mode)..."

    export PATH="$HOME/.cargo/bin:$PATH"

    cargo build --release 2>/dev/null

    if [ ! -f "target/release/$BINARY_NAME" ]; then
        echo "Error: La compilacion del binario falló."
        echo "Verifique que Rust esté correctamente instalado e intente de nuevo."
        exit 1
    fi
    echo "Binario construido correctamente."

    echo ""
    echo "[1/7] Guardando binario en bin/pardus-v${VERSION}..."
    mkdir -p "$BIN_OUT_DIR"
    cp "target/release/$BINARY_NAME" "$BIN_OUT_DIR/pardus-v${VERSION}"
    echo "  Binario guardado en: $BIN_OUT_DIR/pardus-v${VERSION}"
}

install_binary() {
    echo "[2/7] Instalando binario..."

    mkdir -p "$INSTALL_DIR"

    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        rm -f "$INSTALL_DIR/$BINARY_NAME"
    fi

    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    if [[ ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
        echo "Binario instalado en: $INSTALL_DIR/$BINARY_NAME (ya en PATH)"
    else
        echo "Binario instalado en: $INSTALL_DIR/$BINARY_NAME"
        echo "  ANADE '$INSTALL_DIR' A TU PATH si no está ya:"
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
    echo "[3/7] Creando helper 'pardus'..."

    cat > "$INSTALL_DIR/$HELPER_NAME" << 'HELPER_SCRIPT'
#!/bin/bash
DB_DIR="$HOME/.pardus"
DEFAULT_DB="$DB_DIR/pardus-rag.db"

mkdir -p "$DB_DIR"

if [ $# -eq 0 ]; then
    if [ ! -f "$DEFAULT_DB" ]; then
        mkdir -p "$DB_DIR"
        echo "Creando base de datos por defecto: $DEFAULT_DB"
        echo ".create $DEFAULT_DB" | pardusdb > /dev/null 2>&1
    fi
    exec pardusdb "$DEFAULT_DB"
else
    exec pardusdb "$@"
fi
HELPER_SCRIPT

    chmod +x "$INSTALL_DIR/$HELPER_NAME"
    echo "Helper instalado en: $INSTALL_DIR/$HELPER_NAME"
}

create_config() {
    echo "[4/7] Creando archivo de configuracion..."

    mkdir -p "$CONFIG_DIR"

    cat > "$CONFIG_DIR/config.toml" << 'CONFIG_EOF'
# PardusDB Configuration File

[database]
default_path = "~/.pardus/pardus-rag.db"

[logging]
level = "info"
CONFIG_EOF

    echo "  Configuracion en: $CONFIG_DIR/config.toml"
}

install_mcp() {
    echo "[5/7] Instalando servidor MCP..."

    if [ ! -d "$SCRIPT_DIR/mcp" ]; then
        echo "  ADVERTENCIA: Directorio mcp/ no encontrado, saltando MCP server"
        return
    fi

    cd "$SCRIPT_DIR/mcp"

    if [ -d "node_modules" ]; then
        rm -rf node_modules
    fi

    npm install --silent 2>/dev/null

    if [ ! -f "dist/index.js" ]; then
        npm run build 2>/dev/null
    fi

    if [ ! -f "dist/index.js" ]; then
        echo "  ADVERTENCIA: MCP server no pudo ser construido"
        cd "$SCRIPT_DIR"
        return
    fi

    mkdir -p "$MCP_DIR"

    if [ -d "$MCP_DIR/dist" ]; then
        rm -rf "$MCP_DIR/dist"
    fi

    cp -r dist "$MCP_DIR/"
    cp package.json "$MCP_DIR/"

    if [ -d "node_modules" ]; then
        cp -r node_modules "$MCP_DIR/"
    fi

    cd "$SCRIPT_DIR"
    echo "  MCP server instalado en: $MCP_DIR/"
}

install_python_sdk() {
    echo "[6/7] Instalando SDK Python..."

    if [ ! -d "$SCRIPT_DIR/sdk/python" ]; then
        echo "  ADVERTENCIA: Directorio sdk/python/ no encontrado, saltando SDK Python"
        return
    fi

    cd "$SCRIPT_DIR/sdk/python"

    pip install -e . --quiet 2>/dev/null

    if command -v python3 &> /dev/null; then
        python3 -c "import pardusdb" 2>/dev/null && echo "  SDK Python instalado correctamente" || echo "  ADVERTENCIA: SDK Python no disponible"
    fi

    cd "$SCRIPT_DIR"
}

create_data_dir() {
    echo "[7/7] Creando directorio de datos..."

    mkdir -p "$DATA_DIR"

    echo "  Directorio de datos: $DATA_DIR/"
    echo "  Base de datos por defecto: $DATA_DIR/pardus-rag.db"

    if [ ! -f "$DATA_DIR/pardus-rag.db" ]; then
        echo "  Creando base de datos por defecto..."
        echo ".create $DATA_DIR/pardus-rag.db" | "$INSTALL_DIR/$BINARY_NAME" > /dev/null 2>&1 || true
        if [ -f "$DATA_DIR/pardus-rag.db" ]; then
            echo "  Base de datos creada exitosamente."
        fi
    fi
}

verify_installation() {
    echo ""
    echo "==================================="
    echo "   Instalacion Completada!"
    echo "==================================="
    echo ""
    echo "Archivos instalados:"
    echo "  - $INSTALL_DIR/pardusdb    (binario principal)"
    echo "  - $INSTALL_DIR/pardus      (helper, crea BD por defecto)"
    echo "  - $MCP_DIR/              (servidor MCP)"
    echo "  - $CONFIG_DIR/config.toml (configuracion)"
    echo ""
    echo "Binarios compilados:"
    echo "  - $BIN_OUT_DIR/pardus-v${VERSION}"
    echo ""
    echo "Uso rapido:"
    echo "  pardus                    # Abre la BD por defecto"
    echo "  pardusdb                  # Binario directo (in-memory)"
    echo "  pardusdb mi.db            # Abre archivo especifico"
    echo ""
    echo "Para usar el MCP server con OpenCode, ver INSTALL.md"
    echo ""
}

do_install() {
    check_prerequisites
    build_binary
    install_binary
    create_helper
    create_config
    install_mcp
    install_python_sdk
    create_data_dir
    verify_installation
}

do_uninstall() {
    echo "==================================="
    echo "   PardusDB v${VERSION} - Desinstalacion"
    echo "==================================="
    echo ""

    local removed=0

    if [ -f "$INSTALL_DIR/pardusdb" ]; then
        rm -f "$INSTALL_DIR/pardusdb"
        echo "  Eliminado: $INSTALL_DIR/pardusdb"
        removed=1
    fi

    if [ -f "$INSTALL_DIR/$HELPER_NAME" ]; then
        rm -f "$INSTALL_DIR/$HELPER_NAME"
        echo "  Eliminado: $INSTALL_DIR/$HELPER_NAME"
        removed=1
    fi

    if [ -d "$PARDUS_HOME" ]; then
        rm -rf "$PARDUS_HOME"
        echo "  Eliminado: $PARDUS_HOME/ (bases de datos)"
        removed=1
    fi

    if [ -d "$CONFIG_DIR" ]; then
        rm -rf "$CONFIG_DIR"
        echo "  Eliminado: $CONFIG_DIR/"
        removed=1
    fi

    if [ $removed -eq 0 ]; then
        echo "No se encontro ninguna instalacion de PardusDB."
    else
        echo ""
        echo "Desinstalacion completada."
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
        echo "Opcion desconocida: $1"
        echo "Usa --help para ver las opciones disponibles."
        exit 1
        ;;
esac