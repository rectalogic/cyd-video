use cyd_format::{HEADER_SIZE, parse_header};
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
    let fps = parse_header(&header);
    Command::new("ffplay")
        .args([
            "-skip_initial_bytes",
            &HEADER_SIZE.to_string(),
            "-framerate",
            &fps.to_string(),
            "-f",
            "mjpeg",
            &args.input,
        ])
        .status()?;

    Ok(())
}
