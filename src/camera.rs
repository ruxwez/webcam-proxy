// Importaciones necesarias para el sistema de archivos,
// hasheo de datos y metadata de dispositivos Linux.
use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    os::linux::fs::MetadataExt,
};
use udev::{Device as UdevDevice, DeviceType};

// Capacidades de la cámara usando bitflags.
// VIDEO_CAPTURE = captura de video, META_CAPTURE = metadatos.
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CameraCapabilities: u32 {
        const VIDEO_CAPTURE = 0x00000001;
        const META_CAPTURE  = 0x01000000;
    }
}

// Tipo de cámara: física (USB) o virtual (v4l2loopback).
pub enum CameraType {
    Physical,
    Virtual,
}

// Representa una cámara de video encontrada en /dev/video*.
pub struct Camera {
    pub uuid: String,                     // Identificador único
    pub name: String,                     // Nombre legible del dispositivo
    pub path: String,                     // Ruta en /dev/videoN
    pub driver: String,                   // Driver del kernel
    pub bus_info: String,                 // Información del bus
    pub capabilities: CameraCapabilities, // Capacidades
    pub camera_type: CameraType,          // Física o virtual
}

impl Camera {
    // Constructor público de Camera.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uuid: String,
        name: String,
        path: String,
        driver: String,
        bus_info: String,
        camera_type: CameraType,
        capabilities: CameraCapabilities,
    ) -> Self {
        Self {
            uuid,
            name,
            path,
            driver,
            bus_info,
            capabilities,
            camera_type,
        }
    }
}

