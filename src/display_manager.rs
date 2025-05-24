use std::io::{self, Write};
use std::sync::mpsc;

use crate::result::Res;
use crate::ring_buffer::{Frame, RingBuffer};
use crate::{Arc, Mutex};

pub struct DisplayManager {
    encoded_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    encoding_done_rx: mpsc::Receiver<()>,
}

impl DisplayManager {
    pub fn new(
        encoded_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        encoding_done_rx: mpsc::Receiver<()>,
    ) -> Self {
        DisplayManager {
            encoded_buffer,
            encoding_done_rx,
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

        loop {
            if self.encoded_buffer.lock().unwrap().len() == 0 {
                if self.encoding_done_rx.try_recv().is_ok() {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_secs(3));
            } else {
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
