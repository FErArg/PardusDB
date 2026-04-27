#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="0.2.0"
BINARY_NAME="pardusdb"
HELPER_NAME="pardus"
INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/pardus"
MCP_DIR="$DATA_DIR/mcp"

show_help() {
    cat << EOF
PardusDB v${VERSION} - Instalador

USO:
    ./setup.sh [OPCION]

OPCIONES:
    --install     Instalar PardusDB (por defecto)
    --uninstall   Desinstalar PardusDB completamente
    --help        Mostrar esta ayuda

INSTALACIÓN:
    Instala el binario pardusdb, el helper 'pardus',
    el servidor MCP para agentes AI, y el SDK Python.

    Rutas de instalación:
      - Binario:     ~/.local/bin/pardusdb
      - Helper:      ~/.local/bin/pardus
      - Datos BD:   ~/.local/share/pardus/
      - MCP Server:  ~/.local/share/pardus/mcp/

DESINSTALACIÓN:
    Elimina todos los archivos instalados incluyendo
    las bases de datos almacenadas en ~/.local/share/pardus/

EOF
}

check_prerequisites() {
    echo "==================================="
    echo "   PardusDB v${VERSION} Installer"
    echo "==================================="
    echo ""

    local missing=()

    if ! command -v cargo &> /dev/null; then
        missing+=("Rust (cargo) - instalar desde https://rustup.rs/")
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
        echo "ERROR: Node.js 18+ requerido. Versión actual: $(node -v)"
        exit 1
    fi

    local python_version=$(python3 --version | sed 's/Python //' | cut -d. -f1-2)
    echo "Prerrequisitos verificados."
    echo ""
}

build_binary() {
    echo "[1/7] Construyendo binario Rust (release mode)..."
    cargo build --release 2>/dev/null

    if [ ! -f "target/release/$BINARY_NAME" ]; then
        echo "Error: La compilación del binario falló."
        echo "Verifique que Rust esté correctamente instalado e intente de nuevo."
        exit 1
    fi
    echo "Binario construido correctamente."
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
        echo "AÑADE '$INSTALL_DIR' A TU PATH si no está ya:"
        echo "  echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.bashrc"
        echo "  source ~/.bashrc"
    fi
}

create_helper() {
    echo "[3/7] Creando helper 'pardus'..."

    cat > "$INSTALL_DIR/$HELPER_NAME" << 'HELPER_SCRIPT'
#!/bin/bash
DB_DIR="$HOME/.local/share/pardus"
DEFAULT_DB="$DB_DIR/data.pardus"

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

install_mcp() {
    echo "[4/7] Instalando servidor MCP..."

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
    echo "  Para usar con OpenCode, ver INSTALL.md"
}

install_python_sdk() {
    echo "[5/7] Instalando SDK Python..."

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
    echo "[6/7] Creando directorio de datos..."

    mkdir -p "$DATA_DIR"

    echo "  Directorio de datos: $DATA_DIR/"
    echo "  Base de datos por defecto: $DATA_DIR/data.pardus"
}

install_typescript_sdk() {
    echo "[7/7] Instalando SDK TypeScript..."

    if [ ! -d "$SCRIPT_DIR/sdk/typescript/pardusdb" ]; then
        echo "  ADVERTENCIA: Directorio sdk/typescript/pardusdb/ no encontrado"
        return
    fi

    cd "$SCRIPT_DIR/sdk/typescript/pardusdb"

    if [ -d "node_modules" ]; then
        rm -rf node_modules
    fi

    npm install --silent 2>/dev/null

    if [ ! -f "dist/index.js" ]; then
        npm run build 2>/dev/null
    fi

    if [ -f "dist/index.js" ]; then
        echo "  SDK TypeScript instalado correctamente"
    else
        echo "  ADVERTENCIA: SDK TypeScript no pudo ser construido"
    fi

    cd "$SCRIPT_DIR"
}

verify_installation() {
    echo ""
    echo "==================================="
    echo "   Instalación Completada!"
    echo "==================================="
    echo ""
    echo "Archivos instalados:"
    echo "  - $INSTALL_DIR/pardusdb    (binario principal)"
    echo "  - $INSTALL_DIR/pardus      (helper, crea BD por defecto)"
    echo "  - $MCP_DIR/              (servidor MCP)"
    echo ""
    echo "Uso rápido:"
    echo "  pardus                    # Abre la BD por defecto"
    echo "  pardusdb                  # Binario directo (in-memory)"
    echo "  pardusdb mi.db            # Abre archivo específico"
    echo ""
    echo "Para usar el MCP server con OpenCode, ver INSTALL.md"
    echo ""
}

do_install() {
    check_prerequisites
    build_binary
    install_binary
    create_helper
    install_mcp
    install_python_sdk
    create_data_dir
    install_typescript_sdk
    verify_installation
}

do_uninstall() {
    echo "==================================="
    echo "   PardusDB v${VERSION} - Desinstalación"
    echo "==================================="
    echo ""

    local removed=0

    if [ -f "$INSTALL_DIR/pardusdb" ]; then
        rm -f "$INSTALL_DIR/pardusdb"
        echo "  Eliminado: $INSTALL_DIR/pardusdb"
        removed=1
    fi

    if [ -f "$INSTALL_DIR/pardus" ]; then
        rm -f "$INSTALL_DIR/pardus"
        echo "  Eliminado: $INSTALL_DIR/pardus"
        removed=1
    fi

    if [ -d "$DATA_DIR" ]; then
        rm -rf "$DATA_DIR"
        echo "  Eliminado: $DATA_DIR/ (bases de datos)"
        removed=1
    fi

    if [ $removed -eq 0 ]; then
        echo "No se encontró ninguna instalación de PardusDB."
    else
        echo ""
        echo "Desinstalación completada."
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
        echo "Opción desconocida: $1"
        echo "Usa --help para ver las opciones disponibles."
        exit 1
        ;;
esac