// Crea una cámara virtual fija usando v4l2loopback.
// Usa video_nr=62 para que el dispositivo siempre sea /dev/video62,
// dando un UUID estable entre reinicios.
//
// Estrategia (por orden de preferencia):
// 1. Si /dev/video62 ya existe → ok.
// 2. Si v4l2loopback está cargado, usa /sys/.../add para crear el
//    dispositivo (no necesita recargar el módulo).
// 3. Si el módulo no está cargado, intenta modprobe (puede requerir
//    sudo; si falla, muestra el comando manual).
//
// Si v4l2loopback ya está cargado por otro programa (ej: OBS),
// se puede configurar el módulo desde el arranque para evitar
// conflictos:
//   echo "options v4l2loopback video_nr=62 card_label=webcam_proxy exclusive_caps=1" | sudo tee /etc/modprobe.d/webcam-proxy.conf
pub fn init_virtual_camera() -> Result<(), String> {
    let dev_path = std::path::Path::new("/dev/video62");
    if dev_path.exists() {
        return Ok(());
    }

    let mod_path = std::path::Path::new("/sys/module/v4l2loopback");

    if mod_path.exists() {
        return try_sysfs_add();
    }

    try_modprobe()?;

    // Esperar a que udev cree el device node
    for _ in 0..10 {
        if dev_path.exists() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Err(
        "modprobe se ejecutó pero /dev/video62 no apareció. ".to_string()
            + "Verifica que el módulo v4l2loopback esté instalado:\n  sudo modprobe v4l2loopback video_nr=62 card_label=webcam_proxy exclusive_caps=1",
    )
}

// Crea el device /dev/video62 si v4l2loopback ya está cargado
fn try_sysfs_add() -> Result<(), String> {
    let add_path = std::path::Path::new("/sys/devices/virtual/video4linux/v4l2loopback/add");

    if !add_path.exists() {
        return Err(
            "v4l2loopback cargado pero no expone sysfs add. ".to_string()
                + "Recarga el módulo:\n  sudo modprobe -r v4l2loopback"
                + " && sudo modprobe v4l2loopback video_nr=62"
                + " card_label=webcam_proxy exclusive_caps=1",
        );
    }

    // Algunas versiones necesitan el número solo, otras con \n
    fs::write(add_path, "62")
        .or_else(|_| fs::write(add_path, "62\n"))
        .map_err(|e| format!("Error escribiendo {}: {}", add_path.display(), e))?;

    // Esperar a que udev cree el device node
    let dev_path = std::path::Path::new("/dev/video62");
    for _ in 0..10 {
        if dev_path.exists() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Err("sysfs add ok pero /dev/video62 no apareció. ".to_string()
        + "Prueba recargar el módulo:\n  sudo modprobe -r v4l2loopback"
        + " && sudo modprobe v4l2loopback video_nr=62"
        + " card_label=webcam_proxy exclusive_caps=1")
}

// Carga el módulo v4l2loopback con sudo si hace falta
fn try_modprobe() -> Result<(), String> {
    let modprobe_candidates = ["modprobe", "/sbin/modprobe", "/usr/sbin/modprobe"];
    let mut last_err = String::new();
    let mut permission_denied = false;

    for cmd in &modprobe_candidates {
        let output = std::process::Command::new(cmd)
            .args([
                "v4l2loopback",
                "video_nr=62",
                "card_label=webcam_proxy",
                "exclusive_caps=1",
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => return Ok(()),
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("Operation not permitted") {
                    permission_denied = true;
                }
                last_err = stderr.to_string();
            }
            Err(e) => {
                last_err = format!("{}: {}", cmd, e);
            }
        }
    }

    if permission_denied {
        eprintln!("se necesita sudo para cargar el módulo v4l2loopback.");
        for cmd in &modprobe_candidates {
            let status = std::process::Command::new("sudo")
                .arg(cmd)
                .args([
                    "v4l2loopback",
                    "video_nr=62",
                    "card_label=webcam_proxy",
                    "exclusive_caps=1",
                ])
                .status()
                .map_err(|e| format!("Error ejecutando sudo: {}", e))?;

            if status.success() {
                return Ok(());
            }
        }
        return Err("sudo no pudo cargar v4l2loopback.".to_string());
    }

    Err(format!(
        "No se pudo cargar v4l2loopback ({}). Prueba manualmente:\n  sudo modprobe v4l2loopback video_nr=62 card_label=webcam_proxy exclusive_caps=1",
        last_err.trim()
    ))
}

// Escanea /dev/video* y devuelve todas las cámaras detectadas.
// Para cada entrada en /dev que empiece con "video", obtiene metadata
// del dispositivo udev para extraer UUID, nombre, driver y bus.
pub fn get_all_cameras() -> Vec<Camera> {
    let mut cameras: Vec<Camera> = Vec::new();

    // Leer contenido de /dev
    let video_devices = match fs::read_dir("/dev") {
        Ok(entries) => entries,
        Err(_) => return cameras,
    };

    // Iterar sobre cada entrada en /dev
    for entry in video_devices.flatten() {
        let path = entry.path();

        // Extraer el nombre del archivo (ej: "video0")
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Solo nos interesan dispositivos /dev/videoN
        if !name.starts_with("video") {
            continue;
        }

        // Obtener metadata del archivo para el device number (st_rdev)
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Obtener el dispositivo udev a partir del device number
        let Some(device) = UdevDevice::from_devnum(DeviceType::Character, meta.st_rdev()).ok()
        else {
            continue;
        };

        // Detectar si el dispositivo está conectado por USB
        let is_usb = device.parent_with_subsystem("usb").ok().flatten().is_some();

        // Generar UUID:
        // - USB: usamos ID_SERIAL (estable entre reinicios, viene del hardware).
        // - Virtual: usamos ID_PATH (ruta topológica en sysfs, ej:
        //   "pci-0000:00:14.0-usb-0:1:1.0-video-index0"), más estable
        //   que /dev/videoN. Como respaldo, hasheamos el nombre del
        //   archivo + device number.
        let uuid = if is_usb {
            device
                .property_value("ID_SERIAL")
                .or_else(|| device.property_value("ID_SERIAL_SHORT"))
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            device
                .property_value("ID_PATH")
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    let mut hasher = DefaultHasher::new();
                    name.hash(&mut hasher);
                    meta.st_rdev().hash(&mut hasher);
                    format!("{:016x}", hasher.finish())
                })
        };

        // Obtener nombre legible del dispositivo udev
        let name = device
            .property_value("ID_MODEL")
            .or_else(|| device.property_value("ID_V4L_PRODUCT"))
            .or_else(|| device.property_value("ID_V4L_DRIVER"))
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(name)
            .replace("_", " ");

        // Obtener nombre del driver
        let driver = device
            .property_value("ID_V4L_DRIVER")
            .or_else(|| device.property_value("ID_V4L2_DRIVER"))
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Obtener información del bus
        let bus_info = device
            .property_value("ID_BUS")
            .or_else(|| device.property_value("ID_USB_INTERFACES"))
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Agregar la cámara a la lista
        cameras.push(Camera::new(
            uuid,
            name.trim().to_string(),
            path.to_string_lossy().to_string(),
            driver,
            bus_info,
            if is_usb {
                CameraType::Physical
            } else {
                CameraType::Virtual
            },
            CameraCapabilities::VIDEO_CAPTURE,
        ));
    }

    cameras
}
