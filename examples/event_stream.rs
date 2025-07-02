use std::time::{Duration, Instant};

use futures::StreamExt;
use handbrake_rs::{HandBrake, InputSource, JobEvent, OutputDestination};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Attempt to discover HandBrakeCLI
    let handbrake = match HandBrake::new().await {
        Ok(hb) => hb,
        Err(e) => {
            error!("Error finding HandBrakeCLI: {}", e);
            error!("Please ensure HandBrakeCLI is installed and in your system PATH.");
            return;
        }
    };
    info!("Found HandBrakeCLI: {}", handbrake.version());

    info!("Starting job with a non-existent file to demonstrate error events.");
    let input = InputSource::from(std::path::PathBuf::from(
        "completely-random-file-name.mkv",
    ));

    let mut job_handle = match handbrake
        .job(input, OutputDestination::Stdout)
        .video_codec("qwerty")
        .format("mkv")
        .start()
    {
        Ok(handle) => handle,
        Err(e) => {
            error!("Failed to start job: {}", e);
            return;
        }
    };

    info!("Job started. Monitoring events...");

    let mut event_stream = job_handle.events();
    let mut then = Instant::now();
    while let Some(event) = event_stream.next().await {
        match event {
            JobEvent::Config(config) => info!(?config, "Job config received"),
            JobEvent::Progress(progress) => info!(?progress, "Progress update"),
            JobEvent::Log(log) => info!(?log, "Log message"),
            JobEvent::Done(result) => {
                info!(?result, "Job finished");
                break;
            },
            JobEvent::Fragment(fragment) => {
                if then.elapsed() >= Duration::from_secs(10) {
                    info!("Fragment received: {} bytes", fragment.len());
                    then = Instant::now();
                }
            }
        }
    }
}
