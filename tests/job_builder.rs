use handbrake::{job::{SubtitleBurnMode, SubtitleDefaultMode}, InputSource, JobBuilder, OutputDestination};
use std::path::PathBuf;

#[test]
fn test_job_builder_basic_args_file_to_file() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("/path/to/input.mkv"));
    let output = OutputDestination::File(PathBuf::from("/path/to/output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output);
    let args = builder.build_args();

    assert_eq!(
        args,
        vec!["-i", "/path/to/input.mkv", "-o", "/path/to/output.mp4",]
    );
}

#[test]
fn test_job_builder_basic_args_stdin_to_stdout() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::Stdin;
    let output = OutputDestination::Stdout;

    let builder = JobBuilder::new(handbrake_path, input, output);
    let args = builder.build_args();

    assert_eq!(args, vec!["-i", "pipe:0", "-o", "pipe:1",]);
}

#[test]
fn test_job_builder_with_preset() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output).preset("Fast 1080p30");
    let args = builder.build_args();

    assert_eq!(
        args,
        vec![
            "-i",
            "input.mkv",
            "-o",
            "output.mp4",
            "--preset",
            "Fast 1080p30",
        ]
    );
}

#[test]
fn test_job_builder_with_video_codec() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output).video_codec("x265");
    let args = builder.build_args();

    assert_eq!(
        args,
        vec!["-i", "input.mkv", "-o", "output.mp4", "--encoder", "x265",]
    );
}

#[test]
fn test_job_builder_with_quality() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output).quality(20.5);
    let args = builder.build_args();

    assert_eq!(
        args,
        vec!["-i", "input.mkv", "-o", "output.mp4", "--quality", "20.5",]
    );
}

#[test]
fn test_job_builder_with_single_audio_codec() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output).audio_codec(1, "aac");
    let args = builder.build_args();

    assert_eq!(
        args,
        vec!["-i", "input.mkv", "-o", "output.mp4", "--audio", "1,aac",]
    );
}

#[test]
fn test_job_builder_with_multiple_audio_codecs() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output)
        .audio_codec(2, "ac3")
        .audio_codec(1, "mp3"); // Order should be sorted by track number

    let args = builder.build_args();

    assert_eq!(
        args,
        vec![
            "-i",
            "input.mkv",
            "-o",
            "output.mp4",
            "--audio",
            "1,mp3",
            "--audio",
            "2,ac3",
        ]
    );
}

#[test]
fn test_job_builder_last_call_wins_preset() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output)
        .preset("Old Preset")
        .preset("New Preset"); // New preset should override

    let args = builder.build_args();

    assert_eq!(
        args,
        vec![
            "-i",
            "input.mkv",
            "-o",
            "output.mp4",
            "--preset",
            "New Preset",
        ]
    );
}

#[test]
fn test_job_builder_last_call_wins_video_codec() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output)
        .video_codec("x264")
        .video_codec("vp9"); // VP9 should override

    let args = builder.build_args();

    assert_eq!(
        args,
        vec!["-i", "input.mkv", "-o", "output.mp4", "--encoder", "vp9",]
    );
}

#[test]
fn test_job_builder_last_call_wins_quality() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output)
        .quality(25.0)
        .quality(18.0); // 18.0 should override

    let args = builder.build_args();

    assert_eq!(
        args,
        vec![
            "-i",
            "input.mkv",
            "-o",
            "output.mp4",
            "--quality",
            "18", // Floats are formatted without trailing .0 if whole number
        ]
    );
}

#[test]
fn test_job_builder_last_call_wins_audio_codec_same_track() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output)
        .audio_codec(1, "aac")
        .audio_codec(1, "opus"); // Opus should override for track 1

    let args = builder.build_args();

    assert_eq!(
        args,
        vec!["-i", "input.mkv", "-o", "output.mp4", "--audio", "1,opus",]
    );
}

#[test]
fn test_job_builder_last_call_wins_quality_float() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output)
        .quality(25.0)
        .quality(18.5); // 18.5 should override

    let args = builder.build_args();

    assert_eq!(
        args,
        vec![
            "-i",
            "input.mkv",
            "-o",
            "output.mp4",
            "--quality",
            "18.5",
        ]
    );
}

