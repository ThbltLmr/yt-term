use std::io::{self, Write};

use crate::kitty_encoder::KittyFrame;
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

    pub fn display_frame(&self, frame: KittyFrame) {
        let mut stdout = io::stdout();
        stdout.write_all(&frame.data).unwrap();
        stdout.flush().expect("Failed to flush stdout");
    }

    pub fn display(&self) {
        while self.kitty_buffer.lock().unwrap().len() < 100 {
            // Wait for the buffer to fill up
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let mut current_timestamp = 0;

        loop {
            if self.kitty_buffer.lock().unwrap().len() == 0 {
                // No frames to display, sleep for a bit
                std::thread::sleep(std::time::Duration::from_secs(3));
            } else {
                let kitty_frame = self.kitty_buffer.lock().unwrap().get_frame();
                if let Some(frame) = kitty_frame {
                    if frame.timestamp == current_timestamp {
                        let mut stdout = io::stdout();

                        let reset_cursor = b"\x1B[H";
                        stdout.write_all(reset_cursor).unwrap();
                        stdout.flush().expect("Failed to flush stdout");

                        self.display_frame(frame);
                        current_timestamp += 40;
                    } else {
                        panic!("Frame timestamp mismatch");
                    }
                }
            }
        }
    }
}
