use std::time::{Duration, Instant};

use clap::Parser;
use futures::StreamExt;
use handbrake::{HandBrake, InputSource, JobEvent, OutputDestination};
use tracing::{error, info};

/// A simple example of using the event stream.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The path to the input file.
    #[arg(short, long)]
    input: std::path::PathBuf,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

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

    info!("Starting job with input file: {:?}", cli.input);
    let input = InputSource::from(cli.input);

    let mut job_handle = match handbrake
        .job(input, OutputDestination::Stdout)
        .preset("Very Fast 1080p30")
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
            }
            JobEvent::Fragment(fragment) => {
                if then.elapsed() >= Duration::from_secs(10) {
                    info!("Fragment received: {} bytes", fragment.len());
                    then = Instant::now();
                }
            }
        }
    }
}
