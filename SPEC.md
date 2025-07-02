## **Developer Specification: `handbrake-rs` Rust Crate**

**Version:** 1.0
**Date:** 16 June 2025

### **1. Introduction & Project Goal**

This document outlines the complete specification for `handbrake-rs`, a Rust crate designed to provide a safe, ergonomic, and robust interface for the `HandBrakeCLI` command-line video transcoding tool.

The primary goal is to allow Rust applications to programmatically start, configure, monitor, and control HandBrake encoding jobs without needing to manually handle command-line arguments or parse raw text output. The crate will feature a modern, asynchronous API suitable for integration into complex applications.

### **2. Core Functional Requirements**

* **Job Configuration**: Allow developers to define an encoding job, including input/output sources and detailed encoding parameters.
* **Process Execution**: Support two modes of execution: a simple "fire-and-forget" mode and a fully monitored mode for real-time feedback.
* **Real-time Monitoring**: In monitored mode, provide a stream of structured events for progress, logs (info, warnings, errors), and job completion.
* **Process Control**: Allow a running encoding job to be gracefully cancelled or forcefully killed.
* **Robust Error Handling**: Implement a "fail-fast" strategy and provide clear, specific errors for both setup and runtime failures.
* **Flexible Setup**: Do not require `HandBrakeCLI` to be in a hardcoded location; search the system `PATH` by default and allow developers to provide a specific path.

### **3. Architectural Design**

The crate's architecture is divided into three main components: the main `HandBrake` factory object, a `JobBuilder` for configuration, and a `JobHandle` for monitoring and control. The entire API will be `async`.

1.  **`HandBrake` (The Factory)**
    * This struct is the main entry point. Its responsibility is to locate and validate the `HandBrakeCLI` executable.
    * It acts as a factory for creating new `JobBuilder` instances.

2.  **`JobBuilder` (The Fluent Configurator)**
    * An instance of `JobBuilder` represents a single, configurable encoding job.
    * It uses the builder pattern with a fluent API (e.g., `job.preset(...).video_codec(...)`) to construct the exact command-line arguments for `HandBrakeCLI`.
    * It is immutable in the sense that each method call can return a new, updated instance of the builder.

3.  **`JobHandle` (The Controller & Monitor)**
    * This struct is returned when a job is started in monitored mode.
    * It represents a running `HandBrakeCLI` process and provides two key functionalities:
        1.  An `async` stream of events parsed from the process's `stderr`.
        2.  Control methods (`cancel`, `kill`) to manage the underlying process.

### **4. API Specification**

Below are the proposed public structs and functions.

#### **4.1. `HandBrake` Struct**

```rust
use std::path::PathBuf;

pub struct HandBrake {
    executable_path: PathBuf,
}

impl HandBrake {
    /// Creates a new HandBrake instance by searching for `HandBrakeCLI` in the system PATH.
    ///
    /// # Errors
    /// Returns an error if `HandBrakeCLI` is not found, is not executable,
    /// or is an unsupported version.
    pub async fn new() -> Result<Self, Error> {
        // ...
    }

    /// Creates a new HandBrake instance using a specific path to `HandBrakeCLI`.
    ///
    /// # Errors
    /// Returns an error if the file at the path does not exist, is not executable,
    /// or is an unsupported version.
    pub async fn new_with_path(path: impl Into<PathBuf>) -> Result<Self, Error> {
        // ...
    }

    /// Creates a new job builder.
    pub fn job(&self, input: InputSource, output: OutputDestination) -> JobBuilder {
        // ...
    }
}
```

#### **4.2. `JobBuilder` Struct & I/O Types**

```rust
use std::process::ExitStatus;

pub enum InputSource {
    File(PathBuf),
    Stdin,
}

pub enum OutputDestination {
    File(PathBuf),
    Stdout,
}

pub struct JobBuilder {
    // internal state for arguments
}

impl JobBuilder {
    /// Sets the HandBrake preset.
    /// e.g., "Fast 1080p30"
    pub fn preset(self, preset: impl Into<String>) -> Self { /* ... */ }

    /// Overrides the video codec.
    /// e.g., "x265", "hevc", "av1"
    pub fn video_codec(self, codec: impl Into<String>) -> Self { /* ... */ }

    /// Overrides the audio codec for a specific track.
    pub fn audio_codec(self, track: u32, codec: impl Into<String>) -> Self { /* ... */ }

    /// Sets the constant quality (RF) for video encoding.
    /// Value typically ranges from 0 to 51 (lower is better quality).
    pub fn quality(self, quality: f32) -> Self { /* ... */ }

    /// Sets the output container format.
    /// e.g., "mp4", "mkv"
    pub fn format(self, format: impl Into<String>) -> Self { /* ... */ }

    // ... other fluent methods for filters, etc.

    /// Executes the job and waits for completion, returning only the final status.
    /// Ideal for "fire-and-forget" scenarios.
    ///
    /// # Errors
    /// Returns an error if the process could not be spawned.
    pub async fn status(self) -> Result<ExitStatus, Error> { /* ... */ }

    /// Starts the job in monitored mode.
    /// Returns a `JobHandle` to monitor and control the running process.
    ///
    /// # Errors
    /// Returns an error if the process could not be spawned.
    pub fn start(self) -> Result<JobHandle, Error> { /* ... */ }
}
```

#### **4.3. `JobHandle` Struct & Event Stream**

