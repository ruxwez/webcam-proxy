mod camera;

fn main() {
    let cameras = camera::get_all_cameras();

    for camera in cameras {
        let type_str = match camera.camera_type {
            camera::CameraType::Physical => "fisica",
            camera::CameraType::Virtual => "virtual",
        };
        println!("[{}] {} ({})", type_str, camera.name, camera.path);
    }
}
