use std::collections::VecDeque;

const MAX_BUFFER_SIZE: usize = 100;

struct VideoFrame {
    data: Vec<u8>,
    timestamp: u64,
}

struct VideoBuffer {
    frames: VecDeque<VideoFrame>,
}

impl VideoBuffer {
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
