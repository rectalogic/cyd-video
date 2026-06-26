use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::{fs, path::PathBuf};
use xshell::{Shell, cmd};

const MAX_WIDTH: u32 = 320;
const MAX_HEIGHT: u32 = 240;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Xtask {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Copy, Clone)]
enum Format {
    /// mjpeg
    Mjpeg,
    /// rgb
    Rgb,
    /// yuv
    Yuv,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode video into AVI with specified format
    Encode {
        /// video format (mjpeg, rgb or yuv)
        #[arg(short, long, default_value_t = Format::Mjpeg)]
        format: Format,
        /// frames per second
        #[arg(short, long, default_value_t = 15)]
        fps: u8,
        /// path to subtitles srt/vtt file
        #[arg(short, long)]
        subtitles: Option<PathBuf>,
        /// Input video path
        #[arg()]
        input: PathBuf,
        /// Output AVI video path
        #[arg()]
        output: PathBuf,
    },
    /// Play video with
    Preview {
        /// Input AVI video path
        #[arg()]
        input: PathBuf,
    },
}

fn main() {
    let xtask = Xtask::parse();
    let result = match xtask.command {
        Commands::Encode {
            format,
            fps,
            subtitles,
            input,
            output,
        } => match format {
            Format::Mjpeg => encode_mjpeg(fps, subtitles, input, output),
            Format::Rgb => encode_rgb(fps, subtitles, input, output),
            Format::Yuv => encode_yuv(fps, subtitles, input, output),
        },
        Commands::Preview { input } => preview(input),
    };
    if let Err(err) = result {
        fatal_error(err);
    }
}

fn preview(input: PathBuf) -> anyhow::Result<()> {
    Command::new("ffplay")
        .args([
            "-hide_banner",
            "-f",
            "avi",
            input
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("invalid input file"))?,
        ])
        .status()?;

    Ok(())
}
fn encode_mjpeg(
    fps: u8,
    subtitles: Option<PathBuf>,
    input: PathBuf,
    output: PathBuf,
) -> anyhow::Result<()> {
    let mut filter = format!(
        "framerate={},scale=size={}x{}:force_original_aspect_ratio=decrease:reset_sar=1:flags=lanczos",
        fps, MAX_WIDTH, MAX_HEIGHT
    );
    if let Some(subtitles) = subtitles {
        filter.insert_str(
            0,
            &format!("subtitles='{}',", subtitles.replace("'", r"\'")),
        );
    }
    Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            input
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("invalid input file"))?,
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
            output
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("invalid output file"))?,
        ])
        .status()?;
    Ok(())
}

fn encode_yuv(
    fps: u8,
    subtitles: Option<PathBuf>,
    input: PathBuf,
    output: PathBuf,
) -> anyhow::Result<()> {
    let mut filter = format!(
        "framerate={},scale=size={}x{}:force_original_aspect_ratio=decrease:reset_sar=1:out_color_matrix=bt709:out_range=full:out_primaries=bt709:out_transfer=bt709",
        fps, MAX_WIDTH, MAX_HEIGHT
    );
    if let Some(subtitles) = subtitles {
        filter.insert_str(
            0,
            &format!("subtitles='{}',", subtitles.replace("'", r"\'")),
        );
    }
    Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            input
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("invalid input file"))?,
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
            output
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("invalid input file"))?,
        ])
        .output()?;
    Ok(())
}

fn encode_rgb(
    fps: u8,
    subtitles: Option<PathBuf>,
    input: PathBuf,
    output: PathBuf,
) -> anyhow::Result<()> {
    let mut filter = format!(
        "framerate={},scale=size={}x{}:force_original_aspect_ratio=decrease:reset_sar=1:out_color_matrix=bt709:out_range=full:out_primaries=bt709:out_transfer=bt709",
        fps, MAX_WIDTH, MAX_HEIGHT
    );
    if let Some(subtitles) = subtitles {
        filter.insert_str(
            0,
            &format!("subtitles='{}',", subtitles.replace("'", r"\'")),
        );
    }
    Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            input
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("invalid input file"))?,
            "-an",
            "-vf",
            &filter,
            "-c:v",
            "rawvideo",
            "-pix_fmt",
            "rgb565be",
            "-tag:v",
            "0",
            "-f",
            "avi",
            "-y",
            output
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("invalid input file"))?,
        ])
        .status()?;
    Ok(())
}

fn fatal_error(err: anyhow::Error) {
    eprintln!("{}", err);
    std::process::exit(1);
}
