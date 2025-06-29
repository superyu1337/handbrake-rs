# `handbrake-rs` Implementation Checklist

This document outlines the development tasks for building the `handbrake-rs` crate. Check off items as they are completed.

## Chunk 1: Project Setup & Core Types
- [X] Initialize `cargo` project.
- [X] Add `tokio`, `thiserror`, `futures`, `regex`, and `async-stream` to `Cargo.toml`.
- [X] Create the initial module structure (`lib.rs`, `error.rs`, `job.rs`, `handle.rs`, `event.rs`).
- [X] Define the `Error` enum in `error.rs` using `thiserror`.
- [X] Define placeholder `JobEvent`, `Progress`, `Log`, and `LogLevel` types in `event.rs`.
- [X] Declare modules and re-export primary types in `lib.rs`.

## Chunk 2: Executable Discovery & Validation
- [X] Implement a private helper function `find_executable_in_path` to search the system `PATH`.
- [X] Implement the `HandBrake` struct.
- [X] Implement the `HandBrake::new()` constructor.
- [X] Implement the `HandBrake::new_with_path()` constructor.
- [X] Implement the private `validate_executable` function using `tokio::process::Command` to run `--version`.
- [X] Integrate `validate_executable` into the `new()` and `new_with_path()` constructors.
- [X] Implement the `HandBrake::version()` method to return the version string.
- [X] Add `Command` wrapper, called `MockCommand`, that is used to run `HandBrakeCLI` commands, and to simulate the output of `HandBrakeCLI` commands in tests.
- [X] Create unit tests for the discovery and validation logic. (Note: `find_executable_in_path` and `HandBrake::new()` tests are deferred due to mocking complexities.)

## Chunk 3: Job Configuration (`JobBuilder`)
- [X] Define `InputSource` and `OutputDestination` enums in `job.rs`.
- [X] Define the `JobBuilder` struct with fields for configuration options.
- [X] Implement `HandBrake::job(...)` to create and return a `JobBuilder`.
- [X] Implement the `.preset()` fluent method on `JobBuilder`.
- [X] Implement the `.video_codec()` fluent method on `JobBuilder`.
- [X] Implement other necessary configuration methods (e.g., `audio_codec`, `quality`).
- [X] Ensure the "last call wins" precedence logic for arguments is implemented.
- [X] Create unit tests to verify correct argument vector construction.

## Chunk 4: Simple Execution (`status`)
- [X] Implement the `async fn status()` method on `JobBuilder`.
- [X] Configure `tokio::process::Command` with arguments from the builder.
- [X] Implement correct `stdin`/`stdout`/`stderr` redirection logic within `status()`.
- [X] Implement a simple example of using `status()` command.
- [X] Create an `#[ignore]` integration test in the `tests/` directory for the `status()` method.

## Chunk 5: Monitored Execution (`start` and `JobHandle`)
- [X] Define the final event data structures in `event.rs`: `JobSummary` and `JobFailure`.
- [ ] Implement the `JobHandle` struct in `handle.rs` (containing `Child` and the stream handle).
- [ ] Implement the `start()` method on `JobBuilder`.
- [ ] In `start()`, create the async-stream of `JobEvent`s.
- [ ] In `start()`, spawn the `HandBrakeCLI` process with `stderr` piped.
- [ ] In `start()`, spawn the background task for reading `stderr`.
- [ ] Implement the `stderr` reader task logic.
- [ ] In `handle.rs`, implement `events()` to yield every event of the `Stream`.

## Chunk 6: Event Parsing & Streaming
- [ ] Define `Config` struct and add `JobEvent::Config` variant in `event.rs` as per the spec.
- [ ] Implement parsing logic for the initial JSON job configuration block into a `JobEvent::Config`.
- [ ] Implement parsing logic for `Progress` events from `stderr` lines using `regex`.
- [ ] Implement parsing logic for `Log` events from non-progress `stderr` lines.
- [ ] In the `stderr` reader task, send parsed `Config`, `Progress`, and `Log` events over the channel.
- [ ] In the `stderr` reader task, await the final process `ExitStatus` after the stream ends.
- [ ] In the `stderr` reader task, send the final `JobEvent::Done` event.
- [ ] Create an `#[ignore]` integration test for the `start()` method to verify event parsing.

## Chunk 7: Process Control
- [ ] Implement `JobHandle::kill()` to terminate the process.
- [ ] Implement `JobHandle::cancel()` for graceful shutdown (`SIGINT`/`CTRL_C_EVENT`).
- [ ] Use `#[cfg(unix)]` and `#[cfg(windows)]` for platform-specific cancellation logic.
- [ ] Create an `#[ignore]` integration test for `kill()`.
- [_] Create an `#[ignore]` integration test for `cancel()`.

## Chunk 8: Finalization & Documentation
- [ ] Add comprehensive `rustdoc` comments to all public APIs.
- [ ] Include `# Examples` in documentation for key functions.
- [ ] Create a high-quality `README.md` file with installation and usage examples.
- [ ] Review and polish all unit and integration tests.
- [ ] Ensure integration tests are properly marked `#[ignore]`.
- [ ] Prepare `Cargo.toml` for publishing (license, repository, etc.).
- [ ] Run `cargo publish --dry-run` to check for issues.
- [ ] Publish the crate to `crates.io`.
