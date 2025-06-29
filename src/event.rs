use serde::Deserialize;
use std::time::Duration;

/// Represents an event emitted during a HandBrake job.
#[derive(Debug)]
pub enum JobEvent {
    /// The initial job configuration, parsed from HandBrake's JSON output.
    /// This event is emitted once at the beginning of a monitored job.
    Config(Config),
        /// Progress update (percentage, speed, etc.)
    Progress(Progress),
    /// Log message from HandBrake stderr
    Log(Log),
    /// The job has finished successfully or with a known failure.
    Done(Result<JobSummary, JobFailure>),
}

/// The full job configuration as reported by `HandBrakeCLI`.
///
/// This struct captures the most relevant fields from the JSON block
/// that HandBrake prints at the start of a job, providing confirmation
/// of the settings being used for the encode.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    pub source: SourceConfig,
    pub destination: DestinationConfig,
    pub video: VideoConfig,
    #[serde(rename = "Audio")]
    pub audio_config: AudioConfig,
}

/// Details about the input source from the job configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SourceConfig {
    pub path: std::path::PathBuf,
    pub title: u32,
}

/// Details about the output destination from the job configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DestinationConfig {
    pub file: std::path::PathBuf,
    pub mux: String,
}

/// Details about the video encoding from the job configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VideoConfig {
    pub encoder: String,
    pub quality: f64,
    pub preset: String,
}

/// Details about the audio tracks from the job configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AudioConfig {
    pub audio_list: Vec<AudioTrackConfig>,
}

/// Details for a single audio track.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AudioTrackConfig {
    #[serde(rename = "PresetEncoder")]
    pub encoder_name: String,
    pub bitrate: u32,
}

/// Represents the progress of a HandBrake job.
#[derive(Debug)]
pub struct Progress {
    pub percentage: f32,
    pub fps: f32,
    pub avg_fps: f32,
    pub eta: Duration,
}

/// Represents a log message from HandBrake stderr.
#[derive(Debug)]
pub struct Log {
    pub level: LogLevel,
    pub message: String,
}

/// Represents the log level of a message.
#[derive(Debug)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

/// Summary of a completed job.
#[derive(Debug)]
pub struct JobSummary {
    pub duration: Duration,
    pub avg_fps: f32,
}

/// Details of a job failure.
#[derive(Debug, Clone)]
pub struct JobFailure {
    // Placeholder fields
    pub message: String,
    pub exit_code: Option<i32>,
}
