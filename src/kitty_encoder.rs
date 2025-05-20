use base64::{engine::general_purpose, Engine as _};
use std::sync::{Arc, Mutex};

use crate::video_buffer::{VideoBuffer, VideoFrame};

// Kitty graphics protocol constants
const KITTY_GRAPHICS_START: &str = "\x1B_G";
const KITTY_GRAPHICS_END: &str = "\x1B\\";

pub struct KittyFrame {
    data: String,
    timestamp: u64,
}

impl KittyFrame {
    pub fn new(data: String, timestamp: u64) -> Self {
        KittyFrame { data, timestamp }
    }
}

pub struct KittyEncoder {
    buffer: Arc<Mutex<VideoBuffer>>,
    width: usize,
    height: usize,
}

impl KittyEncoder {
    pub fn new(buffer: Arc<Mutex<VideoBuffer>>, width: usize, height: usize) -> Self {
        KittyEncoder {
            buffer,
            width,
            height,
        }
    }

    // Convert a frame to Kitty graphics protocol
    fn encode_frame_kitty(&self, frame: VideoFrame) -> KittyFrame {
        // Base64 encode the frame data
        let encoded_data = general_purpose::STANDARD.encode(&frame.data);

        // Format according to Kitty protocol
        // a=T: Transmit directly (not compressed)
        // f=24: RGB (24-bit color)
        // s=<width>: Width of image
        // v=<height>: Height of image
        // t=d: Direct (not using placement id)
        let kitty_data: String = format!(
            "{};a=T;f=24;s={};v={};t=d,{}{}",
            KITTY_GRAPHICS_START, self.width, self.height, encoded_data, KITTY_GRAPHICS_END
        );

        KittyFrame::new(kitty_data, frame.timestamp)
    }
}
