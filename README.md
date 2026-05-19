# Water Reminder

Recordatorio de hidratación para Linux con interfaz gráfica invasiva y cero paciencia, escrito en Rust con GTK/AppIndicator.

Cuando llega la hora, ocupa toda la pantalla y no se va hasta que confirmas que has bebido. Si prefieres menos drama, puedes ponerla en modo notificación pequeña, pero entonces la app te juzga igual.

## Características

- Pantalla completa que se impone sobre cualquier ventana activa
- Mensajes aleatorios en tono de bronca cariñosa, por si tu fuerza de voluntad está de adorno
- Fade-in al aparecer
- Contador de vasos del día con barra visual (💧○○○○○○○)
- Sonido al mostrar el recordatorio
- Icono en la bandeja del sistema con countdown al próximo recordatorio
- Ajustes gráficos desde el icono de la bandeja
- Pausa rápida durante 1 hora o hasta el día siguiente
- Estadísticas diarias guardadas en `~/.local/share/water-reminder/stats.json`
- Configuración persistente en `~/.config/water-reminder/config.json`
- Servicio systemd de usuario (arranca con la sesión gráfica, se reinicia automáticamente)
- Variables de entorno compatibles para sobrescribir intervalo y objetivo diario

## Requisitos runtime

- Linux con entorno de escritorio (X11 o Wayland)
- systemd de usuario disponible en la sesión gráfica
- GTK3 y GLib runtime
- AppIndicator/Ayatana AppIndicator runtime para la bandeja
- `paplay` para el sonido (paquete `pulseaudio-utils`). Opcional: sin él la app sigue funcionando sin sonido

```bash
sudo apt install libgtk-3-0 libayatana-appindicator3-1 pulseaudio-utils
sudo pacman -S gtk3 libayatana-appindicator libpulse
sudo dnf install gtk3 libappindicator-gtk3 pulseaudio-utils
```

## Instalación para usuario final

Descarga el tarball de release `water-reminder-linux-x86_64.tar.gz`, descomprímelo y ejecuta el instalador incluido:

```bash
tar -xzf water-reminder-linux-x86_64.tar.gz
cd water-reminder-linux-x86_64
./install.sh
```

No necesitas Rust, Cargo ni conservar el repositorio clonado si instalas desde el tarball.

El instalador:

1. Copia el binario a `~/.local/bin/water-reminder`
2. Instala el servicio systemd de usuario en `~/.config/systemd/user/water-reminder.service`
3. Ejecuta `systemctl --user daemon-reload`
4. Ejecuta `systemctl --user enable --now water-reminder`

El servicio arranca automáticamente con cada inicio de sesión gráfica.

## Desinstalación

Desde el directorio descomprimido del release:

```bash
./uninstall.sh
```

El desinstalador para y deshabilita el servicio, borra el binario instalado y pregunta si quieres conservar datos:

- Configuración: `~/.config/water-reminder`
- Estadísticas: `~/.local/share/water-reminder`

Conservar datos sin preguntar:

```bash
./uninstall.sh --keep-data
```

Borrar configuración y estadísticas sin preguntar:

```bash
./uninstall.sh --purge
```

## Desarrollo

Para compilar desde el código fuente necesitas Rust estable con Cargo y dependencias de compilación GTK/AppIndicator.

```bash
git clone https://github.com/dejordev/water-reminder.git
cd water-reminder
./scripts/install-dev.sh
```

Dependencias de compilación orientativas:

```bash
sudo apt install cargo libgtk-3-dev libayatana-appindicator3-dev pulseaudio-utils
sudo pacman -S rust gtk3 libayatana-appindicator libpulse
sudo dnf install cargo gtk3-devel libappindicator-gtk3-devel pulseaudio-utils
```

Núcleo Rust testeable sin dependencias gráficas:

```bash
cargo test
```

Compilar y ejecutar la app Rust completa:

```bash
cargo run --release --features gui
```

Comprobar solo compilación GUI:

```bash
cargo check --features gui
```

Generar un paquete de release:

```bash
./scripts/build-release.sh
```

El paquete se crea en `dist/water-reminder-linux-x86_64.tar.gz`.

## Configuración

La forma normal de configurar la app es desde el icono de la bandeja:

- Click izquierdo: abre los ajustes
- Click derecho: menú de pausa, reset, ajustes y salida

Los ajustes se guardan en:

```bash
~/.config/water-reminder/config.json
```

También puedes sobrescribir el intervalo o el objetivo diario desde systemd,
útil para instalaciones antiguas o despliegues simples:

```bash
systemctl --user edit water-reminder
```

```ini
[Service]
Environment=WATER_INTERVAL=30
Environment=WATER_DAILY_GOAL=10
```

`WATER_INTERVAL` son los minutos entre recordatorios. `WATER_DAILY_GOAL` es el objetivo diario de vasos.

Después recarga:

```bash
systemctl --user daemon-reload && systemctl --user restart water-reminder
```

## Comandos útiles

```bash
systemctl --user status water-reminder
journalctl --user -u water-reminder -f
systemctl --user restart water-reminder
systemctl --user stop water-reminder
systemctl --user start water-reminder
```

## Estadísticas

Los registros se guardan en `~/.local/share/water-reminder/stats.json`:

```json
{
  "2026-03-25": 6,
  "2026-03-26": 8
}
```

Si el archivo se corrompe o contiene datos inesperados, la app empieza el día
desde 0 y deja una advertencia en logs en lugar de fallar al arrancar.

## Troubleshooting

```bash
journalctl --user -u water-reminder -f
systemctl --user restart water-reminder
systemctl --user status water-reminder
```

Si no aparece el icono, probablemente tu entorno está haciendo cosas raras con el system tray. Mira el estado y los logs antes de insultar al programa, aunque se lo haya ganado un poco.

## Licencia

MIT
