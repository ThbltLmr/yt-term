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

pub struct Sample {
    pub data: Vec<u8>,
    pub start_timestamp: u64,
    pub end_timestamp: u64,
}

impl Sample {
    pub fn new(data: Vec<u8>, start_timestamp: u64, end_timestamp: u64) -> Self {
        Sample {
            data,
            start_timestamp,
            end_timestamp,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut buffer = RingBuffer::new();
        assert_eq!(buffer.len(), 0);

        let frame1 = Frame::new(vec![1, 2, 3], 123456789);
        let frame2 = Frame::new(vec![4, 5, 6], 987654321);

        buffer.push_frame(frame1);
        buffer.push_frame(frame2);

        assert_eq!(buffer.len(), 2);

        let retrieved_frame = buffer.get_frame().unwrap();
        assert_eq!(retrieved_frame.data, vec![1, 2, 3]);
        assert_eq!(retrieved_frame.timestamp, 123456789);

        assert_eq!(buffer.len(), 1);
    }
}
