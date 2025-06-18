use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HandBrake executable not found in system PATH. Searched paths: {searched_paths:?}")]
    ExecutableNotFound {
        searched_paths: Vec<std::path::PathBuf>,
    },
    #[error("Invalid HandBrake executable at '{path}': {reason}")]
    InvalidExecutable {
        path: std::path::PathBuf,
        reason: String,
    },
    #[error("Failed to spawn HandBrake process: {source}")]
    ProcessSpawnFailed {
        source: std::io::Error,
    },
    // Placeholder for other errors
    #[error("An unknown error occurred")]
    Unknown,
}
