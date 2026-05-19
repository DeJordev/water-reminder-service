#!/usr/bin/env bash
set -euo pipefail

SERVICE_NAME="water-reminder"
SERVICE_FILE="$HOME/.config/systemd/user/$SERVICE_NAME.service"
INSTALL_BIN="$HOME/.local/bin/water-reminder"
CONFIG_DIR="$HOME/.config/water-reminder"
DATA_DIR="$HOME/.local/share/water-reminder"
DATA_MODE="ask"

usage() {
    cat <<EOF
Uso: ./uninstall.sh [--purge|--keep-data]

Opciones:
  --purge       Borra configuración y estadísticas sin preguntar.
  --keep-data   Conserva configuración y estadísticas sin preguntar.
  -h, --help    Muestra esta ayuda.
EOF
}

die() {
    echo "Error: $*" >&2
    exit 1
}

safe_remove_dir() {
    local dir="$1"
    local base

    [[ -n "$dir" ]] || die "ruta vacía al borrar datos"
    base="$(basename -- "$dir")"
    [[ "$base" == "water-reminder" ]] || die "ruta inesperada al borrar datos: $dir"

    if [[ -d "$dir" ]]; then
        rm -rf -- "$dir"
    fi
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --purge)
                [[ "$DATA_MODE" == "ask" ]] || die "usa solo una opción de datos"
                DATA_MODE="purge"
                shift
                ;;
            --keep-data)
                [[ "$DATA_MODE" == "ask" ]] || die "usa solo una opción de datos"
                DATA_MODE="keep"
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                die "opción desconocida: $1"
                ;;
        esac
    done
}

should_purge_data() {
    case "$DATA_MODE" in
        purge)
            return 0
            ;;
        keep)
            return 1
            ;;
    esac

    if [[ ! -t 0 ]]; then
        echo "Entrada no interactiva detectada; se conservan los datos."
        return 1
    fi

    local answer
    while true; do
        read -r -p "¿Borrar configuración y estadísticas? [y/N] " answer
        case "${answer,,}" in
            y|yes|s|si|sí)
                return 0
                ;;
            ""|n|no)
                return 1
                ;;
            *)
                echo "Responde y o n."
                ;;
        esac
    done
}

parse_args "$@"

echo "==> Desinstalando Water Reminder..."

if command -v systemctl >/dev/null 2>&1; then
    systemctl --user stop "$SERVICE_NAME" 2>/dev/null || true
    systemctl --user disable "$SERVICE_NAME" 2>/dev/null || true
else
    echo "Aviso: systemctl no está disponible; se omiten stop/disable."
fi

rm -f -- "$SERVICE_FILE"
rm -f -- "$INSTALL_BIN"

if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload 2>/dev/null || true
fi

if should_purge_data; then
    safe_remove_dir "$CONFIG_DIR"
    safe_remove_dir "$DATA_DIR"
    echo "Datos de usuario borrados."
else
    echo "Datos de usuario conservados:"
    echo "  $CONFIG_DIR"
    echo "  $DATA_DIR"
fi

echo ""
echo "Water Reminder desinstalado."
