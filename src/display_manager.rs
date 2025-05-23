use std::io::{self, Write};

use crate::kitty_graphics_protocol_encoder::KittyGraphicsProtocolFrame;
use crate::result::Res;
use crate::ring_buffer::RingBuffer;
use crate::{Arc, Mutex};

use crate::kitty_graphics_protocol_encoder::KittyGraphicsProtocolBuffer;

pub struct DisplayManager {
    kitty_graphics_protocolbuffer: Arc<Mutex<KittyGraphicsProtocolBuffer>>,
}

impl DisplayManager {
    pub fn new(kitty_graphics_protocolbuffer: Arc<Mutex<KittyGraphicsProtocolBuffer>>) -> Self {
        DisplayManager {
            kitty_graphics_protocolbuffer,
        }
    }

    fn display_frame(&self, frame: KittyGraphicsProtocolFrame) -> Res<()> {
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
            if self.kitty_graphics_protocolbuffer.lock().unwrap().len() == 0 {
                // No frames to display, sleep for a bit
                std::thread::sleep(std::time::Duration::from_secs(3));
            } else {
                let kitty_graphics_protocolframe = self
                    .kitty_graphics_protocolbuffer
                    .lock()
                    .unwrap()
                    .get_frame();
                if let Some(frame) = kitty_graphics_protocolframe {
                    if std::time::Instant::now().duration_since(now).as_millis()
                        >= frame.timestamp as u128
                    {
                        self.reset_cursor()?;
                        self.display_frame(frame)?;
                    }
                }
            }
        }
        Ok(())
    }
}
