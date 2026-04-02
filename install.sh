#!/bin/bash

# Detener el script si ocurre algún error
set -e

echo "📷 Detectando cámaras disponibles..."
echo "-----------------------------------"
if command -v v4l2-ctl &> /dev/null; then
    v4l2-ctl --list-devices
else
    ls -l /dev/video*
fi
echo "-----------------------------------"

read -p "Introduce la ruta de la cámara de entrada que deseas usar (ej. /dev/video1) [Por defecto: /dev/video0]: " INPUT_CAM
INPUT_CAM=${INPUT_CAM:-/dev/video0}

echo ""
echo "✅ Usando $INPUT_CAM como cámara de entrada."
echo ""

echo "⚙️  Construyendo webcam-proxy..."
cargo build --release

echo "📦 Instalando binario en /usr/local/bin/webcam-proxy..."
sudo install -m 755 target/release/webcam-proxy /usr/local/bin/webcam-proxy

echo "📝 Creando archivo de servicio systemd en /etc/systemd/system/webcam-proxy.service..."
sudo tee /etc/systemd/system/webcam-proxy.service > /dev/null <<EOF
[Unit]
Description=Webcam Proxy (v4l2loopback)
After=network.target
Wants=network.target

[Service]
Type=simple
ExecStartPre=-/usr/sbin/modprobe v4l2loopback devices=1 video_nr=10 card_label=WebcamVirtual exclusive_caps=1
ExecStart=/usr/local/bin/webcam-proxy --input ${INPUT_CAM} --output /dev/video10 --width 1280 --height 720 --fps 30 --retry 3
Restart=always
RestartSec=3
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

echo "🔄 Recargando systemd y habilitando el servicio..."
sudo systemctl daemon-reload
sudo systemctl enable --now webcam-proxy.service

echo "🚀 Reiniciando servicio para aplicar posibles cambios del binario..."
sudo systemctl restart webcam-proxy.service

echo "✅ ¡Instalación completada con éxito!"
echo ""
echo "📊 Estado del servicio:"
sudo systemctl status webcam-proxy.service --no-pager || true

echo ""
echo "📝 Para ver los registros en tiempo real, ejecuta:"
echo "journalctl -u webcam-proxy.service -f"
