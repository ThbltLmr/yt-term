use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::{
    structs::{Frame, RingBuffer, Sample},
    types::Res,
};

pub struct Logger {
    log_file: std::fs::File,
    raw_video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    encoded_video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    audio_buffer: Arc<Mutex<RingBuffer<Sample>>>,
    ready_video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    ready_audio_buffer: Arc<Mutex<RingBuffer<Sample>>>,
    playing_done_rx: std::sync::mpsc::Receiver<()>,
}

impl Logger {
    pub fn new(
        raw_video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        encoded_video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        audio_buffer: Arc<Mutex<RingBuffer<Sample>>>,
        ready_video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        ready_audio_buffer: Arc<Mutex<RingBuffer<Sample>>>,
        playing_done_rx: std::sync::mpsc::Receiver<()>,
    ) -> Res<Self> {
        let log_file =
            File::create("log.csv").map_err(|e| format!("Failed to create log file: {}", e))?;
        Ok(Logger {
            log_file,
            raw_video_buffer,
            encoded_video_buffer,
            audio_buffer,
            ready_video_buffer,
            ready_audio_buffer,
            playing_done_rx,
        })
    }

    fn write_header(&mut self) -> Res<()> {
        writeln!(
            self.log_file,
            "timestamp,raw_video_size,encoded_video_size,audio_size,ready_video_size,ready_audio_size"
        )
        .map_err(|e| format!("Failed to write header to log file: {}", e))?;
        Ok(())
    }

    fn add_log_entry(&mut self) -> Res<()> {
        writeln!(
            self.log_file,
            "{},{},{},{},{},{}",
            Instant::now().elapsed().as_secs_f64(),
            self.raw_video_buffer.lock().unwrap().len(),
            self.encoded_video_buffer.lock().unwrap().len(),
            self.audio_buffer.lock().unwrap().len(),
            self.ready_video_buffer.lock().unwrap().len(),
            self.ready_audio_buffer.lock().unwrap().len(),
        )
        .map_err(|e| format!("Failed to write to log file: {}", e))?;
        Ok(())
    }

    pub fn log(&mut self) -> Res<()> {
        let _ = self.write_header()?;

        while let Err(_) = self.playing_done_rx.try_recv() {
            std::thread::sleep(std::time::Duration::from_millis(200));
            self.add_log_entry()?;
        }

        Ok(())
    }
}
