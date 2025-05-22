use crate::ring_buffer::{RingBuffer, MAX_BUFFER_SIZE};
use base64::{engine::general_purpose, Engine as _};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use crate::video_buffer::{VideoBuffer, VideoFrame};

pub struct KittyFrame {
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl KittyFrame {
    pub fn new(data: Vec<u8>, timestamp: u64) -> Self {
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

    pub fn encode_test_frame(&self) -> KittyFrame {
        // Create a test frame with a simple pattern
        let mut test_frame = Vec::new();
        for _y in 0..32 {
            for _x in 0..32 {
                test_frame.push(255);
                test_frame.push(0);
                test_frame.push(0);
            }
        }

        self.encode_frame_kitty(VideoFrame {
            data: test_frame,
            timestamp: 0,
        })
    }
    // Convert a frame to Kitty graphics protocol
    fn encode_frame_kitty(&self, frame: VideoFrame) -> KittyFrame {
        // Base64 encode the frame data
        let (control_data, payload) = (
            HashMap::from([
                ("f".into(), "24".into()),
                ("s".into(), "1280".into()),
                ("v".into(), "720".into()),
                ("t".into(), "d".into()),
                ("a".into(), "T".into()),
            ]),
            frame.data,
        );

        let encoded_payload = self.encode_rbg(payload);
        let encoded_control_data = self.encode_control_data(control_data);
        let prefix = b"\x1b_G";
        let suffix = b"\x1b\\";
        let delimiter = b";";
        let mut buffer = vec![];

        buffer.extend_from_slice(prefix);
        buffer.extend_from_slice(&encoded_control_data);
        buffer.extend_from_slice(delimiter);
        buffer.extend_from_slice(&encoded_payload);
        buffer.extend_from_slice(suffix);

        KittyFrame::new(buffer, frame.timestamp)
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

    fn encode_control_data(&self, control_data: HashMap<String, String>) -> Vec<u8> {
        let mut encoded_data = Vec::new();
        for (key, value) in control_data {
            encoded_data.push(format!("{}={}", key, value));
        }

        encoded_data.join(",").as_bytes().to_vec()
    }

    fn encode_rbg(&self, rgb: Vec<u8>) -> Vec<u8> {
        let encoded = general_purpose::STANDARD.encode(&rgb);
        encoded.as_bytes().to_vec()
    }
}
