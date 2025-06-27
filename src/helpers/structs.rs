use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};

use super::types::{BytesWithTimestamp, Res};

pub struct ContentQueue {
    elements: VecDeque<BytesWithTimestamp>,
    el_per_second: usize,
}

impl ContentQueue {
    pub fn new(el_per_second: usize) -> Self {
        ContentQueue {
            elements: VecDeque::new(),
            el_per_second,
        }
    }

    pub fn has_one_second_ready(&self) -> bool {
        self.elements.len() >= self.el_per_second
    }

    pub fn queue_one_second_into(&mut self, queue: Arc<Mutex<ContentQueue>>) {
        let mut queue = queue.lock().unwrap();
        let elements = self.pop_one_second();
        queue.push_elements(elements);
    }

    pub fn push_el(&mut self, element: BytesWithTimestamp) {
        self.elements.push_back(element);
    }

    fn push_elements(&mut self, elements: Vec<BytesWithTimestamp>) {
        for element in elements {
            self.elements.push_back(element);
        }
    }

    pub fn pop_one_second(&mut self) -> Vec<BytesWithTimestamp> {
        let mut elements = Vec::new();
        for _ in 0..self.el_per_second {
            if let Some(el) = self.elements.pop_front() {
                elements.push(el);
            } else {
                break;
            }
        }
        elements
    }

    pub fn get_el(&mut self) -> Option<BytesWithTimestamp> {
        self.elements.pop_front()
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn bytes_len(&self) -> usize {
        self.elements.iter().map(|el| el.data.len()).sum()
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
    fn ring_buffer_push_and_get() {
        let mut buffer = ContentQueue::new(1);
        assert_eq!(buffer.len(), 0);

        let frame1 = BytesWithTimestamp {
            data: vec![1, 2, 3],
            timestamp_in_ms: 1000,
        };

        let frame2 = BytesWithTimestamp {
            data: vec![4, 5, 6],
            timestamp_in_ms: 2000,
        };

        buffer.push_el(frame1);
        buffer.push_el(frame2);

        assert_eq!(buffer.len(), 2);

        let retrieved_frame = buffer.get_el().unwrap();
        assert_eq!(retrieved_frame.data, vec![1, 2, 3]);

        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn ring_buffer_pop_one_second() {
        let mut buffer = ContentQueue::new(2);
        buffer.push_el(BytesWithTimestamp {
            data: vec![1, 2, 3],
            timestamp_in_ms: 1000,
        });
        buffer.push_el(BytesWithTimestamp {
            data: vec![4, 5, 6],
            timestamp_in_ms: 1000,
        });
        buffer.push_el(BytesWithTimestamp {
            data: vec![7, 8, 9],
            timestamp_in_ms: 1000,
        });

        let popped = buffer.pop_one_second();
        assert_eq!(popped.len(), 2);
        assert_eq!(popped[0].data, vec![1, 2, 3]);
        assert_eq!(popped[1].data, vec![4, 5, 6]);
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn ring_buffer_has_one_second_ready() {
        let mut buffer = ContentQueue::new(2);
        assert!(!buffer.has_one_second_ready());

        buffer.push_el(BytesWithTimestamp {
            data: vec![1, 2, 3],
            timestamp_in_ms: 1000,
        });

        assert!(!buffer.has_one_second_ready());

        buffer.push_el(BytesWithTimestamp {
            data: vec![4, 5, 6],
            timestamp_in_ms: 1000,
        });

        assert!(buffer.has_one_second_ready());
    }

    #[test]
    fn test_screen_guard() {
        let guard = ScreenGuard::new();
        assert!(guard.is_ok());
    }
}
