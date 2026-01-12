use cyd_encoder::{HEADER_SIZE, encode_header};
use std::{
    error::Error,
    fs::{File, rename},
    io::{self, Write},
    path::Path,
    process::Command,
};

#[derive(argh::FromArgs)]
/// Encode video into custom YUV with header format
struct Args {
    #[argh(option, default = "25u8")]
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
    let mut filter = format!(
        "framerate={},scale=size=192x108:force_original_aspect_ratio=decrease:reset_sar=1",
        args.fps
    );
    if let Some(subtitles) = args.subtitles {
        filter.insert_str(
            0,
            &format!("subtitles='{}',", subtitles.replace("'", r"\'")),
        );
    }
    Command::new("ffmpeg")
        .args([
            "-i",
            &args.input,
            "-an",
            "-vf",
            &filter,
            "-f",
            "mjpeg",
            "-y",
            &args.output,
        ])
        .status()?;

    prepend_header(args.output, args.fps)?;

    Ok(())
}

fn prepend_header<P: AsRef<Path>>(path: P, fps: u8) -> io::Result<()> {
    let path = path.as_ref();
    let tmp_path = path.with_extension("tmp");

    let mut input = File::open(path)?;
    let mut output = File::create(&tmp_path)?;

    let mut header = [0u8; HEADER_SIZE];
    encode_header(&mut header, fps);
    output.write_all(&header)?;

    io::copy(&mut input, &mut output)?;

    output.flush()?;

    rename(tmp_path, path)?;

    Ok(())
}
