use handbrake::{HandBrake, JobEvent};
use futures::StreamExt;
use std::path::PathBuf;
use std::time::Duration;

/// This integration test checks the `start()` method for a successful encode.
/// It is marked `#[ignore]` because it requires a real `HandBrakeCLI` executable
/// and a sample video file.
///
/// To run this test:
/// 1. Make sure `HandBrakeCLI` is in your PATH.
/// 2. Create a small video file at `tests/sample_video/sample.mp4`.
/// 3. Run `cargo test -- --ignored`.
#[tokio::test]
#[ignore]
async fn test_start_successful_encode() {
    let hb_result = HandBrake::new().await;
    if hb_result.is_err() {
        println!("Skipping integration test: HandBrakeCLI not found in PATH.");
        return;
    }
    let hb = hb_result.unwrap();

    let input_path = PathBuf::from("tests/sample_video/sample.mp4");
    if !input_path.exists() {
        println!("Skipping integration test: Sample video not found at 'tests/sample_video/sample.mp4'.");
        return;
    }

    let output_path = std::env::temp_dir().join("handbrake_rs_test_start_success.mp4");
    let _ = std::fs::remove_file(&output_path);

    let mut handle = hb
        .job(input_path.into(), output_path.clone().into())
        .preset("Very Fast 240p24")
        .start()
        .expect("Failed to start job");

    let mut got_config = false;
    let mut got_progress = false;
    let mut got_done = false;

    while let Some(event) = handle.events().next().await {
        match event {
            JobEvent::Config(_) => got_config = true,
            JobEvent::Progress(_) => got_progress = true,
            JobEvent::Done(result) => {
                assert!(result.is_ok(), "Job should have completed successfully");
                got_done = true;
                break;
            }
            _ => {}
        }
    }

    assert!(got_config, "Did not receive Config event");
    assert!(got_progress, "Did not receive Progress event");
    assert!(got_done, "Did not receive Done event");
    assert!(output_path.exists(), "Output file was not created");

    let _ = std::fs::remove_file(&output_path);
}

/// This integration test checks the `kill()` method.
#[tokio::test]
#[ignore]
async fn test_kill_job() {
    let hb_result = HandBrake::new().await;
    if hb_result.is_err() {
        println!("Skipping integration test: HandBrakeCLI not found in PATH.");
        return;
    }
    let hb = hb_result.unwrap();

    let input_path = PathBuf::from("tests/sample_video/sample.mp4");
    if !input_path.exists() {
        println!("Skipping integration test: Sample video not found at 'tests/sample_video/sample.mp4'.");
        return;
    }

    let output_path = std::env::temp_dir().join("handbrake_rs_test_kill.mp4");
    let _ = std::fs::remove_file(&output_path);

    let mut handle = hb
        .job(input_path.into(), output_path.clone().into())
        .preset("Very Fast 1080p30") // Use a slower preset to ensure it's running
        .start()
        .expect("Failed to start job");

    // Immediately kill the job
    handle.kill().await.expect("Failed to kill job");

    while let Some(event) = handle.events().next().await {
        if let JobEvent::Done(result) = event {
            assert!(result.is_err(), "Job should have failed due to being killed");
            break;
        }
    }

    // Give a moment for the file system to reflect the change
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!output_path.exists(), "Output file should not have been created");
}

/// This integration test checks the `cancel()` method.
#[tokio::test]
#[ignore]
async fn test_cancel_job() {
    let hb_result = HandBrake::new().await;
    if hb_result.is_err() {
        println!("Skipping integration test: HandBrakeCLI not found in PATH.");
        return;
    }
    let hb = hb_result.unwrap();

    let input_path = PathBuf::from("tests/sample_video/sample.mp4");
    if !input_path.exists() {
        println!("Skipping integration test: Sample video not found at 'tests/sample_video/sample.mp4'.");
        return;
    }

    let output_path = std::env::temp_dir().join("handbrake_rs_test_cancel.mp4");
    let _ = std::fs::remove_file(&output_path);

    let mut handle = hb
        .job(input_path.into(), output_path.clone().into())
        .preset("Very Fast 1080p30")
        .start()
        .expect("Failed to start job");

    // Cancel the job gracefully
    handle.cancel().await.expect("Failed to cancel job");

    while let Some(event) = handle.events().next().await {
        if let JobEvent::Done(result) = event {
            assert!(result.is_err(), "Job should have failed due to being cancelled");
            break;
        }
    }

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!output_path.exists(), "Output file should not have been created");
}
