use pulse::sample::{Format, Spec};
use pulse::stream::Direction;
use simple_pulse::Simple;

use std::sync::mpsc::Receiver;
use std::time::Instant;

use crate::demux::demultiplexer::RawAudioMessage;
use crate::helpers::adapter::Adapter;
use crate::helpers::structs::ContentQueue;
use crate::helpers::types::{BytesWithTimestamp, Res};

pub struct AudioAdapter {
    simple: Simple,
    buffer: ContentQueue,
    producer_rx: Receiver<RawAudioMessage>,
}

impl Adapter<RawAudioMessage> for AudioAdapter {
    fn new(el_per_second: usize, producer_rx: Receiver<RawAudioMessage>) -> Res<Self> {
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

        let buffer = ContentQueue::new(el_per_second);

        Ok(AudioAdapter {
            buffer,
            producer_rx,
            simple,
        })
    }

    fn get_buffer(&mut self) -> &mut ContentQueue {
        &mut self.buffer
    }

    fn process_element(&self, sample: BytesWithTimestamp) -> Res<()> {
        self.simple
            .write(&self.planar_to_interleaved(&sample.data))?;
        Ok(())
    }

    fn is_producer_done(&self) -> bool {
        self.producer_rx.try_recv().is_ok()
    }

    fn run(&mut self) -> Res<()> {
        let mut start_time: Instant;
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
