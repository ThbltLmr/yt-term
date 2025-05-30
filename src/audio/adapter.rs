use pulse::sample::{Format, Spec};
use pulse::stream::Direction;
use simple_pulse::Simple;

use std::sync::mpsc;
use std::time::Duration;

use crate::helpers::structs::{RingBuffer, Sample};
use crate::helpers::types::Res;
use crate::{Arc, Mutex};

pub struct AudioAdapter {
    simple: Simple,
    sample_interval: Duration,
    audio_buffer: Arc<Mutex<RingBuffer<Sample>>>,
    audio_queueing_done_rx: mpsc::Receiver<()>,
}

impl AudioAdapter {
    pub fn new(
        sample_interval_ms: usize,
        audio_buffer: Arc<Mutex<RingBuffer<Sample>>>,
        audio_queueing_done_rx: mpsc::Receiver<()>,
    ) -> Res<Self> {
        let spec = Spec {
            format: Format::S16le, // 16-bit signed little endian
            channels: 2,           // stereo
            rate: 48000,           // 48kHz sample rate
        };

        let simple = Simple::new(
            None,
            "AudioAdapter",
            Direction::Playback,
            None,
            "Audio Playback",
            &spec,
            None,
            None,
        )?;

        Ok(AudioAdapter {
            simple,
            sample_interval: Duration::from_millis(sample_interval_ms as u64),
            audio_buffer,
            audio_queueing_done_rx,
        })
    }

    fn play_sample(&self, sample: Sample) -> Res<()> {
        self.simple.write(&sample.data)?;
        Ok(())
    }

    pub fn play(&self) -> Res<()> {
        let mut last_sample_time = std::time::Instant::now();

        loop {
            if self.audio_buffer.lock().unwrap().len() == 0 {
                if self.audio_queueing_done_rx.try_recv().is_ok() {
                    return Ok(());
                }
            } else if last_sample_time.elapsed() >= self.sample_interval {
                let audio_sample = self.audio_buffer.lock().unwrap().get_el();
                if let Some(sample) = audio_sample {
                    last_sample_time = std::time::Instant::now();
                    self.play_sample(sample)?;
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
    fn audio_adapter_creation_and_play() {
        let (_tx, rx) = mpsc::channel();
        let encoded_buffer = Arc::new(Mutex::new(RingBuffer::new(30)));

        let display_manager = AudioAdapter::new(30, encoded_buffer.clone(), rx);
        assert!(display_manager.is_ok());
    }
}
