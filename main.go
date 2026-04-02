package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
	"os/signal"
	"syscall"
	"time"
)

var (
	input      = flag.String("input", "/dev/video0", "Input webcam device")
	output     = flag.String("output", "/dev/video10", "Output v4l2loopback device")
	width      = flag.Int("width", 1280, "Video width")
	height     = flag.Int("height", 720, "Video height")
	fps        = flag.Int("fps", 30, "Frames per second")
	retryDelay = flag.Duration("retry", 3*time.Second, "Delay between retries on failure")
)

func checkDevice(path string) error {
	_, err := os.Stat(path)
	return err
}

func waitForDevice(path string) {
	for {
		if err := checkDevice(path); err == nil {
			return
		}
		log.Printf("Waiting for device %s...", path)
		time.Sleep(*retryDelay)
	}
}

func runProxy() error {
	args := []string{
		"-loglevel", "quiet",
		"-err_detect", "ignore_err",
		"-fflags", "nobuffer",
		"-flags", "low_delay",
		"-probesize", "32",
		"-analyzeduration", "0",
		"-thread_queue_size", "512",
		"-f", "v4l2",
		"-input_format", "mjpeg",
		"-video_size", fmt.Sprintf("%dx%d", *width, *height),
		"-framerate", fmt.Sprintf("%d", *fps),
		"-i", *input,
		"-vf", "format=yuv420p",
		"-f", "v4l2",
		*output,
	}

	cmd := exec.Command("ffmpeg", args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	log.Printf("Starting proxy: %s -> %s (%dx%d @ %dfps)", *input, *output, *width, *height, *fps)

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start ffmpeg: %w", err)
	}

	// Handle signals to gracefully stop ffmpeg
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)

	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case <-sig:
		log.Println("Shutting down...")
		cmd.Process.Signal(syscall.SIGTERM)
		<-done
		return nil
	case err := <-done:
		if err != nil {
			return fmt.Errorf("ffmpeg exited: %w", err)
		}
		return nil
	}
}

func main() {
	flag.Parse()

	// Check ffmpeg is available
	if _, err := exec.LookPath("ffmpeg"); err != nil {
		log.Fatal("ffmpeg not found in PATH. Install it with: sudo dnf install ffmpeg")
	}

	log.Printf("webcam-proxy starting")
	log.Printf("  input:  %s", *input)
	log.Printf("  output: %s", *output)

	waitForDevice(*input)

	for {
		if err := checkDevice(*output); err != nil {
			log.Fatalf("Output device %s not found. Load v4l2loopback first:\n  sudo modprobe v4l2loopback devices=1 video_nr=10 card_label=\"WebcamVirtual\" exclusive_caps=1", *output)
		}

		if err := runProxy(); err != nil {
			log.Printf("Proxy error: %v — retrying in %s", err, *retryDelay)
			time.Sleep(*retryDelay)
			continue
		}
		break
	}
}
