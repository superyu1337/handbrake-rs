use serde::Deserialize;
use std::{process::ExitStatus, time::Duration};

/// An event emitted by a monitored `HandBrakeCLI` job.
#[derive(Debug)]
pub enum JobEvent {
    /// The initial job configuration, parsed from HandBrake's JSON output.
    /// This event is emitted once at the beginning of a monitored job.
    Config(Config),
    /// A progress update, typically emitted every second during an encode.
    Progress(Progress),
    /// A log message from the `HandBrakeCLI` `stderr` stream.
    Log(Log),
    /// A raw fragment of data from the `HandBrakeCLI` `stdout` stream that is not progress information.
    /// If the job's output destination is `stdout`, this will contain the encoded video data.
    Fragment(Vec<u8>),
    /// Signals that the `HandBrakeCLI` process has terminated.
    /// Contains the final `ExitStatus` on success, or a `JobFailure` on error.
    Done(Result<ExitStatus, JobFailure>),
}

/// The full job configuration as reported by `HandBrakeCLI`.
///
/// This struct captures the most relevant fields from the JSON block
/// that HandBrake prints at the start of a job, providing confirmation
/// of the settings being used for the encode.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    /// Details about the input source.
    pub source: SourceConfig,
    /// Details about the output destination.
    pub destination: DestinationConfig,
    /// Details about the video encoding settings.
    pub video: VideoConfig,
    /// Details about the audio track configuration.
    #[serde(rename = "Audio")]
    pub audio_config: AudioConfig,
}

/// Details about the input source from the job configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SourceConfig {
    /// The path to the input file.
    pub path: std::path::PathBuf,
    /// The selected title from the source.
    pub title: u32,
}

/// Details about the output destination from the job configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DestinationConfig {
    /// The path to the output file.
    pub file: std::path::PathBuf,
    /// The container format (muxer) being used (e.g., "mp4", "mkv").
    pub mux: String,
}

/// Details about the video encoding from the job configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VideoConfig {
    /// The video codec being used (e.g., "x265", "av1").
    pub encoder: String,
    /// The quality setting for the encode.
    pub quality: f64,
    /// The name of the preset being used, if any.
    pub preset: Option<String>,
}

/// Details about the audio tracks from the job configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AudioConfig {
    /// A list of all configured audio tracks for the job.
    pub audio_list: Vec<AudioTrackConfig>,
}

/// Details for a single audio track.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AudioTrackConfig {
    /// The name of the audio codec being used (e.g., "aac", "ac3").
    #[serde(rename = "PresetEncoder")]
    pub encoder_name: String,
    /// The bitrate of the audio track.
    pub bitrate: u32,
}

/// A progress update from an ongoing `HandBrakeCLI` job.
#[derive(Debug)]
pub struct Progress {
    /// The completion percentage of the current task.
    pub percentage: f32,
    /// The instantaneous frames per second (FPS).
    pub fps: f32,
    /// The average frames per second (FPS) for the job.
    pub avg_fps: Option<f32>,
    /// The estimated time remaining until completion.
    pub eta: Option<Duration>,
}

/// A log message from the `HandBrakeCLI` process.
#[derive(Debug)]
pub struct Log {
    /// The content of the log message.
    pub message: String,
}

/// Details of a job failure.
#[derive(Debug, Clone)]
pub struct JobFailure {
    /// A message describing the failure.
    pub message: String,
    /// The exit code of the `HandBrakeCLI` process, if available.
    pub exit_code: Option<i32>,
}
