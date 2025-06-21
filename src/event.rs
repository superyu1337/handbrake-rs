use std::time::Duration;

/// Represents an event emitted during a HandBrake job.
#[derive(Debug)]
pub enum JobEvent {
    /// Progress update (percentage, speed, etc.)
    Progress(Progress),
    /// Log message from HandBrake stderr
    Log(Log),
    /// The job has finished successfully or with a known failure.
    Done(Result<JobSummary, JobFailure>),
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
