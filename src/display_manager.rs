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

    fn display_frame(&self, frame: KittyFrame) {
        let mut stdout = io::stdout();
        // Write the frame data and check for errors
        write!(stdout, "{}", frame.data).expect("Failed to write frame data");
        // Write the log message using the same stdout handle
        write!(stdout, "Displaying frame: {}\n", frame.timestamp).expect("Failed to write log");
        // Flush once at the end
        stdout.flush().expect("Failed to flush stdout");
    }

    pub fn display(&self) {
        loop {
            let kitty_frame = self.kitty_buffer.lock().unwrap().get_frame();
            if let Some(frame) = kitty_frame {
                self.display_frame(frame);
            } else {
                // Sleep for a short duration to avoid busy waiting
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
        }
    }
}
