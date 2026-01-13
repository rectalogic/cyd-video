use cyd_encoder::format::{self, FormatHeader};
use regex::Regex;
use std::{
    error::Error,
    fs::{File, rename},
    io::{self, Write},
    path::Path,
    process::{Command, exit},
    str::FromStr,
};

#[derive(argh::FromArgs)]
/// Encode video into format with custom header
struct Args {
    #[argh(option, default = "\"mjpeg\".to_string()")]
    /// video format (mjpeg or yuv)
    format: String,
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
    match args.format.as_str() {
        "mjpeg" => encode_mjpeg(args),
        "yuv" => encode_yuv(args),
        _ => Err("invalid format".into()),
    }
}

fn encode_mjpeg(args: Args) -> Result<(), Box<dyn Error>> {
    let header = format::mjpeg::MjpegHeader::new(args.fps);
    let mut filter = format!(
        "framerate={},scale=size={}x{}:force_original_aspect_ratio=decrease:reset_sar=1",
        args.fps,
        format::mjpeg::MjpegHeader::MAX_WIDTH,
        format::mjpeg::MjpegHeader::MAX_HEIGHT
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

    prepend_header(args.output, header)?;
    Ok(())
}

fn encode_yuv(args: Args) -> Result<(), Box<dyn Error>> {
    const DUMP_SEPARATOR: &str = "@@!!!!@@";
    let pattern = format!(r"{DUMP_SEPARATOR}.* (\d+)x(\d+) .*{DUMP_SEPARATOR}");
    let re = Regex::new(&pattern)?;

    let mut filter = format!(
        "framerate={},scale=size={}x{}:force_original_aspect_ratio=decrease:reset_sar=1:out_color_matrix=bt709:out_range=full:out_primaries=bt709:out_transfer=bt709",
        args.fps,
        format::yuv::YuvHeader::MAX_WIDTH,
        format::yuv::YuvHeader::MAX_HEIGHT
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

    let header = format::yuv::YuvHeader::new(width, height, args.fps);
    prepend_header(args.output, header)?;
    Ok(())
}

fn prepend_header<P: AsRef<Path>, const HEADER_SIZE: usize, F: FormatHeader<HEADER_SIZE>>(
    path: P,
    header: F,
) -> io::Result<()> {
    let path = path.as_ref();
    let tmp_path = path.with_extension("tmp");

    let mut input = File::open(path)?;
    let mut output = File::create(&tmp_path)?;

    let mut buffer = [0u8; HEADER_SIZE];
    header.encode(&mut buffer);
    output.write_all(&buffer)?;

    io::copy(&mut input, &mut output)?;

    output.flush()?;

    rename(tmp_path, path)?;

    Ok(())
}
