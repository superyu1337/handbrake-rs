use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::process::Stdio;

use tokio::process::Command;

use crate::error::Error;
pub enum InputSource {
    File(PathBuf),
    Stdin,
}

pub enum OutputDestination {
    File(PathBuf),
    Stdout,
}

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
    // Add other options here as needed
}

impl JobBuilder {
    /// Creates a new `JobBuilder` instance.
    pub fn new(handbrake_path: PathBuf, input: InputSource, output: OutputDestination) -> Self {
        JobBuilder {
            handbrake_path,
            input,
            output,
            preset: None,
            video_codec: None,
            audio_codecs: HashMap::new(),
            quality: None,
        }
    }

    /// Sets the HandBrake preset.
    /// e.g., "Fast 1080p30"
    pub fn preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = Some(preset.into());
        self
    }

    /// Overrides the video codec.
    /// e.g., "x265", "hevc", "av1"
    pub fn video_codec(mut self, codec: impl Into<String>) -> Self {
        self.video_codec = Some(codec.into());
        self
    }

    /// Overrides the audio codec for a specific track.
    /// HandBrakeCLI uses `--audio <track_id>,<encoder>`.
    /// If called multiple times for the same track, the last call wins.
    pub fn audio_codec(mut self, track: u32, codec: impl Into<String>) -> Self {
        self.audio_codecs.insert(track, codec.into());
        self
    }

    /// Sets the constant quality (RF) for video encoding.
    /// HandBrakeCLI uses `--quality <value>` or `-q <value>`.
    /// Value typically ranges from 0 to 51 (lower is better quality).
    pub fn quality(mut self, quality: f32) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Executes the job and waits for completion, returning only the final status.
    /// Ideal for "fire-and-forget" scenarios.
    ///
    /// # Errors
    /// Returns an error if the process could not be spawned.
    pub async fn status(&self) -> Result<ExitStatus, Error> {
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

        args
    }
}
