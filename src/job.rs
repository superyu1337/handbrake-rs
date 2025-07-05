use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use futures::io;
use once_cell::sync::Lazy;
use regex::bytes::Captures;
use regex::bytes::Regex;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::select;
use tokio::sync::{Mutex, mpsc};
use tokio_util::codec::FramedRead;
use tokio_util::codec::LinesCodec;

use crate::error::Error;
use crate::event::{JobEvent, Log};
use crate::handle::JobHandle;

static PROGRESS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"Encoding: task \d+ of \d+, (?P<pct>\d{1,2}\.\d{2}) %( \((?P<fps>\d+\.\d{2}) fps, avg (?P<avg_fps>\d+\.\d{2}) fps, ETA (?P<eta>\d{2}h\d{2}m\d{2}s)\))?",
    )
    .expect("BUG: Failed to compile progress regex")
});

/// Parses HandBrake's `HHhMMmSSs` ETA format into a `Duration`.
fn parse_eta(eta_str: &str) -> Duration {
    let h_str = &eta_str[0..2];
    let m_str = &eta_str[3..5];
    let s_str = &eta_str[6..8];

    let h = h_str.parse::<u64>().unwrap_or(0);
    let m = m_str.parse::<u64>().unwrap_or(0);
    let s = s_str.parse::<u64>().unwrap_or(0);

    Duration::from_secs(h * 3600 + m * 60 + s)
}

fn parse_caps<T>(caps: &Captures, name: &str) -> Option<T> where T: Default + FromStr {
    if let Some(v) = caps.name(name) {
        Some(
            String::from_utf8_lossy(v.as_bytes())
                .parse::<T>()
                .unwrap_or_default(),
        )
    } else {
        None
    }
}

/// Represents the input source for a `HandBrakeCLI` job.
pub enum InputSource {
    /// Use a file as the input source.
    File(PathBuf),
    /// Use `stdin` as the input source.
    Stdin,
}

impl From<PathBuf> for InputSource {
    fn from(p: PathBuf) -> Self {
        InputSource::File(p)
    }
}

/// Represents the output destination for a `HandBrakeCLI` job.
pub enum OutputDestination {
    /// Write the output to a file.
    File(PathBuf),
    /// Write the output to `stdout`.
    Stdout,
}

impl From<PathBuf> for OutputDestination {
    fn from(p: PathBuf) -> Self {
        OutputDestination::File(p)
    }
}

/// A fluent builder for configuring a `HandBrakeCLI` encoding job.
pub struct JobBuilder {
    // The path to the HandBrakeCLI executable, copied from HandBrake instance
    handbrake_path: PathBuf,
    // The input source for the job
    input: InputSource,
    // The output destination for the job
    output: OutputDestination,

    // Configuration options, stored to ensure "last call wins"
    preset: Option<String>,
    video_codec: Option<String>,
    // Maps track number to codec string. Allows overriding specific tracks.
    audio_codecs: HashMap<u32, String>,
    quality: Option<f32>,
    format: Option<String>,
}

impl JobBuilder {
    /// Creates a new `JobBuilder` instance.
    ///
    /// This is typically called via `HandBrake::job()`.
    pub fn new(handbrake_path: PathBuf, input: InputSource, output: OutputDestination) -> Self {
        JobBuilder {
            handbrake_path,
            input,
            output,
            preset: None,
            video_codec: None,
            audio_codecs: HashMap::new(),
            quality: None,
            format: None,
        }
    }

