use crate::ring_buffer::{RingBuffer, MAX_BUFFER_SIZE};
use base64::{engine::general_purpose, Engine as _};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::video_buffer::{VideoBuffer, VideoFrame};

// Kitty graphics protocol constants
const KITTY_GRAPHICS_START: &str = "\x1B_G";
const KITTY_GRAPHICS_END: &str = "\x1B\\";

pub struct KittyFrame {
    pub data: String,
    pub timestamp: u64,
}

impl KittyFrame {
    pub fn new(data: String, timestamp: u64) -> Self {
        KittyFrame { data, timestamp }
    }
}

pub struct KittyBuffer {
    frames: VecDeque<KittyFrame>,
}

impl RingBuffer<KittyFrame> for KittyBuffer {
    fn new() -> Self {
        KittyBuffer {
            frames: VecDeque::with_capacity(MAX_BUFFER_SIZE),
        }
    }

    fn push_frame(&mut self, frame: KittyFrame) {
        if self.frames.len() >= MAX_BUFFER_SIZE {
            // If buffer is full, remove the oldest frame
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    fn get_frame(&mut self) -> Option<KittyFrame> {
        self.frames.pop_front()
    }

    fn len(&self) -> usize {
        self.frames.len()
    }
}

pub struct KittyEncoder {
    video_buffer: Arc<Mutex<VideoBuffer>>,
    kitty_buffer: Arc<Mutex<KittyBuffer>>,
    width: usize,
    height: usize,
}

impl KittyEncoder {
    pub fn new(
        video_buffer: Arc<Mutex<VideoBuffer>>,
        kitty_buffer: Arc<Mutex<KittyBuffer>>,
        width: usize,
        height: usize,
    ) -> Self {
        KittyEncoder {
            video_buffer,
            kitty_buffer,
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

    pub fn encode(&self) {
        loop {
            // Get the video frame from the video buffer
            let mut video_buffer = self.video_buffer.lock().unwrap();
            let frame = video_buffer.get_frame();

            if let Some(frame) = frame {
                // Encode the frame to Kitty graphics protocol
                let kitty_frame = self.encode_frame_kitty(frame);

                // Push the encoded frame to the kitty buffer
                let mut kitty_buffer = self.kitty_buffer.lock().unwrap();
                kitty_buffer.push_frame(kitty_frame);
                println!("Kitty buffer length: {}", kitty_buffer.len());
            }
        }
    }
}
