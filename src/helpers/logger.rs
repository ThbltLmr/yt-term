use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::types::Bytes;
use super::{structs::RingBuffer, types::Res};

pub struct Logger {
    start_time: Instant,
    log_file: std::fs::File,
    raw_video_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
    encoded_video_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
    audio_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
    ready_video_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
    ready_audio_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
    playing_done_rx: std::sync::mpsc::Receiver<()>,
}

impl Logger {
    pub fn new(
        raw_video_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
        encoded_video_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
        audio_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
        ready_video_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
        ready_audio_buffer: Arc<Mutex<RingBuffer<Bytes>>>,
        playing_done_rx: std::sync::mpsc::Receiver<()>,
    ) -> Res<Self> {
        let log_file =
            File::create("log.csv").map_err(|e| format!("Failed to create log file: {}", e))?;
        let start_time = Instant::now();

        Ok(Logger {
            start_time,
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
            self.start_time.elapsed().as_secs_f64(),
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
        self.write_header()?;

        while self.playing_done_rx.try_recv().is_err() {
            std::thread::sleep(std::time::Duration::from_millis(200));
            self.add_log_entry()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_logger_creation() {
        let raw_video_buffer = Arc::new(Mutex::new(RingBuffer::new(30)));
        let encoded_video_buffer = Arc::new(Mutex::new(RingBuffer::new(30)));
        let audio_buffer = Arc::new(Mutex::new(RingBuffer::new(30)));
        let ready_video_buffer = Arc::new(Mutex::new(RingBuffer::new(30)));
        let ready_audio_buffer = Arc::new(Mutex::new(RingBuffer::new(30)));
        let (_tx, rx) = mpsc::channel();

        let logger = Logger::new(
            raw_video_buffer,
            encoded_video_buffer,
            audio_buffer,
            ready_video_buffer,
            ready_audio_buffer,
            rx,
        );

        assert!(logger.is_ok());
    }
}
