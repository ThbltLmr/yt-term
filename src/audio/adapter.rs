use pulse::sample::{Format, Spec};
use pulse::stream::Direction;
use simple_pulse::Simple;

use std::sync::mpsc::Receiver;

use crate::helpers::adapter::Adapter;
use crate::helpers::structs::ContentQueue;
use crate::helpers::types::{BytesWithTimestamp, Res};
use crate::{Arc, Mutex};

pub struct AudioAdapter {
    simple: Simple,
    buffer: Arc<Mutex<ContentQueue>>,
    producer_done_rx: Receiver<()>,
}

impl Adapter for AudioAdapter {
    fn new(buffer: Arc<Mutex<ContentQueue>>, producer_done_rx: Receiver<()>) -> Res<Self> {
        let spec = Spec {
            format: Format::F32le,
            channels: 2,
            rate: 44100,
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
            buffer,
            producer_done_rx,
            simple,
        })
    }

    fn get_buffer(&self) -> Arc<Mutex<ContentQueue>> {
        self.buffer.clone()
    }

    fn process_element(&self, sample: BytesWithTimestamp) -> Res<()> {
        self.simple
            .write(&self.planar_to_interleaved(&sample.data))?;
        Ok(())
    }

    fn is_producer_done(&self) -> bool {
        self.producer_done_rx.try_recv().is_ok()
    }
}

impl AudioAdapter {
    fn planar_to_interleaved(&self, planar_data: &[u8]) -> Vec<u8> {
        let samples_per_channel = planar_data.len() / (4 * 2);
        let mut left_channel = Vec::with_capacity(samples_per_channel);
        let mut right_channel = Vec::with_capacity(samples_per_channel);

        for chunk in planar_data[..planar_data.len() / 2].chunks_exact(4) {
            let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            left_channel.push(sample);
        }

        for chunk in planar_data[planar_data.len() / 2..].chunks_exact(4) {
            let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            right_channel.push(sample);
        }

        let mut interleaved = Vec::with_capacity(planar_data.len());
        for (left, right) in left_channel.iter().zip(right_channel.iter()) {
            interleaved.extend_from_slice(&left.to_le_bytes());
            interleaved.extend_from_slice(&right.to_le_bytes());
        }

        interleaved
    }
}
