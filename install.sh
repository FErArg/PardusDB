#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="0.4.4"
BINARY_NAME="pardusdb"
HELPER_NAME="pardus"
BINARY_SOURCE="$SCRIPT_DIR/bin/pardus-v${VERSION}"
INSTALL_DIR="$HOME/.local/bin"
PARDUS_HOME="$HOME/.pardus"
CONFIG_DIR="$HOME/.config/pardus"
DATA_DIR="$PARDUS_HOME"
MCP_DIR="$PARDUS_HOME/mcp"

show_help() {
    cat << EOF
PardusDB v${VERSION} - Instalador (binario precompilado)

USO:
    ./install.sh [OPCION]

OPCIONES:
    --install     Instalar PardusDB (por defecto)
    --uninstall   Desinstalar PardusDB completamente
    --help        Mostrar esta ayuda

INSTALACION:
    Instala PardusDB usando el binario precompilado en bin/pardus-v${VERSION}.
    No requiere Rust ni compilacion.

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

detect_shell() {
    case "$(basename "$SHELL")" in
        bash) SHELL_RC="$HOME/.bashrc" ;;
        zsh)  SHELL_RC="$HOME/.zshrc" ;;
        fish) SHELL_RC="$HOME/.config/fish/config.fish" ;;
        *)    SHELL_RC="" ;;
    esac
}

check_prerequisites() {
    echo "==================================="
    echo "   PardusDB v${VERSION} Installer"
    echo "==================================="
    echo ""

    local missing=()

    if ! command -v python3 &> /dev/null; then
        missing+=("Python 3 (python3) - instalar desde https://python.org/")
    fi

    if ! command -v pip3 &> /dev/null; then
        missing+=("pip3 - se instala con Python")
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

    echo "Prerrequisitos verificados."
    echo ""
}

install_binary() {
    echo "[1/6] Instalando binario..."

    if [ ! -f "$BINARY_SOURCE" ]; then
        echo "ERROR: Binario precompilado no encontrado: $BINARY_SOURCE"
        echo ""
        echo "Este script requiere que el binario este precompilado."
        echo "Ejecuta './setup.sh --install' primero para compilar desde fuente."
        exit 1
    fi

    mkdir -p "$INSTALL_DIR"

    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        rm -f "$INSTALL_DIR/$BINARY_NAME"
    fi

    cp "$BINARY_SOURCE" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    if [[ ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
        echo "Binario instalado en: $INSTALL_DIR/$BINARY_NAME (ya en PATH)"
    else
        echo "Binario instalado en: $INSTALL_DIR/$BINARY_NAME"
        echo "  ANADE '$INSTALL_DIR' A TU PATH si no esta ya:"
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
    echo "[2/6] Creando helper 'pardus'..."

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
    echo "[3/6] Creando archivo de configuracion..."

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
    echo "[4/7] Instalando servidor MCP (Python)..."

    if [ ! -f "$SCRIPT_DIR/mcp/src/server.py" ]; then
        echo "  ADVERTENCIA: mcp/src/server.py no encontrado, saltando MCP server"
        return
    fi

    mkdir -p "$MCP_DIR"

    cp "$SCRIPT_DIR/mcp/src/server.py" "$MCP_DIR/"

    echo "  Instalando paquete MCP de Python..."
    pip3 install mcp --quiet 2>/dev/null || pip3 install mcp --quiet --break-system-packages 2>/dev/null || echo "  ADVERTENCIA: No se pudo instalar el paquete mcp"

    echo "  MCP server instalado en: $MCP_DIR/server.py"
}

configure_opencode() {
    echo "[5/7] Configurando OpenCode..."

    if [ ! -f "$MCP_DIR/server.py" ]; then
        echo "  MCP server no instalado, saltando configuracion OpenCode"
        return
    fi

    local REAL_USER
    REAL_USER=$(logname 2>/dev/null || echo "$USER")

    echo -n "  Configurar PardusDB MCP para OpenCode? (s/N): "
    read -r respuesta
    if [ "$respuesta" != "s" ] && [ "$respuesta" != "S" ]; then
        echo "  Omitido."
        return
    fi

    local OPCODE_CONFIG_DIR="$HOME/.config/opencode"
    local OPCODE_CONFIG="$OPCODE_CONFIG_DIR/opencode.json"
    local OPCODE_SKILLS_DIR="$HOME/.config/opencode/skills"
    local SKILL_SOURCE="$SCRIPT_DIR/skill/skill.md"

    if [ -f "$SKILL_SOURCE" ]; then
        mkdir -p "$OPCODE_SKILLS_DIR"
        cp "$SKILL_SOURCE" "$OPCODE_SKILLS_DIR/pardusdb.md"
        echo "  Skill copiado: $OPCODE_SKILLS_DIR/pardusdb.md"
    fi

    local MCP_PATH="/home/$REAL_USER/.pardus/mcp/server.py"

    if [ -f "$OPCODE_CONFIG" ]; then
        if python3 -c "
import json
with open('$OPCODE_CONFIG') as f:
    cfg = json.load(f)
exit(0 if 'pardusdb' in cfg.get('mcp', {}) else 1)
" 2>/dev/null; then
            echo "  Entrada 'pardusdb' ya existe en $OPCODE_CONFIG"
            echo "  Omitiendo."
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
    'command': ['python3', '$MCP_PATH'],
    'enabled': True
}
with open('$OPCODE_CONFIG', 'w') as f:
    json.dump(cfg, f, indent=2)
    f.write('\n')
" 2>/dev/null && echo "  MCP configurado en: $OPCODE_CONFIG" || echo "  ERROR: No se pudo actualizar $OPCODE_CONFIG"
    else
        mkdir -p "$OPCODE_CONFIG_DIR"
        cat > "$OPCODE_CONFIG" << JSONEOF
{
  "\$schema": "https://opencode.ai/config.json",
  "mcp": {
    "pardusdb": {
      "type": "local",
      "command": ["python3", "$MCP_PATH"],
      "enabled": true
    }
  }
}
JSONEOF
        echo "  Creado: $OPCODE_CONFIG"
    fi

    echo "  Recuerda reiniciar OpenCode para que los cambios surtan efecto."
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
    install_binary
    create_helper
    create_config
    install_mcp
    configure_opencode
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