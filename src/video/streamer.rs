use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{mpsc, Arc, Mutex},
};

use crate::helpers::{
    structs::ContentQueue,
    types::{Bytes, Res},
};

pub struct VideoStreamer {
    rgb_buffer: Arc<Mutex<ContentQueue<Bytes>>>,
    streaming_done_tx: mpsc::Sender<()>,
    url: String,
    width: usize,
    height: usize,
}

impl VideoStreamer {
    pub fn new(
        rgb_buffer: Arc<Mutex<ContentQueue<Bytes>>>,
        streaming_done_tx: mpsc::Sender<()>,
        url: String,
        width: usize,
        height: usize,
    ) -> Res<Self> {
        Ok(VideoStreamer {
            rgb_buffer,
            streaming_done_tx,
            url,
            width,
            height,
        })
    }

    pub fn stream(&self) -> Res<()> {
        let frame_size = self.width * self.height * 3;
        let mut yt_dlp_process = Command::new("yt-dlp")
            .args([
                "-o",
                "-",
                "--no-part",
                "-f",
                format!("bestvideo[height={}][width={}]", self.height, self.width).as_str(),
                &self.url,
            ])
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Could not start yt-dlp process");

        let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

        let mut ffmpeg_process = Command::new("ffmpeg")
            .args(["-i", "pipe:0", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
            .stdin(Stdio::from(yt_dlp_stdout))
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Could not start ffmpeg process");

        let mut ffmpeg_stdout = ffmpeg_process
            .stdout
            .take()
            .expect("Failed to get ffmpeg stdout");

        let mut accumulated_data = Vec::new();

        let mut read_buffer = vec![0u8; frame_size];

        loop {
            match ffmpeg_stdout.read(&mut read_buffer) {
                Ok(0) => {
                    self.streaming_done_tx.send(()).unwrap();
                    break;
                }
                Ok(bytes_read) => {
                    accumulated_data.extend_from_slice(&read_buffer[0..bytes_read]);

                    while accumulated_data.len() >= frame_size {
                        let frame_data = accumulated_data.drain(0..frame_size).collect::<Vec<u8>>();

                        self.rgb_buffer.lock().unwrap().push_el(frame_data);
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from ffmpeg: {}", e);
                    break;
                }
            }
        }

        if !accumulated_data.is_empty() {
            println!("Leftover incomplete data: {} bytes", accumulated_data.len());
        }

        let _ = ffmpeg_process.wait();
        let _ = yt_dlp_process.wait();
        Ok(())
    }
}
