use std::{
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Duration,
};

use super::{
    structs::ContentQueue,
    types::{Bytes, Res},
};

pub trait Adapter {
    fn new(
        interval: Duration,
        buffer: Arc<Mutex<ContentQueue>>,
        producer_done_rx: Receiver<()>,
    ) -> Res<Self>
    where
        Self: Sized;

    fn process_element(&self, element: Bytes) -> Res<()>;

    fn get_buffer(&self) -> Arc<Mutex<ContentQueue>>;

    fn is_buffer_empty(&self) -> bool {
        self.get_buffer().lock().unwrap().is_empty()
    }

    fn get_buffer_element(&self) -> Option<Bytes> {
        self.get_buffer().lock().unwrap().get_el()
    }

    fn get_interval(&self) -> Duration;

    fn is_producer_done(&self) -> bool;

    fn get_interval_plus_five_percent(&self) -> Duration {
        Duration::from_millis((1.05 * self.get_interval().as_millis() as f64) as u64)
    }

    fn run(&self) -> Res<()> {
        let mut last_element_processing_time = std::time::Instant::now();
        let mut total_elements_processed = 0;
        loop {
            if self.is_buffer_empty() {
                if self.is_producer_done() {
                    return Ok(());
                }
            } else if last_element_processing_time.elapsed() >= self.get_interval() {
                if let Some(element) = self.get_buffer_element() {
                    if total_elements_processed > 0
                        && last_element_processing_time.elapsed()
                            > self.get_interval_plus_five_percent()
                    {
                        last_element_processing_time += self.get_interval();
                        total_elements_processed += 1;
                        continue;
                    }

                    last_element_processing_time = std::time::Instant::now();
                    total_elements_processed += 1;

                    self.process_element(element)?;
                }
            }
        }
    }
}
