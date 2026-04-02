# webcam-proxy

`webcam-proxy` is a lightweight webcam proxy tool for Linux. It reads from a physical camera input, transcodes it with `ffmpeg`, and writes the video stream to a virtual `v4l2loopback` device so other applications can use it as a regular webcam.

The goal is to work around webcam compatibility issues on Linux while maintaining the lowest possible latency.

## What it does

- Waits for the input camera to be available.
- If the output virtual device doesn't exist, attempts to create it automatically with `v4l2loopback`.
- Runs `ffmpeg` to read from the physical webcam and write to the virtual device.
- If `ffmpeg` crashes, retries after a short delay.
- Uses low-latency settings to minimize video delay and stuttering.

## Requirements

- Linux.
- `ffmpeg` installed and accessible in `PATH`.
- `v4l2loopback` available on the system (as a loadable kernel module via `modprobe`).
- Read permissions for the input camera device.
- Write permissions for the output virtual device.
- Permissions to run `modprobe` for automatic output device creation.

## Prepare the virtual camera

If the output device doesn't exist, the program automatically attempts to create it with:

```bash
modprobe v4l2loopback devices=1 video_nr=N card_label="WebcamVirtual" exclusive_caps=1
```

Where `N` is extracted from `--output` (e.g., `/dev/video10` becomes `video_nr=10`).

Alternatively, you can load the module manually before starting:

```bash
sudo modprobe v4l2loopback devices=1 video_nr=10 card_label="WebcamVirtual" exclusive_caps=1
```

This creates the `/dev/video10` device.

## Build

```bash
cargo build --release
```

The binary will be at:

```bash
./target/release/webcam-proxy
```

## Run

```bash
sudo ./target/release/webcam-proxy
```

Default settings:

- Input: `/dev/video0`
- Output: `/dev/video10`
- Resolution: `1280x720`
- FPS: `12`
- Retry delay: `3s`

## Available options

```bash
sudo ./target/release/webcam-proxy --help
```

Parameters:

- `--input`: Input device path (physical webcam).
- `--output`: Output device path (v4l2loopback device).
- `--width`: Video width.
- `--height`: Video height.
- `--fps`: Frames per second.
- `--retry`: Delay (in seconds) between retries on input missing or ffmpeg failure.

Example with custom settings:

```bash
sudo ./target/release/webcam-proxy --input /dev/video2 --output /dev/video10 --width 1920 --height 1080 --fps 30
```

## Internal behavior

The program runs `ffmpeg` with low-latency settings:

- `-fflags nobuffer`
- `-flags low_delay`
- `-probesize 32`
- `-analyzeduration 0`
- `-thread_queue_size 512`
- `-err_detect ignore_err`

`ffmpeg` output is silenced to avoid spam from recoverable errors.

If `ffmpeg` crashes, the program retries on the same virtual device. No new camera is created per restart—it reuses the same output (`--output`) throughout the session.

## Common issues

### `ffmpeg not found in PATH`

Install `ffmpeg` and ensure it's available in your terminal.

### `Output device ... not found`

The program will attempt to create it automatically. If it fails, check `modprobe` permissions, `v4l2loopback` availability, and that `--output` follows `/dev/videoN` format.

### Virtual webcam doesn't appear in applications

Verify that `v4l2loopback` is loaded and that the application has read permissions for the device (e.g., `/dev/video10`).

### Input camera takes time to appear

The program automatically waits for the input device before starting `ffmpeg`.

## License

See [LICENSE](LICENSE) for license details.

---

**[Español](README.es.md)**
