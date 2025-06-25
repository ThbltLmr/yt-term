use ffmpeg_next::{self as ffmpeg, frame, Packet};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::usize;

use crate::demux::moov::{parse_moov, FTYPBox};

use crate::demux::sample_data::extract_sample_data;
use crate::helpers::structs::ContentQueue;
use crate::helpers::types::Bytes;

pub struct Demultiplexer {
    pub rgb_frames_queue: Arc<Mutex<ContentQueue>>,
    pub audio_samples_queue: Arc<Mutex<ContentQueue>>,
    pub demultiplexing_done_tx: Sender<()>,
    pub video_decoder: ffmpeg::decoder::Video,
    pub audio_decoder: ffmpeg::decoder::Audio,
    pub nal_length_size: u8,
}

impl Demultiplexer {
    pub fn new(
        rgb_frames_queue: Arc<Mutex<ContentQueue>>,
        audio_samples_queue: Arc<Mutex<ContentQueue>>,
        demultiplexing_done_tx: Sender<()>,
    ) -> Self {
        let input = ffmpeg::format::input("/home/Thibault/Downloads/sample.mp4").unwrap();
        let video_context = ffmpeg::codec::context::Context::from_parameters(
            input
                .streams()
                .best(ffmpeg_next::media::Type::Video)
                .unwrap()
                .parameters(),
        )
        .unwrap();

        let video_decoder = video_context.decoder().video().unwrap();

        let audio_context = ffmpeg::codec::context::Context::from_parameters(
            input
                .streams()
                .best(ffmpeg_next::media::Type::Audio)
                .unwrap()
                .parameters(),
        )
        .unwrap();

        let audio_decoder = audio_context.decoder().audio().unwrap();

        Self {
            rgb_frames_queue,
            audio_samples_queue,
            demultiplexing_done_tx,
            video_decoder,
            audio_decoder,
            nal_length_size: 4,
        }
    }

    fn get_bit(&self, byte: u8, bit_index: u8) -> u8 {
        if bit_index >= 8 {
            panic!("Bit index out of bounds: {}", bit_index);
        }

        ((byte & (1 << bit_index)) != 0).try_into().unwrap()
    }
    fn convert_avcc_to_annexb(&self, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let mut offset = 0;

        while offset + self.nal_length_size as usize <= data.len() {
            let nal_length = match self.nal_length_size {
                1 => data[offset] as u32,
                2 => u16::from_be_bytes([data[offset], data[offset + 1]]) as u32,
                3 => u32::from_be_bytes([0, data[offset], data[offset + 1], data[offset + 2]]),
                4 => u32::from_be_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]),
                _ => break,
            };

            offset += self.nal_length_size as usize;

            if offset + nal_length as usize > data.len() {
                break;
            }

