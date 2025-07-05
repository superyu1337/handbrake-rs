//! A safe, ergonomic, and asynchronous Rust crate for controlling the `HandBrakeCLI` video transcoder.
//! 
//! `handbrake-rs` allows Rust applications to programmatically start, configure, monitor, and control
//! HandBrake encoding jobs without needing to manually handle command-line arguments or parse raw text output.
//! 
//! # Features
//! 
//! - **Fluent Job Configuration**: Use a builder pattern to easily configure encoding jobs.
//! - **Asynchronous API**: Built on `tokio`, the entire API is `async`.
//! - **Real-time Monitoring**: Subscribe to a stream of structured events for progress, logs, and job completion.
//! - **Process Control**: Gracefully `cancel()` or forcefully `kill()` a running encoding job.
//! - **Flexible Setup**: Automatically finds `HandBrakeCLI` in the system `PATH` or allows specifying a direct path.
//! 
//! # Quick Start
//! 
//! ```rust,no_run
//! use handbrake_rs::{HandBrake, JobEvent};
//! use futures::StreamExt;
//! use std::path::PathBuf;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Find HandBrakeCLI in the system PATH
//!     let hb = HandBrake::new().await?;
//! 
//!     let input = PathBuf::from("path/to/your/video.mkv");
//!     let output = PathBuf::from("path/to/your/output.mp4");
//! 
//!     // Configure and start the encoding job
//!     let mut job_handle = hb
//!         .job(input.into(), output.into())
//!         .preset("Fast 1080p30")
//!         .quality(22.0)
//!         .start()?;
//! 
//!     // Listen for events
//!     while let Some(event) = job_handle.events().next().await {
//!         match event {
//!             JobEvent::Progress(p) => {
//!                 println!("Encoding: {:.2}% complete", p.percentage);
//!             }
//!             JobEvent::Done(result) => {
//!                 if result.is_ok() {
//!                     println!("Done!");
//!                 } else {
//!                     eprintln!("Encoding failed.");
//!                 }
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//! 
//!     Ok(())
//! }
//! ```

use std::env;
use std::path::PathBuf;
#[cfg(not(test))]
use tokio::process::Command;

mod error;
mod event;
mod handle;
mod job;

#[cfg(test)]
mod testing;

#[cfg(test)]
use testing::mock_command::MockCommand as Command;

