use std::io::{self, Write};
use std::sync::mpsc;

use crate::result::Res;
use crate::ring_buffer::{Frame, RingBuffer};
use crate::{Arc, Mutex};

pub struct DisplayManager {
    encoded_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    encoding_done_rx: mpsc::Receiver<()>,
    display_started_tx: mpsc::Sender<()>,
}

impl DisplayManager {
    pub fn new(
        encoded_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        encoding_done_rx: mpsc::Receiver<()>,
        display_started_tx: mpsc::Sender<()>,
    ) -> Self {
        DisplayManager {
            encoded_buffer,
            encoding_done_rx,
            display_started_tx,
        }
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
        let msg_sent = false;

        loop {
            if self.encoded_buffer.lock().unwrap().len() == 0 {
                if self.encoding_done_rx.try_recv().is_ok() {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_secs(3));
            } else {
                if !msg_sent {
                    self.display_started_tx.send(()).unwrap();
                }

                let encoded_frame = self.encoded_buffer.lock().unwrap().get_frame();
                if let Some(frame) = encoded_frame {
                    if std::time::Instant::now().duration_since(now).as_millis()
                        >= frame.timestamp as u128
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
    use crate::ring_buffer::RingBuffer;
    use std::sync::mpsc;

    #[test]
    fn test_display_manager() {
        let (tx, rx) = mpsc::channel();
        let encoded_buffer = Arc::new(Mutex::new(RingBuffer::new()));
        let display_manager = DisplayManager::new(encoded_buffer.clone(), rx);

        let frame = Frame {
            data: vec![1, 2, 3],
            timestamp: 1000,
        };

        encoded_buffer.lock().unwrap().push_frame(frame);

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
