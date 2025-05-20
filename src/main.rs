use std::process::{Command, Stdio};

use clap::Parser;

mod result;
mod video_buffer;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long)]
    url: String,

    #[clap(short, long, default_value = "247")]
    format: String,
}

fn main() {
    let args = Args::parse();
    let url = args.url;

    let mut yt_dlp_process = Command::new("yt-dlp")
        .args([
            "-o",
            "-",         // Output to stdout
            "--no-part", // Don't create .part files
            "-f",
            "best", // Choose format
            &url,
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start child process");

    // Connect yt-dlp's stdout to ffmpeg's stdin
    let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

    let mut ffmpeg_process = Command::new("ffmpeg")
        .args([
            "-i", "pipe:0", // Read from stdin
            "-f", "rawvideo", "-pix_fmt", "rgb24",
            // Other options...
            "-", // Output to stdout
        ])
        .stdin(Stdio::from(yt_dlp_stdout)) // Connect the pipes
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start child process");
}
