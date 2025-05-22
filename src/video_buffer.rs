use std::collections::VecDeque;

use crate::ring_buffer::{RingBuffer, MAX_BUFFER_SIZE};

pub struct VideoFrame {
    pub data: Vec<u8>,
}

impl VideoFrame {
    pub fn new(data: Vec<u8>) -> Self {
        VideoFrame { data }
    }
}

pub struct VideoBuffer {
    frames: VecDeque<VideoFrame>,
}

impl RingBuffer<VideoFrame> for VideoBuffer {
    fn new() -> Self {
        VideoBuffer {
            frames: VecDeque::with_capacity(MAX_BUFFER_SIZE),
        }
    }

    fn push_frame(&mut self, frame: VideoFrame) {
        if self.frames.len() >= MAX_BUFFER_SIZE {
            // If buffer is full, remove the oldest frame
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    fn get_frame(&mut self) -> Option<VideoFrame> {
        self.frames.pop_front()
    }

    fn len(&self) -> usize {
        self.frames.len()
    }
}
