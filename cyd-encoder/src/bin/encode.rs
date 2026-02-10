use std::{error::Error, process::Command};

const MAX_WIDTH: u32 = 320;
const MAX_HEIGHT: u32 = 240;

#[derive(argh::FromArgs)]
/// Encode video into format with custom header
struct Args {
    #[argh(option, default = "\"mjpeg\".to_string()")]
    /// video format (mjpeg, rgb or yuv)
    format: String,
    #[argh(option, default = "15u8")]
    /// frames per second
    fps: u8,
    #[argh(option)]
    /// path to subtitles srt/vtt file
    subtitles: Option<String>,
    #[argh(positional)]
    input: String,
    #[argh(positional)]
    output: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    match args.format.as_str() {
        "mjpeg" => encode_mjpeg(args),
        "yuv" => encode_yuv(args),
        "rgb" => encode_rgb(args),
        _ => Err("invalid format".into()),
    }
}

fn encode_mjpeg(args: Args) -> Result<(), Box<dyn Error>> {
    let mut filter = format!(
        "framerate={},scale=size={}x{}:force_original_aspect_ratio=decrease:reset_sar=1:flags=lanczos",
        args.fps, MAX_WIDTH, MAX_HEIGHT
    );
    if let Some(subtitles) = args.subtitles {
        filter.insert_str(
            0,
            &format!("subtitles='{}',", subtitles.replace("'", r"\'")),
        );
    }
    Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            &args.input,
            "-an",
            "-vf",
            &filter,
            "-c:v",
            "mjpeg",
            "-q:v",
            "10",
            "-f",
            "avi",
            "-y",
            &args.output,
        ])
        .status()?;
    Ok(())
}

fn encode_yuv(args: Args) -> Result<(), Box<dyn Error>> {
    let mut filter = format!(
        "framerate={},scale=size={}x{}:force_original_aspect_ratio=decrease:reset_sar=1:out_color_matrix=bt709:out_range=full:out_primaries=bt709:out_transfer=bt709",
        args.fps, MAX_WIDTH, MAX_HEIGHT
    );
    if let Some(subtitles) = args.subtitles {
        filter.insert_str(
            0,
            &format!("subtitles='{}',", subtitles.replace("'", r"\'")),
        );
    }
    Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            &args.input,
            "-an",
            "-vf",
            &filter,
            "-c:v",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
            "-f",
            "avi",
            "-y",
            &args.output,
        ])
        .output()?;
    Ok(())
}

fn encode_rgb(args: Args) -> Result<(), Box<dyn Error>> {
    let mut filter = format!(
        "framerate={},scale=size={}x{}:force_original_aspect_ratio=decrease:reset_sar=1:out_color_matrix=bt709:out_range=full:out_primaries=bt709:out_transfer=bt709",
        args.fps, MAX_WIDTH, MAX_HEIGHT
    );
    if let Some(subtitles) = args.subtitles {
        filter.insert_str(
            0,
            &format!("subtitles='{}',", subtitles.replace("'", r"\'")),
        );
    }
    Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            &args.input,
            "-an",
            "-vf",
            &filter,
            "-c:v",
            "rawvideo",
            "-tag:v",
            "0",
            "-f",
            "avi",
            "-y",
            &args.output,
        ])
        .status()?;
    Ok(())
}
