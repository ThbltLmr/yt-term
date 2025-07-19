use std::io::{self, Write};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use crate::helpers::types::{BytesWithTimestamp, Res};

use super::encoder::EncodedVideoMessage;

pub struct TerminalAdapter {
    producer_rx: Receiver<EncodedVideoMessage>,
}

impl TerminalAdapter {
    pub fn new(producer_rx: Receiver<EncodedVideoMessage>) -> Res<Self> {
        Ok(TerminalAdapter { producer_rx })
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

    pub fn run(&mut self) -> Res<()> {
        let mut start_time = Instant::now();
        let mut started_playing = false;

        loop {
            match self.producer_rx.try_recv() {
                Ok(message) => match message {
                    EncodedVideoMessage::EncodedVideoMessage(frame) => {
                        if !started_playing {
                            started_playing = true;
                            start_time = Instant::now();
                        }

                        if frame.timestamp_in_ms > start_time.elapsed().as_millis() as usize {
                            thread::sleep(Duration::from_millis(
                                (frame.timestamp_in_ms - start_time.elapsed().as_millis() as usize)
                                    as u64,
                            ));
                        }

                        self.process_element(frame).unwrap();
                    }
                    EncodedVideoMessage::Done => {}
                },
                Err(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn video_adapter_creation() {
        let (_tx, rx) = mpsc::channel();
        let display_manager = TerminalAdapter::new(rx);

        assert!(display_manager.is_ok());
    }
}
