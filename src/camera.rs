use std::fs;

use v4l2r::ioctl::{querycap, Capabilities};

pub enum CameraType {
    Physical,
    Virtual,
}

pub struct Camera {
    pub name: String,
    pub path: String,
    pub camera_type: CameraType,
}

impl Camera {
    pub fn new(name: String, path: String, camera_type: CameraType) -> Self {
        Self {
            name,
            path,
            camera_type,
        }
    }
}

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

    fs::write(add_path, "62")
        .or_else(|_| fs::write(add_path, "62\n"))
        .map_err(|e| format!("Error escribiendo {}: {}", add_path.display(), e))?;

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

pub fn get_all_cameras() -> Vec<Camera> {
    let mut cameras: Vec<Camera> = Vec::new();

    let video_devices = match fs::read_dir("/dev") {
        Ok(entries) => entries,
        Err(_) => return cameras,
    };

    for entry in video_devices.flatten() {
        let path = entry.path();

        let _name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.starts_with("video") => n.to_string(),
            _ => continue,
        };

        let fd = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
        {
            Ok(f) => f,
            Err(_) => continue,
        };

        let caps: v4l2r::ioctl::Capability = match querycap(&fd) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let device_caps = caps.device_caps();
        let is_capture = device_caps.contains(Capabilities::VIDEO_CAPTURE)
            || device_caps.contains(Capabilities::VIDEO_CAPTURE_MPLANE);
        let is_output = device_caps.contains(Capabilities::VIDEO_OUTPUT)
            || device_caps.contains(Capabilities::VIDEO_OUTPUT_MPLANE);

        if !is_capture && !is_output {
            continue;
        }

        let is_virtual = caps.driver == "v4l2 loopback" || caps.bus_info.is_empty();

        let camera_type = if is_virtual {
            CameraType::Virtual
        } else {
            CameraType::Physical
        };

        cameras.push(Camera::new(
            caps.card.replace('_', " "),
            path.to_string_lossy().to_string(),
            camera_type,
        ));
    }

    cameras
}
