use pulse::sample::{Format, Spec};
use pulse::stream::Direction;
use simple_pulse::Simple;

use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use crate::demux::demultiplexer::RawAudioMessage;
use crate::helpers::types::{BytesWithTimestamp, Res};

pub struct AudioAdapter {
    simple: Simple,
    producer_rx: Receiver<RawAudioMessage>,
}

impl AudioAdapter {
    fn newp(producer_rx: Receiver<RawAudioMessage>) -> Res<Self> {
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
            producer_rx,
            simple,
        })
    }

    fn process_element(&self, sample: BytesWithTimestamp) -> Res<()> {
        self.simple
            .write(&self.planar_to_interleaved(&sample.data))?;
        Ok(())
    }

    fn run(&mut self) -> Res<()> {
        let mut start_time: Instant;
        let mut started_playing = false;

        loop {
            match self.producer_rx.try_recv() {
                Ok(message) => match message {
                    RawAudioMessage::AudioMessage(sample) => {
                        if !started_playing {
                            started_playing = true;
                            start_time = Instant::now();
                        }
                        if sample.timestamp_in_ms > start_time.elapsed().as_millis() as usize {
                            thread::sleep(Duration::from_millis(
                                sample.timestamp_in_ms - start_time.elapsed().as_millis(),
                            ));
                        }

                        self.process_element(sample);
                    }
                    RawAudioMessage::Done => {}
                },
                Err(_) => {}
            }
        }
    }

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
