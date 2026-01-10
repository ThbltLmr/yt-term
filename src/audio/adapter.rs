use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleRate, StreamConfig};

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::demux::demultiplexer::RawAudioMessage;
use crate::helpers::types::{BytesWithTimestamp, Res};

pub struct AudioAdapter {
    producer_rx: Receiver<RawAudioMessage>,
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl AudioAdapter {
    pub fn new(producer_rx: Receiver<RawAudioMessage>) -> Res<Self> {
        let audio_buffer = Arc::new(Mutex::new(VecDeque::new()));

        Ok(AudioAdapter {
            producer_rx,
            audio_buffer,
            cancel_flag: None,
        })
    }

    pub fn set_cancel_flag(&mut self, flag: Arc<AtomicBool>) {
        self.cancel_flag = Some(flag);
    }

    fn process_element(&self, sample: BytesWithTimestamp) -> Res<()> {
        let interleaved_data = self.planar_to_interleaved(&sample.data);
        let float_samples: Vec<f32> = interleaved_data
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        
        let mut buffer = self.audio_buffer.lock().unwrap();
        buffer.extend(float_samples);
        Ok(())
    }

    pub fn run(&mut self) -> Res<()> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("No output device available")?;
        
        let config = StreamConfig {
            channels: 2,
            sample_rate: SampleRate(44100),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer_clone = Arc::clone(&self.audio_buffer);
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buffer = buffer_clone.lock().unwrap();
                for sample in data.iter_mut() {
                    *sample = buffer.pop_front().unwrap_or(0.0);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;
        
        stream.play()?;
        
        let mut start_time = Instant::now();
        let mut started_playing = false;

        loop {
            if let Some(ref flag) = self.cancel_flag {
                if flag.load(Ordering::SeqCst) {
                    return Ok(());
                }
            }

            match self.producer_rx.recv_timeout(Duration::from_millis(16)) {
                Ok(message) => match message {
                    RawAudioMessage::AudioMessage(sample) => {
                        if !started_playing {
                            started_playing = true;
                            start_time = Instant::now();
                        }

                        if sample.timestamp_in_ms > start_time.elapsed().as_millis() as usize {
                            thread::sleep(Duration::from_millis(
                                (sample.timestamp_in_ms - start_time.elapsed().as_millis() as usize)
                                    as u64,
                            ));
                        }

                        self.process_element(sample).unwrap();
                    }
                    RawAudioMessage::Done => {
                        return Ok(());
                    }
                },
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
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
