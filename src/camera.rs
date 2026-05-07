use std::{fs, os::linux::fs::MetadataExt};
use udev::{Device as UdevDevice, DeviceType};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CameraCapabilities: u32 {
        const VIDEO_CAPTURE = 0x00000001;
        const META_CAPTURE  = 0x01000000;
    }
}

pub enum CameraType {
    Physical,
    Virtual,
}

pub struct Camera {
    pub uuid: String,
    pub name: String,
    pub path: String,
    pub driver: String,
    pub bus_info: String,
    pub capabilities: CameraCapabilities,
    pub camera_type: CameraType,
}

impl Camera {
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

pub fn get_all_cameras() -> Vec<Camera> {
    let mut cameras: Vec<Camera> = Vec::new();

    let video_devices = match fs::read_dir("/dev") {
        Ok(entries) => entries,
        Err(_) => return cameras,
    };

    for entry in video_devices.flatten() {
        let path = entry.path();

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if !name.starts_with("video") {
            continue;
        }

        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let Some(device) = UdevDevice::from_devnum(DeviceType::Character, meta.st_rdev()).ok() else {
            continue;
        };

        let is_usb = device.parent_with_subsystem("usb").ok().flatten().is_some();

        let uuid = device
            .property_value("ID_SERIAL")
            .or_else(|| device.property_value("ID_SERIAL_SHORT"))
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let name = device
            .property_value("ID_MODEL")
            .or_else(|| device.property_value("ID_V4L_PRODUCT"))
            .or_else(|| device.property_value("ID_V4L_DRIVER"))
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(name)
            .replace("_", " ");

        let driver = device
            .property_value("ID_V4L_DRIVER")
            .or_else(|| device.property_value("ID_V4L2_DRIVER"))
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let bus_info = device
            .property_value("ID_BUS")
            .or_else(|| device.property_value("ID_USB_INTERFACES"))
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

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
