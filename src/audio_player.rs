use std::{
    io::Read,
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
            .spawn()
            .expect("Could not start yt-dlp process");

        let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

        //self.display_started_rx
        //   .recv()
        //  .expect("Failed to receive display started signal");

        let mut ffmpeg_process = Command::new("ffmpeg")
            .args([
                "-i", "pipe:0", "-vn", "-f", "s16le", "-ac", "2", "-ar", "48000", "-",
            ])
            .stdin(Stdio::from(yt_dlp_stdout))
            .stdout(Stdio::piped()) // Add this line - you need stdout for reading
            .spawn()
            .expect("Could not start ffmpeg process");

        let mut ffmpeg_stdout = ffmpeg_process
            .stdout
            .take()
            .expect("Failed to get ffmpeg stdout");

        println!("Audio player started, waiting for display to start...");

        // Pre-allocate buffer with size - empty vec has no capacity to read into
        let mut read_buffer = vec![0u8; 8192]; // 8KB buffer

        loop {
            match ffmpeg_stdout.read(&mut read_buffer) {
                Ok(0) => {
                    println!("End of audio stream");
                    break;
                }
                Ok(bytes_read) => {
                    println!("Read {} bytes from ffmpeg", bytes_read);
                    // Here you would calculate timestamp and buffer the audio data
                    // For now, just continue reading
                }
                Err(e) => {
                    eprintln!("Error reading from ffmpeg: {}", e);
                    break;
                }
            }
        }

        let _ = yt_dlp_process.wait();
        let _ = ffmpeg_process.wait();
    }
}
