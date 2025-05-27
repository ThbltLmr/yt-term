use std::io::{self, Write};
use std::sync::mpsc;

use crate::helpers::structs::{Frame, RingBuffer};
use crate::helpers::types::Res;
use crate::{Arc, Mutex};

pub struct TerminalAdapter {
    frame_interval: usize,
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
            frame_interval,
            encoded_buffer,
            video_queueing_done_rx,
        })
    }

    fn display_frame(&self, frame: Frame) -> Res<()> {
        let mut stdout = io::stdout();
        stdout.write_all(&frame.data)?;
        stdout.flush()?;
        Ok(())
    }

    fn reset_cursor(&self) -> Res<()> {
        let mut stdout = io::stdout();

        let reset_cursor = b"\x1B[H";
        let mut buffer = vec![];

        buffer.extend_from_slice(reset_cursor);
        stdout.write_all(&buffer)?;
        stdout.flush()?;
        Ok(())
    }

    pub fn display(&self) -> Res<()> {
        let now = std::time::Instant::now();

        loop {
            if self.encoded_buffer.lock().unwrap().len() == 0 {
                if self.video_queueing_done_rx.try_recv().is_ok() {
                    return Ok(());
                }
            } else {
                let encoded_frame = self.encoded_buffer.lock().unwrap().get_el();
                if let Some(frame) = encoded_frame {
                    if std::time::Instant::now().duration_since(now).as_millis()
                        >= self.frame_interval as u128
                    {
                        self.reset_cursor()?;
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
    fn test_display_manager() {
        let (tx, rx) = mpsc::channel();
        let encoded_buffer = Arc::new(Mutex::new(RingBuffer::new(30)));
        let display_manager = TerminalAdapter::new(30, encoded_buffer.clone(), rx).unwrap();

        let frame = Frame {
            data: vec![1, 2, 3],
        };

        encoded_buffer.lock().unwrap().push_el(frame);

        std::thread::spawn(move || {
            display_manager.display().unwrap();
        });

        std::thread::sleep(std::time::Duration::from_secs(1));

        tx.send(()).unwrap();

        assert_eq!(
            encoded_buffer.lock().unwrap().len(),
            0,
            "Buffer should be empty after sending done signal",
        );
    }
}
