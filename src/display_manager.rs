use std::io::{self, Write};

use crate::kitty_encoder::KittyFrame;
use crate::result::Res;
use crate::ring_buffer::RingBuffer;
use crate::{Arc, Mutex};

use crate::kitty_encoder::KittyBuffer;

pub struct DisplayManager {
    kitty_buffer: Arc<Mutex<KittyBuffer>>,
}

impl DisplayManager {
    pub fn new(kitty_buffer: Arc<Mutex<KittyBuffer>>) -> Self {
        DisplayManager { kitty_buffer }
    }

    fn display_frame(&self, frame: KittyFrame) -> Res<()> {
        let mut stdout = io::stdout();
        stdout.write_all(&frame.data)?;
        stdout.flush()?;
        Ok(())
    }

    fn clear_terminal(&self) -> Res<()> {
        let mut stdout = io::stdout();

        let reset_cursor = b"\x1B[H";
        let clear_terminal = b"\x1B[2J";
        let mut buffer = vec![];

        buffer.extend_from_slice(reset_cursor);
        buffer.extend_from_slice(clear_terminal);
        stdout.write_all(reset_cursor)?;
        stdout.flush()?;
        Ok(())
    }

    pub fn display(&self) -> Res<()> {
        let now = std::time::Instant::now();

        loop {
            if self.kitty_buffer.lock().unwrap().len() == 0 {
                // No frames to display, sleep for a bit
                std::thread::sleep(std::time::Duration::from_secs(3));
            } else {
                let kitty_frame = self.kitty_buffer.lock().unwrap().get_frame();
                if let Some(frame) = kitty_frame {
                    if std::time::Instant::now().duration_since(now).as_millis()
                        >= frame.timestamp as u128
                    {
                        self.clear_terminal()?;
                        self.display_frame(frame)?;
                    }
                }
            }
        }
        Ok(())
    }
}