/// Validates that the given path points to a runnable HandBrakeCLI executable.
/// Runs `HandBrakeCLI --version` and checks the exit code.
async fn validate_executable(path: &PathBuf) -> Result<String, Error> {
    let mut command = Command::new(path);
    let output = command
        .arg("--version")
        .output()
        .await
        .map_err(|e| Error::InvalidExecutable {
            path: path.clone(),
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(Error::InvalidExecutable {
            path: path.clone(),
            reason: format!(
                "'--version' command failed with exit code: {:?}",
                output.status.code()
            ),
        });
    }

    let version_string = String::from_utf8(output.stdout)
        .map_err(|e| Error::InvalidExecutable {
            path: path.clone(),
            reason: format!("Failed to parse version output as UTF-8: {}", e),
        })?
        .trim()
        .to_string();

    if version_string.is_empty() {
        return Err(Error::InvalidExecutable {
            path: path.clone(),
            reason: "HandBrakeCLI --version returned empty output".to_string(),
        });
    }

    Ok(version_string)
}

/// Searches the given PATH string for the HandBrake executable.
fn find_executable_in_path(path_env: &std::ffi::OsStr) -> Result<PathBuf, Error> {
    let paths = env::split_paths(path_env).collect::<Vec<_>>();
    for path in &paths {
        let executable_path = path.join("HandBrakeCLI");
        if executable_path.is_file() {
            return Ok(executable_path);
        }
    }

    Err(Error::ExecutableNotFound {
        searched_paths: paths,
    })
}

pub use error::Error;
pub use event::{
    AudioConfig, AudioTrackConfig, Config, DestinationConfig, JobEvent, JobFailure, Log, Progress,
    SourceConfig, VideoConfig,
};
pub use handle::JobHandle;
pub use job::{InputSource, JobBuilder, OutputDestination};

/// The main entry point for the `handbrake-rs` crate.
///
/// This struct is responsible for locating and validating the `HandBrakeCLI` executable.
/// It acts as a factory for creating new `JobBuilder` instances.
#[derive(Debug)]
pub struct HandBrake {
    executable_path: PathBuf,
    version: String,
}

impl HandBrake {
    /// Creates a new `HandBrake` instance by searching for `HandBrakeCLI` in the system `PATH`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if `HandBrakeCLI` is not found in any of the directories listed
    /// in the `PATH` environment variable, or if the found executable is invalid (e.g., fails to
    /// return a version string).
    pub async fn new() -> Result<Self, Error> {
        let path_var = env::var_os("PATH").expect("PATH environment variable not set");
        let executable_path = find_executable_in_path(&path_var)?;
        let version = validate_executable(&executable_path).await?;
        Ok(Self {
            executable_path,
            version,
        })
    }

    /// Creates a new `HandBrake` instance using a specific path to `HandBrakeCLI`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the file at the given path does not exist, is not executable,
    /// or is an unsupported version of `HandBrakeCLI`.
    pub async fn new_with_path(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let executable_path = path.into();
        let version = validate_executable(&executable_path).await?;
        Ok(Self {
            executable_path,
            version,
        })
    }

    /// Returns the version string of the discovered `HandBrakeCLI` executable.
    ///
    /// The version is obtained by running `HandBrakeCLI --version` during initialization.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Creates a new `JobBuilder` to configure an encoding job.
    ///
    /// # Arguments
    ///
    /// * `input` - The source for the encoding job (e.g., a file path or stdin).
    /// * `output` - The destination for the encoded file (e.g., a file path or stdout).
    pub fn job(&self, input: InputSource, output: OutputDestination) -> JobBuilder {
        JobBuilder::new(self.executable_path.clone(), input, output)
    }
}

#[cfg(test)]
mod tests {
    use super::{HandBrake, validate_executable};
    use crate::testing::mock_command::{MockCommandExpect, MockResult};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_validate_executable_success() {
        MockCommandExpect::clear_all_expectations();
        let test_path = PathBuf::from("/usr/local/bin/HandBrakeCLI");
        MockCommandExpect::when(&test_path)
            .with_arg("--version")
            .returns(MockResult::success().with_stdout(b"HandBrake 1.5.0\n"));

        let version = validate_executable(&test_path).await.unwrap();
        assert_eq!(version, "HandBrake 1.5.0");
    }

    #[tokio::test]
    async fn test_validate_executable_failure_non_zero_exit() {
        MockCommandExpect::clear_all_expectations();
        let test_path = PathBuf::from("/usr/local/bin/HandBrakeCLI");
        MockCommandExpect::when(&test_path)
            .with_arg("--version")
            .returns(MockResult::failure(1).with_stderr(b"Error: Something went wrong\n"));

        let err = validate_executable(&test_path).await.unwrap_err();
        assert!(
            matches!(err, super::Error::InvalidExecutable { path, reason } if path == test_path && reason.contains("command failed with exit code"))
        );
    }

    #[tokio::test]
    async fn test_validate_executable_failure_empty_output() {
        MockCommandExpect::clear_all_expectations();
        let test_path = PathBuf::from("/usr/local/bin/HandBrakeCLI");
        MockCommandExpect::when(&test_path)
            .with_arg("--version")
            .returns(MockResult::success().with_stdout(b"")); // Empty stdout

        let err = validate_executable(&test_path).await.unwrap_err();
        assert!(
            matches!(err, super::Error::InvalidExecutable { path, reason } if path == test_path && reason.contains("returned empty output"))
        );
    }

    #[tokio::test]
    async fn test_validate_executable_failure_invalid_utf8() {
        MockCommandExpect::clear_all_expectations();
        let test_path = PathBuf::from("/usr/local/bin/HandBrakeCLI");
        MockCommandExpect::when(&test_path)
            .with_arg("--version")
            .returns(MockResult::success().with_stdout(vec![0xC3, 0x28])); // Invalid UTF-8 sequence

        let err = validate_executable(&test_path).await.unwrap_err();
        assert!(
            matches!(err, super::Error::InvalidExecutable { path, reason } if path == test_path && reason.contains("Failed to parse version output as UTF-8"))
        );
    }

    #[tokio::test]
    async fn test_handbrake_new_with_path_success() {
        MockCommandExpect::clear_all_expectations();
        let test_path = PathBuf::from("/opt/handbrake/HandBrakeCLI");
        MockCommandExpect::when(&test_path)
            .with_arg("--version")
            .returns(MockResult::success().with_stdout(b"HandBrake 1.6.0\n"));

        let hb = HandBrake::new_with_path(&test_path).await.unwrap();
        assert_eq!(hb.executable_path, test_path);
        assert_eq!(hb.version(), "HandBrake 1.6.0");
    }

    #[tokio::test]
    async fn test_handbrake_new_with_path_invalid_executable() {
        MockCommandExpect::clear_all_expectations();
        let test_path = PathBuf::from("/nonexistent/HandBrakeCLI");
        MockCommandExpect::when(&test_path)
            .with_arg("--version")
            .returns(MockResult::failure(127).with_stderr(b"command not found\n"));

        let err = HandBrake::new_with_path(&test_path).await.unwrap_err();
        assert!(
            matches!(err, super::Error::InvalidExecutable { path, reason } if path == test_path && reason.contains("command failed with exit code"))
        );
    }
}
