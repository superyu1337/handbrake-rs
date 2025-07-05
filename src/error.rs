use thiserror::Error;

/// The primary error type for the `handbrake-rs` crate.
#[derive(Error, Debug)]
pub enum Error {
    /// The `HandBrakeCLI` executable could not be found in the system's `PATH`.
    #[error("HandBrake executable not found in system PATH. Searched paths: {searched_paths:?}")]
    ExecutableNotFound {
        /// The list of directories that were searched.
        searched_paths: Vec<std::path::PathBuf>,
    },
    /// The executable at the specified path is not a valid `HandBrakeCLI`.
    #[error("Invalid HandBrake executable at '{path}': {reason}")]
    InvalidExecutable {
        /// The path to the invalid executable.
        path: std::path::PathBuf,
        /// The reason why the executable is considered invalid.
        reason: String,
    },
    /// The `HandBrakeCLI` process could not be spawned.
    #[error("Failed to spawn HandBrake process: {source}")]
    ProcessSpawnFailed {
        /// The underlying I/O error that occurred.
        #[from]
        source: std::io::Error,
    },
    /// A control command (e.g., `cancel`, `kill`) failed.
    #[error("Failed to send {action} to HandBrake process, due to {source}")]
    ControlFailed {
        /// The control action that failed (e.g., "cancel", "kill").
        action: &'static str,
        /// The underlying I/O error that occurred.
        source: std::io::Error,
    },
    /// A placeholder for any other kind of error.
    #[error("An unknown error occurred")]
    Unknown,
}
