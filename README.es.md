# webcam-proxy

`webcam-proxy` es un pequeño proxy de webcam para Linux. Toma una cámara física de entrada, la reemite con `ffmpeg` y escribe el vídeo en un dispositivo virtual `v4l2loopback` para que otras aplicaciones lo vean como si fuera una webcam normal.

El objetivo del proyecto es resolver cámaras con compatibilidades raras en Linux y mantener la menor latencia posible.

## Qué hace

- Espera a que exista la cámara de entrada.
- Si falta el dispositivo virtual de salida, intenta crearlo automáticamente con `v4l2loopback`.
- Lanza `ffmpeg` para leer de la webcam física y escribir en la cámara virtual.
- Si `ffmpeg` falla, reintenta después de un pequeño retraso.
- Usa opciones de baja latencia para reducir retardo y microcortes.

## Requisitos

- Linux.
- `ffmpeg` instalado y accesible en `PATH`.
- `v4l2loopback` disponible en el sistema (módulo instalable/cargable por `modprobe`).
- Permisos para leer la cámara de entrada y escribir en el dispositivo virtual.
- Permisos para ejecutar `modprobe` si quieres creación automática del dispositivo de salida.

## Preparar la cámara virtual

Si el dispositivo de salida no existe, el programa intenta crearlo automáticamente con:

```bash
modprobe v4l2loopback devices=1 video_nr=N card_label="WebcamVirtual" exclusive_caps=1
```

Donde `N` es el número tomado de `--output` (por ejemplo, `/dev/video10` implica `video_nr=10`).

Si prefieres, también puedes cargar el módulo manualmente antes de arrancar:

```bash
sudo modprobe v4l2loopback devices=1 video_nr=10 card_label="WebcamVirtual" exclusive_caps=1
```

Eso crea el dispositivo `/dev/video10`.

## Compilar

```bash
cargo build --release
```

El binario quedará en:

```bash
./target/release/webcam-proxy
```

## Ejecutar

```bash
sudo ./target/release/webcam-proxy
```

Por defecto usa:

- entrada: `/dev/video0`
- salida: `/dev/video10`
- resolución: `1280x720`
- FPS: `12`
- reintento: `3s`

## Opciones disponibles

```bash
sudo ./target/release/webcam-proxy --help
```

Parámetros:

- `--input`: dispositivo de entrada de la webcam física.
- `--output`: dispositivo virtual de salida `v4l2loopback`.
- `--width`: ancho del vídeo de salida.
- `--height`: alto del vídeo de salida.
- `--fps`: frames por segundo.
- `--retry`: espera entre reintentos cuando falta la entrada o falla `ffmpeg`.

Ejemplo con valores personalizados:

```bash
sudo ./target/release/webcam-proxy --input /dev/video2 --output /dev/video10 --width 1920 --height 1080 --fps 30
```

## Comportamiento interno

El programa lanza `ffmpeg` con parámetros orientados a baja latencia:

- `-fflags nobuffer`
- `-flags low_delay`
- `-probesize 32`
- `-analyzeduration 0`
- `-thread_queue_size 512`
- `-err_detect ignore_err`

Además, silencia la salida de `ffmpeg` para evitar spam de errores o advertencias en consola.

Si `ffmpeg` se cae, el programa espera y vuelve a arrancarlo sobre el mismo dispositivo virtual. No crea una webcam nueva en cada reinicio: reutiliza la misma salida (`--output`) durante toda la ejecución.

## Problemas comunes

### `ffmpeg not found in PATH`

Instala `ffmpeg` y asegúrate de que el comando esté disponible en tu terminal.

### `Output device ... not found`

El programa intentará crearlo automáticamente. Si falla, revisa permisos de `modprobe`, disponibilidad de `v4l2loopback` y que `--output` tenga formato `/dev/videoN`.

### La webcam virtual no aparece en las apps

Comprueba que el módulo `v4l2loopback` esté cargado y que la aplicación tenga permisos para leer `/dev/video10`.

### La cámara de entrada tarda en aparecer

El programa espera automáticamente a que el dispositivo de entrada exista antes de arrancar `ffmpeg`.

## Licencia

Consulta [LICENSE](LICENSE) para los detalles de la licencia.

---

**[English](README.md)**
