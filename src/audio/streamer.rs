use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{mpsc, Arc, Mutex},
};

use crate::helpers::{
    structs::RingBuffer,
    types::{Bytes, Res},
};

pub struct AudioStreamer {
    audio_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
    url: String,
    streaming_done_tx: mpsc::Sender<()>,
}

impl AudioStreamer {
    pub fn new(
        audio_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
        url: String,
        streaming_done_tx: mpsc::Sender<()>,
    ) -> Res<Self> {
        Ok(AudioStreamer {
            audio_buffer,
            url,
            streaming_done_tx,
        })
    }

    pub fn stream(&self) -> Res<()> {
        let mut yt_dlp_process = Command::new("yt-dlp")
            .args(["-o", "-", "--no-part", "-f", "bestaudio", &self.url])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Could not start yt-dlp process");

        let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

        let mut ffmpeg_process = Command::new("ffmpeg")
            .args([
                "-i", "pipe:0", "-vn", "-f", "s16le", "-ac", "2", "-ar", "48000", "-",
            ])
            .stdin(Stdio::from(yt_dlp_stdout))
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Could not start ffmpeg process");

        let mut ffmpeg_stdout = ffmpeg_process
            .stdout
            .take()
            .expect("Failed to get ffmpeg stdout");

        let sample_size = 48000 * 2 * 2;
        let mut read_buffer = vec![0u8; sample_size];

        let mut accumulated_data = Vec::new();

        loop {
            match ffmpeg_stdout.read(&mut read_buffer) {
                Ok(0) => {
                    self.streaming_done_tx
                        .send(())
                        .expect("Failed to send streaming done signal");

                    break;
                }

                Ok(bytes_read) => {
                    accumulated_data.extend_from_slice(&read_buffer[0..bytes_read]);

                    while accumulated_data.len() >= sample_size {
                        let sample_data =
                            accumulated_data.drain(0..sample_size).collect::<Vec<u8>>();

                        self.audio_buffer.lock().unwrap().push_el(sample_data);
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

        let _ = yt_dlp_process.wait();
        let _ = ffmpeg_process.wait();

        Ok(())
    }
}
