# handbrake-rs

`handbrake-rs` is a Rust crate that provides a safe, ergonomic, and asynchronous interface for controlling the `HandBrakeCLI` video transcoder.

It allows Rust applications to programmatically start, configure, monitor, and control HandBrake encoding jobs without needing to manually handle command-line arguments or parse raw text output.

## Features

- **Fluent Job Configuration**: Use a builder pattern to easily configure encoding jobs (e.g., `job.preset("Fast 1080p30").video_codec("x265")`).
- **Asynchronous API**: Built on `tokio`, the entire API is `async`, making it suitable for modern, high-performance applications.
- **Real-time Monitoring**: Subscribe to a stream of structured events for progress updates, logs, and job completion notifications.
- **Two Execution Modes**:
    - **Monitored**: Get a `JobHandle` to receive live events and control the process.
    - **Fire-and-Forget**: Simply execute a job and wait for its final exit status.
- **Process Control**: Gracefully `cancel()` or forcefully `kill()` a running encoding job.
- **Flexible Setup**: Automatically finds `HandBrakeCLI` in your system's `PATH` or lets you specify a direct path to the executable.

## Quick Start

First, add `handbrake-rs` to your `Cargo.toml`:

```toml
[dependencies]
handbrake-rs = "0.1.0" # Replace with the latest version
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

Here is a basic example of how to start and monitor an encoding job.

```rust
use handbrake_rs::{HandBrake, JobEvent, LogLevel};
use futures::StreamExt;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Find HandBrakeCLI in the system PATH
    let hb = HandBrake::new().await?;

    let input = PathBuf::from("path/to/your/video.mkv");
    let output = PathBuf::from("path/to/your/output.mp4");

    // Configure the encoding job
    let mut job_handle = hb
        .job(input.into(), output.into())
        .preset("Fast 1080p30")
        .video_codec("x265")
        .start()?; // Start in monitored mode

    // Listen for events
    while let Some(event) = job_handle.events().next().await {
        match event {
            JobEvent::Progress(p) => {
                println!(
                    "Encoding: {:.2}% complete, FPS: {:.2}, ETA: {}",
                    p.percentage, p.fps, p.eta
                );
            }
            JobEvent::Log(log) => {
                if log.level == LogLevel::Error {
                    eprintln!("[HandBrake Log] ERROR: {}", log.message);
                } else {
                    println!("[HandBrake Log] {}", log.message);
                }
            }
            JobEvent::Done(result) => match result {
                Ok(summary) => println!("Done! Encoding took {:?}", summary.total_time),
                Err(failure) => eprintln!("Encoding failed: {}", failure.final_error_message),
            },
        }
    }

    Ok(())
}
```

## How it Works

The crate is designed around three main components:

1.  **`HandBrake`**: The factory for creating jobs. It locates and validates the `HandBrakeCLI` executable on your system.
2.  **`JobBuilder`**: A fluent interface to define all the parameters for a specific encoding job.
3.  **`JobHandle`**: Returned when a job is started in monitored mode. It represents the running process and gives you access to the event stream and control methods like `cancel()` and `kill()`.

This crate works by spawning `HandBrakeCLI` as a child process and parsing its `stderr` output in real-time to generate a structured stream of `JobEvent`s.
