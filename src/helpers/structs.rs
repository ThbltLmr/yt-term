use std::collections::VecDeque;
use std::io::Write;

use super::types::Res;

pub struct Frame {
    pub data: Vec<u8>,
}

impl Frame {
    pub fn new(data: Vec<u8>) -> Self {
        Frame { data }
    }
}

pub struct Sample {
    pub data: Vec<u8>,
}

impl Sample {
    pub fn new(data: Vec<u8>) -> Self {
        Sample { data }
    }
}

pub struct RingBuffer<T> {
    elements: VecDeque<T>,
}

impl<T> RingBuffer<T> {
    pub fn new() -> Self {
        RingBuffer {
            elements: VecDeque::new(),
        }
    }

    pub fn push_el(&mut self, element: T) {
        self.elements.push_back(element);
    }

    pub fn get_el(&mut self) -> Option<T> {
        self.elements.pop_front()
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

pub struct ScreenGuard {}

impl ScreenGuard {
    pub fn new() -> Res<Self> {
        let mut stdout = std::io::stdout();
        let alternate_screen = b"\x1B[?1049h";

        stdout.write_all(alternate_screen)?;
        stdout.flush()?;
        Ok(ScreenGuard {})
    }
}

impl Drop for ScreenGuard {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();

        let mut buffer = vec![];
        let reset = b"\x1B[?1049l";
        let clear = b"\x1b[2J";
        let cursor = b"\x1b[H";
        buffer.extend_from_slice(reset);
        buffer.extend_from_slice(clear);
        buffer.extend_from_slice(cursor);

        stdout.write_all(&buffer).unwrap();
        stdout.flush().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut buffer = RingBuffer::new();
        assert_eq!(buffer.len(), 0);

        let frame1 = Frame::new(vec![1, 2, 3]);
        let frame2 = Frame::new(vec![4, 5, 6]);

        buffer.push_el(frame1);
        buffer.push_el(frame2);

        assert_eq!(buffer.len(), 2);

        let retrieved_frame = buffer.get_el().unwrap();
        assert_eq!(retrieved_frame.data, vec![1, 2, 3]);

        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_screen_guard() {
        let guard = ScreenGuard::new();
        assert!(guard.is_ok());
    }
}
