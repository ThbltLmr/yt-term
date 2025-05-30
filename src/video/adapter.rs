use std::io::{self, Write};
use std::sync::mpsc;
use std::time::Duration;

use crate::helpers::structs::{Frame, RingBuffer};
use crate::helpers::types::Res;
use crate::{Arc, Mutex};

pub struct TerminalAdapter {
    frame_interval: Duration,
    encoded_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    video_queueing_done_rx: mpsc::Receiver<()>,
}

impl TerminalAdapter {
    pub fn new(
        frame_interval: usize,
        encoded_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        video_queueing_done_rx: mpsc::Receiver<()>,
    ) -> Res<Self> {
        Ok(TerminalAdapter {
            frame_interval: Duration::from_millis(frame_interval as u64),
            encoded_buffer,
            video_queueing_done_rx,
        })
    }

    fn display_frame(&self, frame: Frame) -> Res<()> {
        let mut stdout = io::stdout();

        let reset_cursor = b"\x1B[H";
        let mut buffer = vec![];

        buffer.extend_from_slice(reset_cursor);
        buffer.extend_from_slice(&frame.data);

        stdout.write_all(&buffer)?;
        stdout.flush()?;
        Ok(())
    }

    pub fn display(&self) -> Res<()> {
        let mut last_frame_time = std::time::Instant::now();
        let mut total_frames_counter = 0;
        loop {
            if self.encoded_buffer.lock().unwrap().len() == 0 {
                if self.video_queueing_done_rx.try_recv().is_ok() {
                    return Ok(());
                }
            } else {
                if last_frame_time.elapsed() >= self.frame_interval {
                    let encoded_frame = self.encoded_buffer.lock().unwrap().get_el();
                    if let Some(frame) = encoded_frame {
                        if total_frames_counter > 0
                            && last_frame_time.elapsed()
                                > self.frame_interval + Duration::from_millis(2)
                        {
                            last_frame_time += self.frame_interval;
                            total_frames_counter += 1;
                            continue;
                        }

                        last_frame_time = std::time::Instant::now();
                        total_frames_counter += 1;
                        self.display_frame(frame)?;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn video_adapter_creation() {
        let (_tx, rx) = mpsc::channel();
        let encoded_buffer = Arc::new(Mutex::new(RingBuffer::new(30)));
        let display_manager = TerminalAdapter::new(30, encoded_buffer.clone(), rx);

        assert!(display_manager.is_ok());
    }
}