#[test]
fn test_job_builder_path_with_spaces() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("/path with spaces/to/input.mkv"));
    let output = OutputDestination::File(PathBuf::from("/path with spaces/to/output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output);
    let args = builder.build_args();

    assert_eq!(
        args,
        vec![
            "-i",
            "/path with spaces/to/input.mkv",
            "-o",
            "/path with spaces/to/output.mp4",
        ]
    );
}

#[test]
fn test_job_builder_combined_options() {
    let handbrake_path = PathBuf::from("/usr/bin/HandBrakeCLI");
    let input = InputSource::File(PathBuf::from("input.mkv"));
    let output = OutputDestination::File(PathBuf::from("output.mp4"));

    let builder = JobBuilder::new(handbrake_path, input, output)
        .preset("Web Optimized")
        .video_codec("h264")
        .quality(22.0)
        .audio_codec(1, "aac")
        .audio_codec(2, "ac3");

    let args = builder.build_args();

    // Note: Order of audio codecs is sorted by track number
    assert_eq!(
        args,
        vec![
            "-i",
            "input.mkv",
            "-o",
            "output.mp4",
            "--preset",
            "Web Optimized",
            "--encoder",
            "h264",
            "--audio",
            "1,aac",
            "--audio",
            "2,ac3",
            "--quality",
            "22",
        ]
    );
}

#[test]
fn test_subtitle_selection_single_track() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle(1);
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle", "1"]
    );
}

#[test]
fn test_subtitle_selection_multiple_tracks() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle(1)
    .subtitle(3);
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle", "1,3"]
    );
}

#[test]
fn test_subtitle_selection_scan() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle_scan();
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle", "scan"]
    );
}

#[test]
fn test_subtitle_selection_last_call_wins_track_over_scan() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle_scan()
    .subtitle(1);
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle", "1"]
    );
}

#[test]
fn test_subtitle_selection_last_call_wins_scan_over_track() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle(1)
    .subtitle_scan();
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle", "scan"]
    );
}

#[test]
fn test_subtitle_lang_list() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle_lang("eng")
    .subtitle_lang("fre");
    assert_eq!(
        builder.build_args(),
        vec![
            "-i",
            "in.mkv",
            "-o",
            "out.mp4",
            "--subtitle-lang-list",
            "eng,fre"
        ]
    );
}

#[test]
fn test_subtitle_burned() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle_burned(SubtitleBurnMode::Native);
    assert_eq!(
        builder.build_args(),
        vec![
            "-i",
            "in.mkv",
            "-o",
            "out.mp4",
            "--subtitle-burned",
            "native"
        ]
    );
}

#[test]
fn test_subtitle_burned_none() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle_burned(SubtitleBurnMode::None);
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle-burned", "none"]
    );
}

#[test]
fn test_subtitle_forced() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle_forced(2);
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle-forced", "2"]
    );
}

#[test]
fn test_subtitle_default() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle_default(SubtitleDefaultMode::Track(1));
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle-default", "1"]
    );
}

#[test]
fn test_subtitle_default_none() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle_default(SubtitleDefaultMode::None);
    assert_eq!(
        builder.build_args(),
        vec!["-i", "in.mkv", "-o", "out.mp4", "--subtitle-default", "none"]
    );
}

#[test]
fn test_srt_files() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .srt_file("sub1.srt,sub2.srt");
    assert_eq!(
        builder.build_args(),
        vec![
            "-i",
            "in.mkv",
            "-o",
            "out.mp4",
            "--srt-file",
            "sub1.srt,sub2.srt",
        ]
    );
}

#[test]
fn test_ssa_files() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .ssa_file("sub1.ass,sub2.ass");
    assert_eq!(
        builder.build_args(),
        vec![
            "-i",
            "in.mkv",
            "-o",
            "out.mp4",
            "--ssa-file",
            "sub1.ass,sub2.ass",
        ]
    );
}

#[test]
fn test_all_subtitle_options() {
    let builder = JobBuilder::new(
        "hb".into(),
        "in.mkv".into(),
        "out.mp4".into(),
    )
    .subtitle(1)
    .subtitle_lang("eng")
    .subtitle_burned(SubtitleBurnMode::Native)
    .subtitle_forced(1)
    .subtitle_default(SubtitleDefaultMode::Track(1))
    .srt_file("subs.srt")
    .ssa_file("subs.ass");

    assert_eq!(
        builder.build_args(),
        vec![
            "-i",
            "in.mkv",
            "-o",
            "out.mp4",
            "--subtitle",
            "1",
            "--subtitle-lang-list",
            "eng",
            "--subtitle-burned",
            "native",
            "--subtitle-forced",
            "1",
            "--subtitle-default",
            "1",
            "--srt-file",
            "subs.srt",
            "--ssa-file",
            "subs.ass"
        ]
    );
}
