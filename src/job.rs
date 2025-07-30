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

fn parse_caps<T>(caps: &Captures, name: &str) -> Option<T>
where
    T: Default + FromStr,
{
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

impl From<&str> for InputSource {
    fn from(p: &str) -> Self {
        InputSource::File(p.into())
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

impl From<&str> for OutputDestination {
    fn from(p: &str) -> Self {
        OutputDestination::File(p.into())
    }
}

/// Represents the subtitle selection mode.
pub enum SubtitleSelection {
    /// Select specific subtitle tracks by their index.
    Tracks(Vec<u32>),
    /// Scan for foreign audio subtitles.
    Scan,
}

/// Represents the subtitle burn-in mode as per user request.
pub enum SubtitleBurnMode {
    /// Burn subtitles from foreign language audio tracks marked as "forced".
    Native,
    /// Disable burning of subtitles.
    None,
}

/// Represents the default subtitle track selection.
pub enum SubtitleDefaultMode {
    /// Set a specific track as the default.
    Track(u32),
    /// Flag no subtitle track as default.
    None,
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
    width: Option<u32>,
    height: Option<u32>,
    // Maps track number to codec string. Allows overriding specific tracks.
    audio_codecs: HashMap<u32, String>,
    quality: Option<f32>,
    format: Option<String>,
    subtitle_selection: Option<SubtitleSelection>,
    subtitle_langs: Vec<String>,
    subtitle_burned: Option<SubtitleBurnMode>,
    subtitle_forced: Option<u32>,
    subtitle_default: Option<SubtitleDefaultMode>,
    srt_file: Option<String>,
    ssa_file: Option<String>,
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
            width: None,
            height: None,
            audio_codecs: HashMap::new(),
            quality: None,
            format: None,
            subtitle_selection: None,
            subtitle_langs: Vec::new(),
            subtitle_burned: None,
            subtitle_forced: None,
            subtitle_default: None,
            srt_file: None,
            ssa_file: None,
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

    /// Adds a subtitle track to the job.
    ///
    /// This can be called multiple times to include multiple subtitle tracks.
    /// This will override a previous call to `subtitle_scan()`.
    pub fn subtitle(mut self, track: u32) -> Self {
        let tracks = match self.subtitle_selection {
            Some(SubtitleSelection::Tracks(mut existing_tracks)) => {
                existing_tracks.push(track);
                existing_tracks
            }
            _ => vec![track],
        };
        self.subtitle_selection = Some(SubtitleSelection::Tracks(tracks));
        self
    }

    /// Enables foreign audio scan for subtitles.
    ///
    /// This will override any previous calls to `subtitle()`.
    pub fn subtitle_scan(mut self) -> Self {
        self.subtitle_selection = Some(SubtitleSelection::Scan);
        self
    }

    /// Adds a subtitle language to select tracks by.
    ///
    /// Can be called multiple times. e.g., `"eng"`, `"fre"`.
    /// Uses the `iso639-2` code.
    pub fn subtitle_lang(mut self, lang: impl Into<String>) -> Self {
        self.subtitle_langs.push(lang.into());
        self
    }

    /// Sets the subtitle burn-in mode.
    pub fn subtitle_burned(mut self, mode: SubtitleBurnMode) -> Self {
        self.subtitle_burned = Some(mode);
        self
    }

    /// Force display of subtitles from the specified track only if the "forced" flag is set.
    pub fn subtitle_forced(mut self, track: u32) -> Self {
        self.subtitle_forced = Some(track);
        self
    }

    /// Sets the default subtitle track.
    pub fn subtitle_default(mut self, mode: SubtitleDefaultMode) -> Self {
        self.subtitle_default = Some(mode);
        self
    }

    /// Imports subtitles from an external SRT file.
    ///
    /// The `file` string can include comma-separated srt files.
    pub fn srt_file(mut self, file: impl Into<String>) -> Self {
        self.srt_file = Some(file.into());
        self
    }

    /// Imports subtitles from an external SSA file.
    ///
    /// The `file` string can include comma-separated ssa files.
    pub fn ssa_file(mut self, file: impl Into<String>) -> Self {
        self.ssa_file = Some(file.into());
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

    pub fn width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: u32) -> Self {
        self.height = Some(height);
        self
    }

    fn create_process(self) -> Result<Command, Error> {
        let args = self.build_args();

        let stdin_cfg = match self.input {
            InputSource::Stdin => Stdio::piped(),
            _ => Stdio::inherit(), // Default to inheriting stdin
        };

        let stdout_cfg = match self.output {
            OutputDestination::Stdout => Stdio::piped(),
            _ => Stdio::inherit(), // Default to inheriting stdout
        };

        let mut cmd = Command::new(&self.handbrake_path);
        cmd.args(args).stdin(stdin_cfg).stdout(stdout_cfg);
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;
            cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }
        Ok(cmd)
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
        // For status, we don't need to capture stderr, just let it go to parent process's stderr
        let stderr_cfg = Stdio::inherit();
        self.create_process()?
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
        let mut child = self
            .create_process()?
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
        if let Some(w) = &self.width {
            args.extend(["--width".into(), w.to_string()]);   
        }
        if let Some(h) = &self.height {
            args.extend(["--height".into(), h.to_string()]);
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

        if let Some(selection) = &self.subtitle_selection {
            let value = match selection {
                SubtitleSelection::Tracks(tracks) => tracks
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
                SubtitleSelection::Scan => "scan".to_string(),
            };
            args.extend(["--subtitle".into(), value]);
        }

        if !self.subtitle_langs.is_empty() {
            args.extend(["--subtitle-lang-list".into(), self.subtitle_langs.join(",")]);
        }

        if let Some(mode) = &self.subtitle_burned {
            let value = match mode {
                SubtitleBurnMode::Native => "native".to_string(),
                SubtitleBurnMode::None => "none".to_string(),
            };
            args.extend(["--subtitle-burned".into(), value]);
        }

        if let Some(track) = &self.subtitle_forced {
            args.extend(["--subtitle-forced".into(), track.to_string()]);
        }

        if let Some(mode) = &self.subtitle_default {
            let value = match mode {
                SubtitleDefaultMode::Track(t) => t.to_string(),
                SubtitleDefaultMode::None => "none".to_string(),
            };
            args.extend(["--subtitle-default".into(), value]);
        }

        if let Some(srt_file) = &self.srt_file {
            args.extend(["--srt-file".into(), srt_file.clone()]);
        }

        if let Some(ssa_file) = &self.ssa_file {
            args.extend(["--ssa-file".into(), ssa_file.clone()]);
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use crate::job::PROGRESS_RE;

    #[test]
    fn test_progress_re_full_match() {
        let line = "Encoding: task 1 of 1, 12.34 % (120.00 fps, avg 110.00 fps, ETA 00h01m30s)";
        let caps = PROGRESS_RE.captures(line.as_bytes()).unwrap();

        assert_eq!(&caps["pct"], b"12.34");
        assert_eq!(&caps["fps"], b"120.00");
        assert_eq!(&caps["avg_fps"], b"110.00");
        assert_eq!(&caps["eta"], b"00h01m30s");
    }

    #[test]
    fn test_progress_re_pct_only() {
        let line = "Encoding: task 1 of 1, 56.78 %";
        let caps = PROGRESS_RE.captures(line.as_bytes()).unwrap();

        assert_eq!(&caps["pct"], b"56.78");
        assert!(caps.name("fps").is_none());
        assert!(caps.name("avg_fps").is_none());
        assert!(caps.name("eta").is_none());
    }

    #[test]
    fn test_progress_re_no_match() {
        let line = "Some other output that does not match";
        assert!(PROGRESS_RE.captures(line.as_bytes()).is_none());
    }

    #[test]
    fn test_progress_re_another_full_match() {
        let line = "Encoding: task 2 of 5, 99.99 % (30.00 fps, avg 25.50 fps, ETA 01h23m45s)";
        let caps = PROGRESS_RE.captures(line.as_bytes()).unwrap();

        assert_eq!(&caps["pct"], b"99.99");
        assert_eq!(&caps["fps"], b"30.00");
        assert_eq!(&caps["avg_fps"], b"25.50");
        assert_eq!(&caps["eta"], b"01h23m45s");
    }
}
