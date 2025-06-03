use pulse::sample::{Format, Spec};
use pulse::stream::Direction;
use simple_pulse::Simple;

use std::sync::mpsc::Receiver;
use std::time::Duration;

use crate::helpers::adapter::Adapter;
use crate::helpers::structs::ContentQueue;
use crate::helpers::types::{Bytes, Res};
use crate::{Arc, Mutex};

pub struct AudioAdapter {
    simple: Simple,
    interval: Duration,
    buffer: Arc<Mutex<ContentQueue<Bytes>>>,
    producer_done_rx: Receiver<()>,
}

impl Adapter for AudioAdapter {
    fn new(
        interval: Duration,
        buffer: Arc<Mutex<ContentQueue<Bytes>>>,
        producer_done_rx: Receiver<()>,
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
            interval,
            buffer,
            producer_done_rx,
            simple,
        })
    }

    fn get_buffer(&self) -> Arc<Mutex<ContentQueue<Bytes>>> {
        self.buffer.clone()
    }

    fn get_interval(&self) -> Duration {
        self.interval
    }

    fn process_element(&self, sample: Bytes) -> Res<()> {
        self.simple.write(&sample)?;
        Ok(())
    }

    fn is_producer_done(&self) -> bool {
        self.producer_done_rx.try_recv().is_ok()
    }
}