            result.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);

            result.extend_from_slice(&data[offset..offset + nal_length as usize]);
            offset += nal_length as usize;
        }
        result
    }

    pub fn demux(&mut self) {
        let mut yt_dlp_process = Command::new("yt-dlp")
            .args([
                "-o",
                "-",
                "--no-part",
                "-f",
                "18",
                "www.youtube.com/watch?v=dQw4w9WgXcQ",
            ])
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Could not start yt-dlp process");

        let mut yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

        let mut buffer = vec![0; 1000000];

        let mut accumulated_data: Vec<u8> = vec![];

        let mut ftyp_box = None;
        let mut moov_box = None;
        let mut sample_data = None;

        let mut parsed_bytes = 0;
        let mut mdat_reached = false;

        loop {
            match yt_dlp_stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    accumulated_data.extend_from_slice(&buffer[..bytes_read]);

                    if !mdat_reached {
                        while accumulated_data.len() >= 8 {
                            let box_size_bytes: [u8; 4] = [
                                accumulated_data[0],
                                accumulated_data[1],
                                accumulated_data[2],
                                accumulated_data[3],
                            ];

                            let box_size = u32::from_be_bytes(box_size_bytes);

                            let box_title_bytes: [u8; 4] = [
                                accumulated_data[4],
                                accumulated_data[5],
                                accumulated_data[6],
                                accumulated_data[7],
                            ];

                            let box_title = String::from_utf8_lossy(&box_title_bytes);

                            if box_title.to_string().as_str() != "mdat"
                                && accumulated_data.len() < box_size as usize
                            {
                                break;
                            }

                            accumulated_data.drain(..8);

                            parsed_bytes += 8;

                            match box_title.to_string().as_str() {
                                "ftyp" => {
                                    ftyp_box = Some(FTYPBox {
                                        size: box_size,
                                        data: accumulated_data
                                            .drain(..(box_size - 8) as usize)
                                            .collect(),
                                    });

                                    assert_eq!(box_size, ftyp_box.as_ref().unwrap().size);
                                    parsed_bytes += box_size - 8;
                                }
                                "moov" => {
                                    match parse_moov(
                                        box_size,
                                        accumulated_data.drain(..(box_size - 8) as usize).collect(),
                                    ) {
                                        Ok(ok_box) => {
                                            moov_box = Some(ok_box);
                                        }
                                        Err(error) => {
                                            panic!("{}", error);
                                        }
                                    }

                                    assert_eq!(box_size, moov_box.as_ref().unwrap().size);

                                    parsed_bytes += box_size - 8;

                                    for trak in &moov_box.as_ref().unwrap().traks {
                                        if let Some(ref avcc_data) = trak.media.minf.stbl.stsd.avcc
                                        {
                                            self.nal_length_size = self.get_bit(avcc_data[4], 0)
                                                + self.get_bit(avcc_data[4], 1) * 2
                                                + 1;
                                            println!(
                                                "Setting nal_length_size to {}",
                                                self.nal_length_size
                                            );
                                        }
                                    }

                                    sample_data =
                                        Some(extract_sample_data(moov_box.unwrap()).unwrap());
                                }
                                "mdat" => {
                                    if ftyp_box.is_none() {
                                        println!("We are f'ed in the B by ftyp");
                                    }
                                    if sample_data.clone().is_none() {
                                        println!("We are f'ed in the B by moov");
                                    }

                                    println!("Bytes parsed for moov and ftyp: {}", parsed_bytes);

                                    mdat_reached = true;

                                    break;
                                }
                                _ => {
                                    panic!(
                                        "So this is new, we got a {} box",
                                        box_title.to_string()
                                    );
                                }
                            }
                        }
                    }

                    if sample_data.is_some() {
                        while !sample_data.as_ref().unwrap().is_empty()
                            && accumulated_data.len()
                                >= sample_data.as_ref().unwrap().front().unwrap().size as usize
                        {
                            let current_sample_data =
                                sample_data.as_mut().unwrap().pop_front().unwrap();

                            println!(
                                "Sample data len: {}, is_video: {:?}",
                                current_sample_data.size, current_sample_data.is_video
                            );

                            println!("Current offset: {parsed_bytes}");

                            let sample: Bytes = accumulated_data
                                .drain(..current_sample_data.size as usize)
                                .collect();

                            if current_sample_data.is_video {
                                let annexb_data = self.convert_avcc_to_annexb(&sample);
                                if !annexb_data.is_empty() {
                                    println!("We here");
                                    let packet = Packet::copy(&annexb_data);

                                    match self.video_decoder.send_packet(&packet) {
                                        Ok(_) => {
                                            println!("Send video frame ok");
                                            let mut frame = frame::Video::empty();
                                            while self
                                                .video_decoder
                                                .receive_frame(&mut frame)
                                                .is_ok()
                                            {
                                                let data = frame.data(0);

                                                self.rgb_frames_queue
                                                    .lock()
                                                    .unwrap()
                                                    .push_el(data.to_vec());

                                                println!(
                                                    "Decoded video frame, queue len: {}",
                                                    self.rgb_frames_queue.lock().unwrap().len()
                                                );

                                                frame = frame::Video::empty();
                                            }
                                        }
                                        Err(e) => {
                                            println!("Failed to send video packet: {:?}", e);
                                        }
                                    }
                                }
                            } else {
                                let packet = Packet::copy(&sample);

                                match self.audio_decoder.send_packet(&packet) {
                                    Ok(_) => {
                                        let mut frame = frame::Audio::empty();
                                        while self.audio_decoder.receive_frame(&mut frame).is_ok() {
                                            let data = frame.data(0);
                                            self.audio_samples_queue
                                                .lock()
                                                .unwrap()
                                                .push_el(data.to_vec());

                                            println!("Audio frame size{}", data.len());
                                            println!(
                                                "Decoded audio frame, queue len: {}",
                                                self.audio_samples_queue.lock().unwrap().len()
                                            );

                                            frame = frame::Audio::empty();
                                        }
                                    }
                                    Err(e) => {
                                        println!("Failed to send audio packet: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from yt-dlp: {}", e);
                }
            }
        }

        self.demultiplexing_done_tx.send(()).unwrap();
    }
}
