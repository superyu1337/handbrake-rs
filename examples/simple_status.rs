use handbrake::{HandBrake, InputSource, OutputDestination};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
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

    // Define dummy input and output paths for the example.
    // In a real application, these would point to actual video files.
    let input_path = PathBuf::from("input.mp4");
    let output_path = PathBuf::from("output.mkv");

    println!("Attempting to run a dummy HandBrake job (input: {}, output: {})",
             input_path.display(), output_path.display());
    println!("Note: This example will attempt to run HandBrakeCLI. If input.mp4 does not exist, HandBrakeCLI will likely fail.");

    let job_builder = handbrake.job(
        InputSource::File(input_path),
        OutputDestination::File(output_path),
    )
    .preset("Fast 1080p30") // Example preset
    .quality(20.0); // Example quality

    match job_builder.status().await {
        Ok(status) => {
            if status.success() {
                println!("HandBrake job completed successfully!");
            } else {
                eprintln!("HandBrake job failed with exit code: {:?}", status.code());
            }
        }
        Err(e) => {
            eprintln!("Error running HandBrake job: {}", e);
        }
    }
}
