use std::env;
use std::path::PathBuf;
use tokio::process::Command;

mod error;
mod event;
mod handle;
mod job;

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
mod tests {}
