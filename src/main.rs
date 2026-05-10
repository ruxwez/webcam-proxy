mod camera;
mod proxy;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    if let Err(e) = camera::init_virtual_camera() {
        eprintln!("aviso: no se pudo crear cámara virtual: {}", e);
        return;
    }

    let cameras = camera::get_all_cameras();

    let input = cameras.iter().find(|c| matches!(c.camera_type, camera::CameraType::Physical));
    let output = cameras.iter().find(|c| c.path == "/dev/video62");

    let Some(input) = input else {
        eprintln!("No se encontró ninguna cámara física.");
        return;
    };
    let Some(output) = output else {
        eprintln!("No se encontró la cámara virtual /dev/video62.");
        return;
    };

    println!("Proxy: {} ({}) -> {} ({})", input.name, input.path, output.name, output.path);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })
    .expect("Error configurando manejador Ctrl+C");

    if let Err(e) = proxy::start_proxy(input, output, running) {
        eprintln!("Error en proxy: {}", e);
    }
}
