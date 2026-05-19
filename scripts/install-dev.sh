#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
SERVICE_DIR="$HOME/.config/systemd/user"
SERVICE_NAME="water-reminder"
RUST_BIN="$PROJECT_DIR/target/release/water-reminder"
SERVICE_FILE="$SERVICE_DIR/$SERVICE_NAME.service"

die() {
    echo "Error: $*" >&2
    exit 1
}

echo "==> Water Reminder - instalación de desarrollo"
echo ""

command -v cargo >/dev/null 2>&1 || die "cargo no está instalado. Instala Rust o usa un tarball de release."
command -v systemctl >/dev/null 2>&1 || die "systemctl no está instalado o no está en PATH."

cd "$PROJECT_DIR"

echo "==> Compilando binario Rust..."
cargo build --release --features gui

[[ -x "$RUST_BIN" ]] || die "el binario compilado no es ejecutable: $RUST_BIN"
echo "==> Binario Rust validado: $RUST_BIN"

echo "==> Instalando servicio systemd de usuario..."
mkdir -p "$SERVICE_DIR"

cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Water Reminder - recordatorio periodico de hidratacion
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$RUST_BIN
Restart=on-failure
RestartSec=10

[Install]
WantedBy=graphical-session.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now "$SERVICE_NAME"

echo ""
echo "Water Reminder instalado en modo desarrollo y activo."
echo ""
echo "Comandos útiles:"
echo "  Estado:       systemctl --user status $SERVICE_NAME"
echo "  Logs:         journalctl --user -u $SERVICE_NAME -f"
echo "  Reiniciar:    systemctl --user restart $SERVICE_NAME"
echo "  Desinstalar:  ./uninstall.sh --keep-data"
