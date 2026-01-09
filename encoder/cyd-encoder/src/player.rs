use cyd_format::{parse_header, HEADER_SIZE};
use std::{error::Error, fs::File, io::Read, process::Command};

#[derive(argh::FromArgs)]
/// Play video with custom YUV header format
struct Args {
    #[argh(positional)]
    input: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    let mut input = File::open(&args.input)?;
    let mut header = [0u8; HEADER_SIZE];
    input.read_exact(&mut header)?;
    let (width, height, fps) = parse_header(&header);
    let size = format!("{width}x{height}");
    Command::new("ffplay")
        .args([
            "-skip_initial_bytes",
            &HEADER_SIZE.to_string(),
            "-framerate",
            &fps.to_string(),
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
