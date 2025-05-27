use std::io::{self, Write};
use std::sync::mpsc;

use crate::helpers::structs::{RingBuffer, Sample};
use crate::helpers::types::Res;
use crate::{Arc, Mutex};

pub struct AudioAdapter {
    sample_interval: u64,
    audio_buffer: Arc<Mutex<RingBuffer<Sample>>>,
    audio_queueing_done_rx: mpsc::Receiver<()>,
}

impl AudioAdapter {
    pub fn new(
        sample_interval: u64,
        audio_buffer: Arc<Mutex<RingBuffer<Sample>>>,
        audio_queueing_done_rx: mpsc::Receiver<()>,
    ) -> Res<Self> {
        Ok(AudioAdapter {
            sample_interval,
            audio_buffer,
            audio_queueing_done_rx,
        })
    }

    fn play_sample(&self, sample: Sample) -> Res<()> {
        Ok(())
    }

    pub fn play(&self) -> Res<()> {
        let now = std::time::Instant::now();

        loop {
            if self.audio_buffer.lock().unwrap().len() == 0 {
                if self.audio_queueing_done_rx.try_recv().is_ok() {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            } else {
                let audio_sample = self.audio_buffer.lock().unwrap().get_el();
                if let Some(sample) = audio_sample {
                    if std::time::Instant::now().duration_since(now).as_millis()
                        >= self.sample_interval as u128
                    {
                        self.play_sample(sample)?;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_display_manager() {
        let (tx, rx) = mpsc::channel();
        let encoded_buffer = Arc::new(Mutex::new(RingBuffer::new(25)));
        let display_manager = AudioAdapter::new(25, encoded_buffer.clone(), rx).unwrap();

        let sample = Sample {
            data: vec![1, 2, 3],
        };

        encoded_buffer.lock().unwrap().push_el(sample);

        std::thread::spawn(move || {
            display_manager.play().unwrap();
        });

        std::thread::sleep(std::time::Duration::from_secs(1));

        tx.send(()).unwrap();

        assert_eq!(
            encoded_buffer.lock().unwrap().len(),
            0,
            "Buffer should be empty after sending done signal",
        );
    }
}
