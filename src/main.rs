use std::{collections::VecDeque, process::Command};

use clap::Parser;

mod result;

const MAX_BUFFER_SIZE: usize = 100;

struct VideoFrame {
    data: Vec<u8>,
    timestamp: u64,
}

struct VideoBuffer {
    frames: VecDeque<VideoFrame>,
}

impl VideoBuffer {
    fn new() -> Self {
        VideoBuffer {
            frames: VecDeque::with_capacity(MAX_BUFFER_SIZE),
        }
    }

    fn push_frame(&mut self, frame: VideoFrame) {
        if self.frames.len() >= MAX_BUFFER_SIZE {
            // If buffer is full, remove the oldest frame
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    fn get_frame(&mut self) -> Option<VideoFrame> {
        self.frames.pop_front()
    }

    fn len(&self) -> usize {
        self.frames.len()
    }
}

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
        .spawn()?;

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
        .spawn()?;
}
