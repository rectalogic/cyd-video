use cyd_format::{encode_header, HEADER_SIZE};
use regex::Regex;
use std::{
    error::Error,
    fs::{rename, File},
    io::{self, Write},
    path::Path,
    process::{exit, Command},
    str::FromStr,
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
    const DUMP_SEPARATOR: &str = "@@!!!!@@";
    let pattern = format!(r"{DUMP_SEPARATOR}.* (\d+)x(\d+) .*{DUMP_SEPARATOR}");
    let re = Regex::new(&pattern)?;

    let args: Args = argh::from_env();
    let mut filter = format!(
        "framerate={},scale=size=320x240:force_original_aspect_ratio=decrease:reset_sar=1:out_color_matrix=bt709:out_range=full:out_primaries=bt709:out_transfer=bt709",
        args.fps
    );
    if let Some(subtitles) = args.subtitles {
        filter.insert_str(
            0,
            &format!("subtitles='{}',", subtitles.replace("'", r"\'")),
        );
    }
    let result = Command::new("ffmpeg")
        .args([
            "-i",
            &args.input,
            "-an",
            "-vf",
            &filter,
            "-pix_fmt",
            "yuv420p",
            "-f",
            "rawvideo",
            "-dump_separator",
            DUMP_SEPARATOR,
            "-y",
            &args.output,
        ])
        .output()?;
    if !result.status.success() {
        io::stdout().write_all(&result.stdout)?;
        io::stderr().write_all(&result.stderr)?;
        exit(1);
    }
    let stderr = str::from_utf8(&result.stderr)?;
    let cap = re.captures(stderr).ok_or("Failed to parse ffmpeg output")?;
    let width = u16::from_str(
        cap.get(1)
            .ok_or("Failed to parse ffmpeg output width")?
            .as_str(),
    )?;
    let height = u16::from_str(
        cap.get(2)
            .ok_or("Failed to parse ffmpeg output height")?
            .as_str(),
    )?;

    prepend_header(args.output, width, height, args.fps)?;

    Ok(())
}

fn prepend_header<P: AsRef<Path>>(path: P, width: u16, height: u16, fps: u8) -> io::Result<()> {
    let path = path.as_ref();
    let tmp_path = path.with_extension("tmp");

    let mut input = File::open(path)?;
    let mut output = File::create(&tmp_path)?;

    let mut header = [0u8; HEADER_SIZE];
    encode_header(&mut header, width, height, fps);
    output.write_all(&header)?;

    io::copy(&mut input, &mut output)?;

    output.flush()?;

    rename(tmp_path, path)?;

    Ok(())
}
