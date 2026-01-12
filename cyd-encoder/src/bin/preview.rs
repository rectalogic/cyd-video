use cyd_encoder::format::{FormatHeader, mjpeg::MjpegHeader};
use std::{error::Error, fs::File, io::Read, process::Command};

#[derive(argh::FromArgs)]
/// Play video with custom YUV header format
struct Args {
    #[argh(option, default = "\"mjpeg\".to_string()")]
    /// video format
    format: String,
    #[argh(positional)]
    input: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    match args.format.as_str() {
        "mjpeg" => preview_mjpeg(args),
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
