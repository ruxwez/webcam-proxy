mod camera;

fn main() {
    // Crear cámara virtual fija en /dev/video62
    if let Err(e) = camera::init_virtual_camera() {
        eprintln!("aviso: no se pudo crear cámara virtual: {}", e);
    }

    let cameras = camera::get_all_cameras();

    for camera in cameras {
        let type_str = match camera.camera_type {
            camera::CameraType::Physical => "fisica",
            camera::CameraType::Virtual => "virtual",
        };
        println!(
            "[{}] {} ({}) uuid: {}",
            type_str, camera.name, camera.path, camera.uuid
        );
    }
}
