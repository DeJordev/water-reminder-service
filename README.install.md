# Instalación de Water Reminder

Este paquete contiene un binario ya compilado de Water Reminder para Linux x86_64.
No necesitas Rust, Cargo ni conservar el repositorio clonado.

## Requisitos runtime

- Linux con entorno de escritorio (X11 o Wayland)
- systemd de usuario disponible en la sesión gráfica
- GTK3 y GLib runtime
- AppIndicator o Ayatana AppIndicator runtime para el icono de bandeja
- `paplay` para sonido, normalmente en `pulseaudio-utils` o equivalente

En Debian/Ubuntu:

```bash
sudo apt install libgtk-3-0 libayatana-appindicator3-1 pulseaudio-utils
```

## Instalar

Desde este directorio:

```bash
./install.sh
```

El instalador copia:

- `bin/water-reminder` a `~/.local/bin/water-reminder`
- `water-reminder.service` a `~/.config/systemd/user/water-reminder.service`

Después recarga systemd de usuario y activa el servicio.

## Comandos útiles

```bash
systemctl --user status water-reminder
journalctl --user -u water-reminder -f
systemctl --user restart water-reminder
```

## Desinstalar

Desinstalación normal, preguntando si conservar configuración y estadísticas:

```bash
./uninstall.sh
```

Conservar datos sin preguntar:

```bash
./uninstall.sh --keep-data
```

Borrar configuración y estadísticas sin preguntar:

```bash
./uninstall.sh --purge
```

Datos de usuario:

- Configuración: `~/.config/water-reminder`
- Estadísticas: `~/.local/share/water-reminder`
