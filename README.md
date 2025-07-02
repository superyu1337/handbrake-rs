# handbrake-rs

`handbrake-rs` is a Rust crate that provides a safe, ergonomic, and asynchronous interface for controlling the `HandBrakeCLI` video transcoder.

It allows Rust applications to programmatically start, configure, monitor, and control HandBrake encoding jobs without needing to manually handle command-line arguments or parse raw text output.

## Features

- **Fluent Job Configuration**: Use a builder pattern to easily configure encoding jobs (e.g., `job.preset("Fast 1080p30").quality(22.0)`).
- **Asynchronous API**: Built on `tokio`, the entire API is `async`, making it suitable for modern, high-performance applications.
- **Real-time Monitoring**: Subscribe to a stream of structured events:
    - `Config`: The full job configuration, parsed from HandBrake's JSON output.
    - `Progress`: Real-time updates on percentage, FPS, and ETA.
    - `Log`: Raw log messages from `HandBrakeCLI`.
    - `Fragment`: Raw `stdout` data, useful when piping video output.
    - `Done`: Signals the completion (success or failure) of the job.
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
use handbrake_rs::{HandBrake, JobEvent};
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
        .quality(22.0) // Set the quality
        .start()?; // Start in monitored mode

    // Listen for events
    while let Some(event) = job_handle.events().next().await {
        match event {
            JobEvent::Config(config) => {
                println!("[HandBrake] Job Config Received: {:#?}", config);
            }
            JobEvent::Progress(p) => {
                let eta = p.eta.map(|d| format!("{:?}", d)).unwrap_or_else(|| "N/A".to_string());
                println!(
                    "Encoding: {:.2}% complete, FPS: {:.2}, ETA: {}",
                    p.percentage, p.fps, eta
                );
            }
            JobEvent::Log(log) => {
                println!("[HandBrake Log] {}", log.message);
            }
            JobEvent::Fragment(data) => {
                // This event contains raw data from stdout that isn't progress info.
                // If you are piping the output to stdout, this will contain the video data.
                println!("[HandBrake] Received {} bytes of raw data.", data.len());
            }
            JobEvent::Done(result) => {
                match result {
                    Ok(()) => println!("Done! Encoding finished successfully."),
                    Err(failure) => eprintln!("Encoding failed: {}", failure.message),
                }
                break; // Exit the loop
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

This crate works by spawning `HandBrakeCLI` as a child process and parsing its `stdout` and `stderr` streams in real-time. It parses progress indicators from `stdout`, and JSON configuration and logs from `stderr`, to generate a structured stream of `JobEvent`s.
