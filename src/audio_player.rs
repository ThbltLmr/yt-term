use std::{
    process::{Command, Stdio},
    sync::mpsc,
};

use crate::result::Res;

pub struct AudioPlayer {
    display_started_rx: mpsc::Receiver<()>,
    url: String,
}

impl AudioPlayer {
    pub fn new(display_started_rx: mpsc::Receiver<()>, url: String) -> Res<Self> {
        Ok(AudioPlayer {
            display_started_rx,
            url,
        })
    }

    pub fn play(&self) {
        let mut yt_dlp_process = Command::new("yt-dlp")
            .args([
                "-o",
                "-",
                "--no-part",
                "-f",
                format!("bestaudio").as_str(),
                &self.url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Could not start yt-dlp process");

        let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

        self.display_started_rx
            .recv()
            .expect("Failed to receive display started signal");

        let mut ffmpeg_process = Command::new("ffmpeg")
            .args(["-i", "pipe:0", "-vn", "-f", "pulse", "default"])
            .stdin(Stdio::from(yt_dlp_stdout))
            .stderr(Stdio::null())
            .spawn()
            .expect("Could not start ffmpeg process");

        let _ = yt_dlp_process.wait();
        let _ = ffmpeg_process.wait();
    }
}
