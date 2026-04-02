use clap::Parser;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::signal;
use tokio::time::{Duration, sleep};

#[derive(Parser, Debug)]
#[command(name = "webcam-proxy")]
#[command(about = "A webcam proxy tool", long_about = None)]
struct Args {
    #[arg(long, default_value = "/dev/video0", help = "Input webcam device")]
    input: String,

    #[arg(
        long,
        default_value = "/dev/video10",
        help = "Output v4l2loopback device"
    )]
    output: String,

    #[arg(long, default_value_t = 1280, help = "Video width")]
    width: u32,

    #[arg(long, default_value_t = 720, help = "Video height")]
    height: u32,

    #[arg(long, default_value_t = 12, help = "Frames per second")]
    fps: u32,

    #[arg(
        long,
        default_value_t = 3,
        help = "Delay between retries on failure (seconds)"
    )]
    retry: u64,
}

fn check_device(path: &str) -> bool {
    Path::new(path).exists()
}

async fn wait_for_device(path: &str, retry_delay: Duration) {
    loop {
        if check_device(path) {
            return;
        }
        println!("Waiting for device {}...", path);
        sleep(retry_delay).await;
    }
}

async fn run_proxy(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let video_size = format!("{}x{}", args.width, args.height);
    let framerate = format!("{}", args.fps);

    let ffmpeg_args = vec![
        "-loglevel",
        "quiet",
        "-err_detect",
        "ignore_err",
        "-fflags",
        "nobuffer",
        "-flags",
        "low_delay",
        "-probesize",
        "32",
        "-analyzeduration",
        "0",
        "-thread_queue_size",
        "512",
        "-f",
        "v4l2",
        "-input_format",
        "mjpeg",
        "-video_size",
        &video_size,
        "-framerate",
        &framerate,
        "-i",
        &args.input,
        "-vf",
        "format=yuv420p",
        "-f",
        "v4l2",
        &args.output,
    ];

    println!(
        "Starting proxy: {} -> {} ({} @ {}fps)",
        args.input, args.output, video_size, args.fps
    );

    let mut child = Command::new("ffmpeg")
        .args(&ffmpeg_args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("Shutting down...");
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        status = child.wait() => {
            let status = status?;
            if !status.success() {
                return Err(format!("ffmpeg exited with status: {}", status).into());
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let retry_delay = Duration::from_secs(args.retry);

    // Check ffmpeg is available
    if which::which("ffmpeg").is_err() {
        eprintln!("ffmpeg not found in PATH. Install it with: sudo dnf install ffmpeg");
        std::process::exit(1);
    }

    println!("webcam-proxy starting");
    println!("  input:  {}", args.input);
    println!("  output: {}", args.output);

    wait_for_device(&args.input, retry_delay).await;

    loop {
        if !check_device(&args.output) {
            eprintln!(
                "Output device {} not found. Load v4l2loopback first:\n  sudo modprobe v4l2loopback devices=1 video_nr=10 card_label=\"WebcamVirtual\" exclusive_caps=1",
                args.output
            );
            std::process::exit(1);
        }

        if let Err(e) = run_proxy(&args).await {
            println!("Proxy error: {} — retrying in {:?}", e, retry_delay);
            sleep(retry_delay).await;
            continue;
        }
        break;
    }
}
