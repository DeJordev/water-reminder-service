#!/usr/bin/env bash
set -euo pipefail

SERVICE_NAME="water-reminder"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
SOURCE_BIN="$SCRIPT_DIR/bin/water-reminder"
SOURCE_SERVICE="$SCRIPT_DIR/water-reminder.service"
INSTALL_BIN="$HOME/.local/bin/water-reminder"
SERVICE_FILE="$HOME/.config/systemd/user/$SERVICE_NAME.service"

die() {
    echo "Error: $*" >&2
    exit 1
}

check_systemd_user() {
    command -v systemctl >/dev/null 2>&1 || die "systemctl no está instalado o no está en PATH."

    if ! systemctl --user show-environment >/dev/null 2>&1; then
        cat >&2 <<'EOF'
Error: systemd de usuario no está disponible para esta sesión.

Comprueba que has iniciado sesión con systemd --user activo. En algunas
sesiones mínimas puede ser necesario cerrar sesión y volver a entrar.
EOF
        exit 1
    fi
}

check_release_files() {
    [[ -f "$SOURCE_BIN" ]] || die "no se encontró el binario de release en: $SOURCE_BIN"
    [[ -x "$SOURCE_BIN" ]] || die "el binario existe pero no es ejecutable: $SOURCE_BIN"
    [[ -f "$SOURCE_SERVICE" ]] || die "no se encontró la unidad systemd: $SOURCE_SERVICE"

    if ! grep -qxF "ExecStart=%h/.local/bin/water-reminder" "$SOURCE_SERVICE"; then
        die "la unidad systemd no apunta a %h/.local/bin/water-reminder"
    fi
}

echo "==> Water Reminder - instalación de usuario"
echo ""

check_release_files
check_systemd_user

echo "==> Instalando binario en $INSTALL_BIN"
install -Dm755 "$SOURCE_BIN" "$INSTALL_BIN"

if [[ ! -x "$INSTALL_BIN" ]]; then
    die "el binario instalado no es ejecutable: $INSTALL_BIN"
fi

echo "==> Instalando servicio systemd de usuario en $SERVICE_FILE"
install -Dm644 "$SOURCE_SERVICE" "$SERVICE_FILE"

echo "==> Activando servicio systemd de usuario"
systemctl --user daemon-reload
systemctl --user enable --now "$SERVICE_NAME"

echo ""
echo "Water Reminder instalado y activo."
echo ""
echo "Comandos útiles:"
echo "  Estado:       systemctl --user status $SERVICE_NAME"
echo "  Logs:         journalctl --user -u $SERVICE_NAME -f"
echo "  Reiniciar:    systemctl --user restart $SERVICE_NAME"
echo "  Desinstalar:  ./uninstall.sh"
