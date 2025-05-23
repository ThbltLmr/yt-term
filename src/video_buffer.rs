use std::collections::VecDeque;

use crate::ring_buffer::RingBuffer;

pub struct VideoFrame {
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl VideoFrame {
    pub fn new(data: Vec<u8>, timestamp: u64) -> Self {
        VideoFrame { data, timestamp }
    }
}

pub struct VideoBuffer {
    frames: VecDeque<VideoFrame>,
}

impl RingBuffer<VideoFrame> for VideoBuffer {
    fn new() -> Self {
        VideoBuffer {
            frames: VecDeque::new(),
        }
    }

    fn push_frame(&mut self, frame: VideoFrame) {
        self.frames.push_back(frame);
    }

    fn get_frame(&mut self) -> Option<VideoFrame> {
        self.frames.pop_front()
    }

    fn len(&self) -> usize {
        self.frames.len()
    }
}
