use handbrake::{HandBrake, InputSource, OutputDestination};
use std::fs;
use std::path::PathBuf;

/// This integration test checks the `status()` method.
/// It is marked `#[ignore]` because it requires a real `HandBrakeCLI` executable
/// to be present in the system's PATH.
///
/// To run this test, use: `cargo test -- --ignored`
#[tokio::test]
#[ignore]
async fn test_status_with_nonexistent_input_file() {
    // Attempt to find HandBrakeCLI in the path.
    let hb_result = HandBrake::new().await;
    if hb_result.is_err() {
        println!("Skipping integration test: HandBrakeCLI not found in PATH.");
        // If the executable isn't found, we can't run the test.
        // We'll vacuously pass the test, as the condition for running it isn't met.
        return;
    }
    let hb = hb_result.unwrap();

    // Define a non-existent input file. This will cause HandBrakeCLI to exit with an error.
    let input_path = PathBuf::from("this_file_definitely_does_not_exist.mkv");

    // Define an output path in the system's temp directory.
    let output_path = std::env::temp_dir().join("handbrake_rs_test_output.mp4");

    // Ensure the output file from a previous failed run doesn't exist.
    let _ = fs::remove_file(&output_path);

    let job = hb.job(
        InputSource::File(input_path),
        OutputDestination::File(output_path.clone()),
    );

    // Execute the job in "fire-and-forget" mode.
    let result = job.status().await;

    // The `status()` method should succeed, as spawning the process should work.
    let status = result.expect("Spawning HandBrakeCLI process should not fail.");

    // However, the process itself should report a failure (non-zero exit code)
    // because the input file does not exist.
    assert!(!status.success(), "HandBrakeCLI should have failed due to non-existent input file, but it succeeded.");

    // Clean up the output file if it was created (it shouldn't be, but it's good practice).
    let _ = fs::remove_file(&output_path);
}