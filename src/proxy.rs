use std::os::fd::AsFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use nix::poll::{poll, PollFd, PollFlags};
use v4l2r::bindings::{v4l2_captureparm, v4l2_fract, v4l2_streamparm};
use v4l2r::ioctl::*;
use v4l2r::memory::{MemoryType, MmapHandle};
use v4l2r::{Format, PixelFormat, QueueType};

use crate::camera::Camera;

fn set_capture_frame_rate(fd: &mut std::fs::File, fps: u32) -> Result<(), String> {
    let mut parm: v4l2_streamparm = unsafe { std::mem::zeroed() };
    parm.type_ = QueueType::VideoCapture as u32;

    let capture = v4l2_captureparm {
        capability: 0x1000, // V4L2_CAP_TIMEPERFRAME
        capturemode: 0,
        timeperframe: v4l2_fract {
            numerator: 1,
            denominator: fps,
        },
        extendedmode: 0,
        readbuffers: 0,
        reserved: [0; 4],
    };

    unsafe {
        std::ptr::write(std::ptr::addr_of_mut!(parm.parm.capture), capture);
    }

    let result: Result<v4l2_streamparm, _> = s_parm(fd, parm);
    match result {
        Ok(_) => {
            println!("Frame rate fijado a {} fps", fps);
            Ok(())
        }
        Err(e) => Err(format!("No se pudo fijar frame rate: {}", e)),
    }
}

fn configure_input_format(fd: &mut std::fs::File) -> Result<Format, String> {
    // Priorizamos YUYV 640x480 porque es el formato mas compatible
    // con v4l2loopback y aplicaciones como Discord/Chromium.
    let candidates: [([u8; 4], u32, u32); 4] = [
        (*b"YUYV", 640, 480),
        (*b"MJPG", 1280, 720),
        (*b"MJPG", 640, 480),
        (*b"YUYV", 320, 240),
    ];

    for (fourcc, w, h) in candidates {
        let mut fmt = Format::from((PixelFormat::from_fourcc(&fourcc), (w as usize, h as usize)));
        fmt.plane_fmt.clear();

        let result: Result<Format, SFmtError> = s_fmt(fd, (QueueType::VideoCapture, &fmt));
        match result {
            Ok(set_fmt) => {
                if set_fmt.pixelformat == PixelFormat::from_fourcc(&fourcc)
                    && set_fmt.width == w
                    && set_fmt.height == h
                {
                    return Ok(set_fmt);
                }
            }
            Err(_) => continue,
        }
    }

    Err("No se pudo configurar ningún formato soportado en la cámara de entrada".to_string())
}

pub fn start_proxy(
    input: &Camera,
    output: &Camera,
    running: Arc<AtomicBool>,
) -> Result<(), String> {
    // ---- Input (cámara física) ----
    let mut input_fd = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&input.path)
        .map_err(|e| format!("Error abriendo input {}: {}", input.path, e))?;

    let caps: v4l2r::ioctl::Capability = querycap(&input_fd)
        .map_err(|e| format!("Error querycap input: {}", e))?;
    if !caps.device_caps().contains(Capabilities::VIDEO_CAPTURE)
        && !caps.device_caps().contains(Capabilities::VIDEO_CAPTURE_MPLANE)
    {
        return Err(format!(
            "El dispositivo input {} no soporta VIDEO_CAPTURE",
            input.path
        ));
    }

    let source_fmt = configure_input_format(&mut input_fd)?;
    println!(
        "Formato input: {}x{} {:?}",
        source_fmt.width, source_fmt.height, source_fmt.pixelformat
    );

    set_capture_frame_rate(&mut input_fd, 30)
        .unwrap_or_else(|e| eprintln!("Advertencia: {}", e));

    // ---- Output (v4l2loopback) ----
    let mut output_fd = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&output.path)
        .map_err(|e| format!("Error abriendo output {}: {}", output.path, e))?;

    let mut out_fmt =
        Format::from((source_fmt.pixelformat, (source_fmt.width as usize, source_fmt.height as usize)));
    out_fmt.plane_fmt = source_fmt.plane_fmt.clone();

    let sink_fmt: Format = s_fmt(&mut output_fd, (QueueType::VideoOutput, &out_fmt))
        .map_err(|e| format!("Error configurando formato en output: {}", e))?;

    if source_fmt.width != sink_fmt.width
        || source_fmt.height != sink_fmt.height
        || source_fmt.pixelformat != sink_fmt.pixelformat
    {
        return Err(format!(
            "No se pudo forzar el formato {}/{}x{} en la salida",
            source_fmt.pixelformat, source_fmt.width, source_fmt.height
        ));
    }

    // ---- Buffers mmap para CAPTURE ----
    let buf_count: u32 = 4;
    let _req: RequestBuffers = reqbufs(
        &input_fd,
        QueueType::VideoCapture,
        MemoryType::Mmap,
        buf_count,
        MemoryConsistency::empty(),
    )
    .map_err(|e| format!("Error reqbufs input: {}", e))?;

    let mut mappings: Vec<PlaneMapping> = Vec::with_capacity(buf_count as usize);
    for i in 0..buf_count {
        let qbuf_info: QueryBuffer = querybuf(&input_fd, QueueType::VideoCapture, i as usize)
            .map_err(|e| format!("Error querybuf {}: {}", i, e))?;
        let plane = &qbuf_info.planes[0];
        let mapping = mmap(&input_fd, plane.mem_offset, plane.length)
            .map_err(|e| format!("Error mmap {}: {}", i, e))?;
        mappings.push(mapping);
    }

    // Queue inicial de todos los buffers
    for i in 0..buf_count {
        let mut buf = QBuffer::<MmapHandle>::new(QueueType::VideoCapture, i);
        buf.planes = vec![QBufPlane::new(0)];
        let _: V4l2Buffer = qbuf(&input_fd, buf)
            .map_err(|e| format!("Error qbuf inicial {}: {}", i, e))?;
    }

    streamon(&input_fd, QueueType::VideoCapture)
        .map_err(|e| format!("Error streamon: {}", e))?;

    println!("Proxy activo. Presiona Ctrl+C para detener.");

    // ---- Bucle principal ----
    while running.load(Ordering::Relaxed) {
        let poll_fd = PollFd::new(input_fd.as_fd(), PollFlags::POLLIN);
        match poll(&mut [poll_fd], 50u16) {
            Ok(0) => continue,
            Ok(_) => {}
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => return Err(format!("Error poll: {}", e)),
        }

        let buf: V4l2Buffer = dqbuf(&input_fd, QueueType::VideoCapture)
            .map_err(|e| format!("Error dqbuf: {}", e))?;
        let index = buf.index() as usize;
        let bytes_used = *buf.get_first_plane().bytesused as usize;

        if bytes_used > 0 {
            let data = &mappings[index].data[..bytes_used];
            std::io::Write::write_all(&mut output_fd, data)
                .map_err(|e| format!("Error escribiendo frame: {}", e))?;
        }

        let mut buffer = QBuffer::<MmapHandle>::new(QueueType::VideoCapture, index as u32);
        buffer.planes = vec![QBufPlane::new(0)];
        let _: V4l2Buffer = qbuf(&input_fd, buffer)
            .map_err(|e| format!("Error re-queue buffer: {}", e))?;
    }

    let _ = streamoff(&input_fd, QueueType::VideoCapture);
    Ok(())
}