```rust
use futures::stream::Stream;
use std::time::Duration;

pub struct JobHandle {
    // handle to the child process
    // receiver for the event stream
}

impl JobHandle {
    /// Attempts to gracefully shut down the HandBrake process.
    /// (Sends SIGINT on Unix, CTRL_C_EVENT on Windows).
    pub async fn cancel(&self) -> Result<(), Error> { /* ... */ }

    /// Forcefully terminates the HandBrake process immediately.
    /// (Sends SIGKILL on Unix, TerminateProcess on Windows).
    pub async fn kill(&self) -> Result<(), Error> { /* ... */ }

    /// Returns an async stream of events from the running job.
    pub fn events(&mut self) -> impl Stream<Item = JobEvent> { /* ... */ }
}

// --- Event Data Structures ---

#[derive(Debug)]
pub enum JobEvent {
    /// Raw, unparsed data from the output stream, typically from `stdout`.
    /// This can contain interleaved video data if streaming to `stdout`.
    Fragment(Vec<u8>),
    /// A progress update from the encoding process.
    Progress(Progress),
    /// A log message from `HandBrakeCLI`.
    Log(Log),
    /// Signals that the `HandBrakeCLI` process has finished.
    Done,
}

#[derive(Debug)]
pub struct Progress {
    pub percentage: f32,
    pub fps: f32,
    pub avg_fps: Option<f32>,
    pub eta: Option<Duration>,
}

#[derive(Debug)]
pub struct Log {
    pub message: String,
}
```

### **5. Data Handling & Output Parsing**

The implementation must correctly handle the `stdout` and `stderr` streams from the `HandBrakeCLI` child process.

*   **`stdout`**: `HandBrakeCLI` writes progress information and, if `OutputDestination` is `Stdout`, raw video data to this stream.
*   **`stderr`**: `HandBrakeCLI` writes all log messages to this stream.
*   **Parsing Logic**: A dedicated asynchronous task must read both `stdout` and `stderr`.
    *   **Progress Lines**: `stdout` is read in chunks, looking for lines matching a pattern like `Encoding: task ..., XX.XX %, ...`. These are parsed into a `Progress` struct.
    *   **Log Lines**: Each line from `stderr` is treated as a `Log` event.
    *   **Raw Data**: Any data from `stdout` that is not a progress line is emitted as a `Fragment` event. This is important for streaming scenarios.
    *   **JSON Job**: The specification requires that `HandBrakeCLI` will print a multiline JSON string with the entire job configuration. This should be parsed into a `JobConfig` struct. (Note: This is a future requirement not yet fully implemented).
    *   **Completion**: When `HandBrakeCLI` prints that it has exited, a final `Done` event must be sent.

### **6. Error Handling Strategy**

A robust error handling strategy is crucial for the crate's usability.

1.  **Fail-Fast Initialization**: As specified, `HandBrake::new()` will fail immediately with a descriptive `Error` if `HandBrakeCLI` is not found or is invalid. This prevents any further configuration attempts on an invalid setup.
2.  **Specific Error Types**: Use a dedicated `Error` enum for the crate, leveraging a library like `thiserror` for clean implementation. Example variants:
    * `ExecutableNotFound { searched_paths: Vec<PathBuf> }`
    * `InvalidExecutable { path: PathBuf, reason: String }`
    * `UnsupportedVersion { version_string: String }`
    * `ProcessSpawnFailed { source: std::io::Error }`
    * `ControlFailed { action: &'static str, source: std::io::Error }`
3.  **Runtime Errors**:
    * Fatal runtime errors from `HandBrakeCLI` (e.g., "Source not found", "Invalid preset") will be captured as `Log` events with `LogLevel::Error`.
    * When the process terminates with a non-zero exit code, the `Done` event will contain `Err(JobFailure)`, providing the exit code and any final error messages captured from the logs.

### **7. Testing Plan**

A multi-layered testing approach is required to ensure correctness and robustness.

1.  **Unit Tests**:
    * Test the `stderr` parsing logic in isolation. Create static string samples of `HandBrakeCLI` output (progress lines, warnings, errors, completion summaries) and assert that the parser correctly transforms them into the appropriate `JobEvent` structs.
    * Test individual argument-building logic within the `JobBuilder`.

2.  **Integration Tests**:
    * These tests will run against a real `HandBrakeCLI` executable and require small, short sample video files. They should be marked as `#[ignore]` by default so they don't run in standard `cargo test` workflows unless explicitly requested (`cargo test -- --ignored`).
    * **Success Case**: Test a full, successful encode from a file to a file. Verify that the output file is created and that the event stream contains `Progress` events and a final `Done(Ok(...))` event.
    * **Failure Case**: Test a job that is designed to fail (e.g., pointing to a non-existent input file). Verify that the event stream emits a `Done(Err(...))` event with the correct failure details.
    * **I/O Streaming**: Test `stdin` to `stdout` piping.
    * **Control Case**: Start a long-running encode and immediately call `cancel()`. Verify that the process terminates and the `Done` event reflects the cancellation. Do the same for `kill()`.

3.  **Mocking**:
    * For CI/CD environments where `HandBrakeCLI` might not be installed, the integration tests should be skipped. Alternatively, a use special class implementation `Command` class that can emulate `HandBrakeCLI` behavior and be used in tests, based on the `stderr` output and exit codes of `HandBrakeCLI` based on its input arguments, allowing for more controlled testing of the crate's interaction logic.
