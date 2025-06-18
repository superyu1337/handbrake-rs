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
    // Placeholder fields
    pub percent_complete: f32,
    pub fps: f32,
    pub avg_fps: f32,
    pub eta: String, // Or a more structured time type
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
    Debug, // If we decide to parse debug logs
    Unknown,
}

/// Summary of a completed job.
#[derive(Debug)]
pub struct JobSummary {
    // Placeholder fields
    pub duration: String, // Or a more structured time type
    pub output_file: String,
}

/// Details of a job failure.
#[derive(Debug)]
pub struct JobFailure {
    // Placeholder fields
    pub message: String,
    pub exit_code: Option<i32>,
}
