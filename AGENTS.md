# AGENTS.md

## Proyecto
Proxy de cámara: captura una cámara física y la retransmite como cámara virtual estándar (UVC). El problema de cámaras antiguas (ej: HD Trust Trino 720p) que envían raw sin comprimir, causando lag en apps como Discord.

## Comandos
- `cargo run` - Compila y ejecuta el proyecto
- `cargo build` - Solo compila
- `cargo clippy` - Linter
- `cargo check` - Verificación rápida de tipos

## Requisitos
- Linux (usa `/dev/video*` y udev)
- libudev-dev instalado en el sistema (`sudo apt install libudev-dev`)

## Workflow
1. Antes de instalar una nueva dependencia, buscar en Context7 y en internet
2. Usar las skills disponibles (ejecutar `skill` tool cuando la tarea lo requiera)
3. Comentar todo el código en español con comentarios simples
4. Preferir soluciones simples: menos código es mejor si hace lo mismo

## Estructura
- `src/main.rs` - Punto de entrada
- `src/camera.rs` - Lógica de detección de cámaras
