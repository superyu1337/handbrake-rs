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
use testing::mock_command::{MockCommand as Command};

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
pub use event::{JobEvent, JobFailure, JobSummary, Log, LogLevel, Progress};
// pub use handle::JobHandle; // Will be re-exported later
pub use job::{InputSource, JobBuilder, OutputDestination}; // Will be re-exported later

/// Represents the HandBrake executable.
#[derive(Debug)]
pub struct HandBrake {
    executable_path: PathBuf,
    version: String,
}

impl HandBrake {
    /// Discovers the HandBrake executable in the system PATH.
    pub async fn new() -> Result<Self, Error> {
        let path_var = env::var_os("PATH").expect("PATH environment variable not set");
        let executable_path = find_executable_in_path(&path_var)?;
        let version = validate_executable(&executable_path).await?;
        Ok(Self {
            executable_path,
            version,
        })
    }

    /// Creates a new HandBrake instance with a specific executable path.
    pub async fn new_with_path(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let executable_path = path.into();
        let version = validate_executable(&executable_path).await?;
        Ok(Self {
            executable_path,
            version,
        })
    }

    /// Returns the version string of the HandBrakeCLI executable.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Starts building a new HandBrake job.
    pub fn job(&self, input: InputSource, output: OutputDestination) -> JobBuilder {
        JobBuilder::new(self.executable_path.clone(), input, output)
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_executable, HandBrake};
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
        assert!(matches!(err, super::Error::InvalidExecutable { path, reason } if path == test_path && reason.contains("command failed with exit code")));
    }

    #[tokio::test]
    async fn test_validate_executable_failure_empty_output() {
        MockCommandExpect::clear_all_expectations();
        let test_path = PathBuf::from("/usr/local/bin/HandBrakeCLI");
        MockCommandExpect::when(&test_path)
            .with_arg("--version")
            .returns(MockResult::success().with_stdout(b"")); // Empty stdout

        let err = validate_executable(&test_path).await.unwrap_err();
        assert!(matches!(err, super::Error::InvalidExecutable { path, reason } if path == test_path && reason.contains("returned empty output")));
    }

    #[tokio::test]
    async fn test_validate_executable_failure_invalid_utf8() {
        MockCommandExpect::clear_all_expectations();
        let test_path = PathBuf::from("/usr/local/bin/HandBrakeCLI");
        MockCommandExpect::when(&test_path)
            .with_arg("--version")
            .returns(MockResult::success().with_stdout(vec![0xC3, 0x28])); // Invalid UTF-8 sequence

        let err = validate_executable(&test_path).await.unwrap_err();
        assert!(matches!(err, super::Error::InvalidExecutable { path, reason } if path == test_path && reason.contains("Failed to parse version output as UTF-8")));
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
        assert!(matches!(err, super::Error::InvalidExecutable { path, reason } if path == test_path && reason.contains("command failed with exit code")));
    }
}
