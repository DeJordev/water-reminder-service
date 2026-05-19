#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
PACKAGE_NAME="water-reminder-linux-x86_64"
DIST_DIR="$PROJECT_DIR/dist"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"
BIN_SRC="$PROJECT_DIR/target/release/water-reminder"
TARBALL="$DIST_DIR/$PACKAGE_NAME.tar.gz"

die() {
    echo "Error: $*" >&2
    exit 1
}

echo "==> Construyendo Water Reminder release"

command -v cargo >/dev/null 2>&1 || die "cargo no está instalado."
command -v tar >/dev/null 2>&1 || die "tar no está instalado."

cd "$PROJECT_DIR"

echo "==> Compilando binario Rust con GUI"
cargo build --release --features gui

[[ -x "$BIN_SRC" ]] || die "el binario compilado no es ejecutable: $BIN_SRC"

echo "==> Preparando paquete en $PACKAGE_DIR"
rm -rf -- "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR/bin"

install -Dm755 "$BIN_SRC" "$PACKAGE_DIR/bin/water-reminder"
install -Dm755 "$PROJECT_DIR/install.sh" "$PACKAGE_DIR/install.sh"
install -Dm755 "$PROJECT_DIR/uninstall.sh" "$PACKAGE_DIR/uninstall.sh"
install -Dm644 "$PROJECT_DIR/water-reminder.service" "$PACKAGE_DIR/water-reminder.service"
install -Dm644 "$PROJECT_DIR/README.install.md" "$PACKAGE_DIR/README.install.md"

echo "==> Generando tarball $TARBALL"
rm -f -- "$TARBALL"
tar -C "$DIST_DIR" -czf "$TARBALL" "$PACKAGE_NAME"

echo ""
echo "Release generado:"
echo "  $PACKAGE_DIR"
echo "  $TARBALL"
