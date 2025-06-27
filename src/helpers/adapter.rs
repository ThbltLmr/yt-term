use std::{
    sync::{mpsc::Receiver, Arc, Mutex},
    thread,
    time::{Duration, Instant},
    usize,
};

use super::{
    structs::ContentQueue,
    types::{BytesWithTimestamp, Res},
};

pub trait Adapter {
    fn new(
        interval: Duration,
        buffer: Arc<Mutex<ContentQueue>>,
        producer_done_rx: Receiver<()>,
    ) -> Res<Self>
    where
        Self: Sized;

    fn process_element(&self, element: BytesWithTimestamp) -> Res<()>;

    fn get_buffer(&self) -> Arc<Mutex<ContentQueue>>;

    fn is_buffer_empty(&self) -> bool {
        self.get_buffer().lock().unwrap().is_empty()
    }

    fn get_buffer_element(&self) -> Option<BytesWithTimestamp> {
        self.get_buffer().lock().unwrap().get_el()
    }

    fn get_interval(&self) -> Duration;

    fn is_producer_done(&self) -> bool;

    fn run(&self) -> Res<()> {
        let mut start_time = Instant::now();
        let mut started_playing = false;

        loop {
            if self.is_buffer_empty() {
                if self.is_producer_done() {
                    return Ok(());
                }
            } else if let Some(element) = self.get_buffer_element() {
                if !started_playing {
                    start_time = Instant::now();
                    started_playing = true;
                }

                while element.timestamp_in_ms > start_time.elapsed().as_millis() as usize {
                    thread::sleep(Duration::from_millis(1));
                }

                if element.timestamp_in_ms + 5 <= start_time.elapsed().as_millis() as usize {
                    continue;
                }

                assert!(
                    element
                        .timestamp_in_ms
                        .abs_diff(start_time.elapsed().as_millis() as usize)
                        < 5
                );

                self.process_element(element)?;
            }
        }
    }
}
