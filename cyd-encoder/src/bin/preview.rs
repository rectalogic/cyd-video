use std::{error::Error, process::Command};

#[derive(argh::FromArgs)]
/// Play video with custom header format
struct Args {
    #[argh(option, default = "\"mjpeg\".to_string()")]
    /// video format (mjpeg, rgb or yuv)
    format: String,
    #[argh(positional)]
    input: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    match args.format.as_str() {
        "mjpeg" => preview_mjpeg(args),
        "yuv" => preview_yuv(args),
        "rgb" => preview_rgb(args),
        _ => Err("invalid format".into()),
    }
}

fn preview_mjpeg(args: Args) -> Result<(), Box<dyn Error>> {
    Command::new("ffplay")
        .args(["-hide_banner", "-f", "avi", &args.input])
        .status()?;

    Ok(())
}

fn preview_yuv(args: Args) -> Result<(), Box<dyn Error>> {
    Command::new("ffplay")
        .args(["-hide_banner", "-f", "avi", &args.input])
        .status()?;

    Ok(())
}

fn preview_rgb(args: Args) -> Result<(), Box<dyn Error>> {
    eprintln!("ffplay plays rgb555le instead of rgb565be");
    Command::new("ffplay")
        .args(["-hide_banner", "-f", "avi", &args.input])
        .status()?;

    Ok(())
}
