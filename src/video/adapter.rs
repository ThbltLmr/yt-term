use std::io::{self, Write};
use std::sync::mpsc::Receiver;

use crate::helpers::adapter::Adapter;
use crate::helpers::structs::ContentQueue;
use crate::helpers::types::{BytesWithTimestamp, Res};

pub struct TerminalAdapter {
    buffer: ContentQueue,
    producer_done_rx: Receiver<()>,
}

impl Adapter for TerminalAdapter {
    fn new(buffer: ContentQueue, producer_done_rx: Receiver<()>) -> Res<Self> {
        Ok(TerminalAdapter {
            buffer,
            producer_done_rx,
        })
    }

    fn get_buffer(&self) -> ContentQueue {
        self.buffer.clone()
    }

    fn is_producer_done(&self) -> bool {
        self.producer_done_rx.try_recv().is_ok()
    }

    fn process_element(&self, frame: BytesWithTimestamp) -> Res<()> {
        let mut stdout = io::stdout();

        let reset_cursor = b"\x1B[H";
        let mut buffer = vec![];

        buffer.extend_from_slice(reset_cursor);
        buffer.extend_from_slice(&frame.data);

        stdout.write_all(&buffer)?;
        stdout.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn video_adapter_creation() {
        let (_tx, rx) = mpsc::channel();
        let encoded_buffer = ContentQueue::new(30);
        let display_manager = TerminalAdapter::new(encoded_buffer.clone(), rx);

        assert!(display_manager.is_ok());
    }
}
