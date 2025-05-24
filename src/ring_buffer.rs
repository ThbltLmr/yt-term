use std::collections::VecDeque;

pub struct Frame {
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl Frame {
    pub fn new(data: Vec<u8>, timestamp: u64) -> Self {
        Frame { data, timestamp }
    }
}

pub struct RingBuffer<T> {
    frames: VecDeque<T>,
}

impl<T> RingBuffer<T> {
    pub fn new() -> Self {
        RingBuffer {
            frames: VecDeque::new(),
        }
    }

    pub fn push_frame(&mut self, frame: T) {
        self.frames.push_back(frame);
    }

    pub fn get_frame(&mut self) -> Option<T> {
        self.frames.pop_front()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }
}