    /// Sets the `HandBrakeCLI` preset.
    ///
    /// e.g., `"Fast 1080p30"`
    pub fn preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = Some(preset.into());
        self
    }

    /// Overrides the video codec.
    ///
    /// e.g., `"x265"`, `"hevc"`, `"av1"`
    pub fn video_codec(mut self, codec: impl Into<String>) -> Self {
        self.video_codec = Some(codec.into());
        self
    }

    /// Sets the output container format.
    ///
    /// e.g., `"mp4"`, `"mkv"`
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Overrides the audio codec for a specific track.
    ///
    /// `HandBrakeCLI` uses `--audio <track_id>,<encoder>`.
    /// If called multiple times for the same track, the last call wins.
    pub fn audio_codec(mut self, track: u32, codec: impl Into<String>) -> Self {
        self.audio_codecs.insert(track, codec.into());
        self
    }

    /// Sets the constant quality (RF) for video encoding.
    ///
    /// `HandBrakeCLI` uses `--quality <value>` or `-q <value>`.
    /// Value typically ranges from 0 to 51 (lower is better quality).
    pub fn quality(mut self, quality: f32) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Executes the job and waits for completion, returning only the final `ExitStatus`.
    ///
    /// This is ideal for "fire-and-forget" scenarios where real-time monitoring is not needed.
    /// The `stdout` and `stderr` of the child process are inherited by the parent.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the process could not be spawned.
    pub async fn status(self) -> Result<ExitStatus, Error> {
        let args = self.build_args();

        let stdin_cfg = match self.input {
            InputSource::Stdin => Stdio::piped(),
            _ => Stdio::inherit(), // Default to inheriting stdin
        };

        let stdout_cfg = match self.output {
            OutputDestination::Stdout => Stdio::piped(),
            _ => Stdio::inherit(), // Default to inheriting stdout
        };

        // For status, we don't need to capture stderr, just let it go to parent process's stderr
        let stderr_cfg = Stdio::inherit();

        Command::new(&self.handbrake_path)
            .args(args)
            .stdin(stdin_cfg)
            .stdout(stdout_cfg)
            .stderr(stderr_cfg)
            .status()
            .await
            .map_err(|e| Error::ProcessSpawnFailed { source: e })
    }

    /// Starts the job in monitored mode, returning a `JobHandle`.
    ///
    /// This method spawns the `HandBrakeCLI` process and a background task to parse its
    /// `stdout` and `stderr` streams into a series of `JobEvent`s.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the process could not be spawned.
    pub fn start(self) -> Result<JobHandle, Error> {
        let args = self.build_args();

        let stdin_cfg = match self.input {
            InputSource::Stdin => Stdio::piped(),
            _ => Stdio::null(),
        };

        let mut child = Command::new(&self.handbrake_path)
            .args(args)
            .stdin(stdin_cfg)
            .stdout(Stdio::piped()) // always capture stdout
            .stderr(Stdio::piped()) // Must pipe stderr for monitoring
            .spawn()
            .map_err(|e| Error::ProcessSpawnFailed { source: e })?;

        // Channel for sending events from the background task to the main handle.
        let (event_tx, event_rx) = mpsc::channel(128);

        // We must take ownership of stderr to read from it.
        let stderr = child
            .stderr
            .take()
            .expect("BUG: stderr was not captured. This should not happen when piping.");

        let stdout = child.stdout.take().expect("BUG: stdout was not captured.");

        let child = Arc::new(Mutex::new(child));
        let waiter = Arc::clone(&child);

        // Spawn a background task to read from stderr and stdout and parse events.
        tokio::spawn(async move {
            let mut stdout_reader = BufReader::new(stdout);
            let mut stderr_reader = FramedRead::new(stderr, LinesCodec::default());

            // State for parsing the JSON block
            let mut job_config_buffer = String::new();
            let mut in_json_block = false;

            #[derive(PartialEq)]
            enum EventStreamState {
                Active,
                Eof,
            }

            let mut event_parsing_state = EventStreamState::Active;

            while event_parsing_state == EventStreamState::Active {
                let mut out_buf: Vec<u8> = Vec::new();
                let line = select! {
                    read_status = stdout_reader.read_until(b'\r', &mut out_buf) => {
                        // propagate the error
                        if let Ok(bytes_read) = read_status {
                            if bytes_read == 0 {
                                event_parsing_state = EventStreamState::Eof;
                            }
                        }
                        Ok(match PROGRESS_RE.captures(&out_buf) {
                            Some(caps) => {
                                let event = JobEvent::Progress(crate::Progress {
                                    percentage: parse_caps(&caps, "pct").unwrap_or_default(),
                                    fps: parse_caps(&caps, "fps").unwrap_or_default(),
                                    avg_fps: parse_caps(&caps, "avg_fps"),
                                    eta: if let Some(v) = caps.name("eta") {
                                        Some(parse_eta(&String::from_utf8_lossy(v.as_bytes())))
                                    } else {
                                        None
                                    },
                                });
                                // remove all occurrences of the progress
                                out_buf = PROGRESS_RE.replace_all(&out_buf, b"").into();

                                event
                            },
                            None => JobEvent::Fragment(out_buf.to_vec()),
                        })
                    },
                    line = stderr_reader.next() => match line {
                        Some(Ok(v)) => {
                            if v.ends_with("json job:") {
                                in_json_block = true;
                                continue; // Continue to next iteration to buffer more lines
                            }

                            if in_json_block {
                                job_config_buffer.push_str(&v);
                                job_config_buffer.push('\n');
                                if v == "}" {
                                    in_json_block = false;
                                    match serde_json::from_str::<crate::event::Config>(&job_config_buffer) {
                                        Ok(config) => Ok(JobEvent::Config(config)),
                                        Err(e) => Ok(JobEvent::Log(Log { message: format!("JSON Parse Error: {}, \n{}", e, job_config_buffer) })),
                                    }
                                } else {
                                    continue; // Continue buffering
                                }
                            } else {
                                Ok(JobEvent::Log(Log { message: v }))
                            }
                        },
                        Some(Err(e)) => Err(std::io::Error::new(io::ErrorKind::InvalidData, e)),
                        None => continue,
                    },
                };

                match line {
                    Ok(event) => {
                        let _ = event_tx.send(event).await;
                        // send the trailing/preceding output buffer
                        if out_buf.len() > 0 {
                            let _ = event_tx.send(JobEvent::Fragment(out_buf.to_vec())).await;
                        }
                    }
                    Err(e) => {
                        let _ = event_tx
                            .send(JobEvent::Log(Log {
                                message: format!("Failed to read the line: {:?}", e).to_string(),
                            }))
                            .await;
                    }
                };
            }
            match waiter.lock().await.wait().await {
                Ok(status) => event_tx.send(JobEvent::Done(Ok(status))).await,
                Err(e) => {
                    event_tx
                        .send(JobEvent::Done(Err(crate::JobFailure {
                            message: format!("Failed: {}", e),
                            exit_code: e.raw_os_error(),
                        })))
                        .await
                }
            }
        });

        Ok(JobHandle { child, event_rx })
    }

    /// Builds the final list of command-line arguments based on the configured options.
    pub fn build_args(&self) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();

        // Input argument
        match &self.input {
            InputSource::File(path) => args.extend(["-i".into(), path.display().to_string()]),
            InputSource::Stdin => args.extend(["-i".into(), "pipe:0".into()]),
        }

        // Output argument
        match &self.output {
            OutputDestination::File(path) => {
                args.extend(["-o".to_string(), path.display().to_string()])
            }
            OutputDestination::Stdout => args.extend(["-o".into(), "pipe:1".into()]),
        }

        // Optional arguments
        if let Some(p) = &self.preset {
            args.extend(["--preset".into(), p.clone()]);
        }
        if let Some(vc) = &self.video_codec {
            args.extend(["--encoder".into(), vc.clone()]);
        }
        // Audio codecs
        // Sort by track number for consistent argument order, though not strictly necessary for HBCLI
        let mut sorted_audio_codecs: Vec<(&u32, &String)> = self.audio_codecs.iter().collect();
        sorted_audio_codecs.sort_by_key(|&(track, _)| track);
        for (track, codec) in sorted_audio_codecs {
            args.extend(["--audio".into(), format!("{},{}", track, codec)]);
        }
        if let Some(q) = &self.quality {
            args.extend(["--quality".into(), q.to_string()]);
        }
        if let Some(f) = &self.format {
            args.extend(["--format".into(), f.to_string()]);
        }

        args
    }
}
