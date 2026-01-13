use cyd_encoder::format::{FormatHeader, mjpeg::MjpegHeader, yuv::YuvHeader};
use std::{error::Error, fs::File, io::Read, process::Command};

#[derive(argh::FromArgs)]
/// Play video with custom header format
struct Args {
    #[argh(option, default = "\"mjpeg\".to_string()")]
    /// video format (mjpeg or yuv)
    format: String,
    #[argh(positional)]
    input: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    match args.format.as_str() {
        "mjpeg" => preview_mjpeg(args),
        "yuv" => preview_yuv(args),
        _ => Err("invalid format".into()),
    }
}

fn preview_mjpeg(args: Args) -> Result<(), Box<dyn Error>> {
    let mut input = File::open(&args.input)?;
    let mut buffer = [0u8; 1];
    input.read_exact(&mut buffer)?;
    let header = MjpegHeader::parse(&buffer);
    Command::new("ffplay")
        .args([
            "-skip_initial_bytes",
            &MjpegHeader::header_size().to_string(),
            "-framerate",
            &header.fps().to_string(),
            "-f",
            "mjpeg",
            &args.input,
        ])
        .status()?;

    Ok(())
}

fn preview_yuv(args: Args) -> Result<(), Box<dyn Error>> {
    let mut input = File::open(&args.input)?;
    let mut buffer = [0u8; 5];
    input.read_exact(&mut buffer)?;
    let header = YuvHeader::parse(&buffer);
    let size = format!("{}x{}", header.width(), header.height());
    Command::new("ffplay")
        .args([
            "-skip_initial_bytes",
            &YuvHeader::header_size().to_string(),
            "-framerate",
            &header.fps().to_string(),
            "-video_size",
            &size,
            "-pixel_format",
            "yuv420p",
            "-color_range",
            "full",
            "-colorspace",
            "bt709",
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            &args.input,
        ])
        .status()?;

    Ok(())
}
