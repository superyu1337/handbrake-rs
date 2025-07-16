use clap::Parser;
use handbrake::{HandBrake, InputSource, OutputDestination};
use std::path::PathBuf;

/// A simple example that builds a HandBrake job to copy all English subtitle tracks.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input video file
    #[arg(short, long)]
    input: PathBuf,

    /// Output video file
    #[arg(short, long)]
    output: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Attempt to discover HandBrakeCLI
    let handbrake = match HandBrake::new().await {
        Ok(hb) => hb,
        Err(e) => {
            eprintln!("Error finding HandBrakeCLI: {}", e);
            eprintln!("Please ensure HandBrakeCLI is installed and in your system PATH.");
            return;
        }
    };

    println!("Found HandBrakeCLI version: {}", handbrake.version());

    println!(
        "Building a HandBrake job to copy all English subtitle tracks from '{}' to '{}'",
        args.input.display(),
        args.output.display()
    );

    let job_builder = handbrake
        .job(
            InputSource::File(args.input),
            OutputDestination::File(args.output),
        )
        .preset("Fast 1080p30") // Example preset
        .subtitle_lang("eng"); // Copy all English subtitle tracks

    let args = job_builder.build_args();
    println!("HandBrakeCLI arguments: {:?}", args);

    println!("Note: This example only prints the command arguments and does not execute the job.");
    println!("To run the job, you would call .status().await or .start()");
}