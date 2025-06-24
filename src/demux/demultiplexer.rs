use ffmpeg_next::{self as ffmpeg, decoder, frame, Packet};
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
        let video_codec = ffmpeg::codec::decoder::find(ffmpeg_next::codec::Id::H264).unwrap();
        let video_context = ffmpeg::codec::context::Context::new_with_codec(video_codec);
        let video_decoder = video_context.decoder().video().unwrap();

        println!("{:?}", video_decoder.id());

        let audio_codec = ffmpeg::codec::decoder::find(ffmpeg_next::codec::Id::AAC).unwrap();
        let audio_context = ffmpeg::codec::context::Context::new_with_codec(audio_codec);
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

    // Extract SPS and PPS from avcC box and send to decoder
    fn configure_video_decoder(
        &mut self,
        avcc_data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if avcc_data.len() < 7 {
            return Err("avcC data too short".into());
        }

        // Parse avcC box structure
        let _configuration_version = avcc_data[0];
        let _avc_profile_indication = avcc_data[1];
        let _profile_compatibility = avcc_data[2];
        let _avc_level_indication = avcc_data[3];

        // Extract NAL unit length size
        self.nal_length_size = (avcc_data[4] & 0x03) + 1;

        // Number of SPS NAL units
        let num_sps = avcc_data[5] & 0x1F;
        let mut offset = 6;

        let mut extradata = Vec::new();

        // Extract SPS
        for _ in 0..num_sps {
            if offset + 2 > avcc_data.len() {
                return Err("Invalid avcC: not enough data for SPS length".into());
            }

            let sps_length =
                u16::from_be_bytes([avcc_data[offset], avcc_data[offset + 1]]) as usize;
            offset += 2;

            if offset + sps_length > avcc_data.len() {
                return Err("Invalid avcC: not enough data for SPS".into());
            }

            // Add start code
            extradata.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            extradata.extend_from_slice(&avcc_data[offset..offset + sps_length]);
            offset += sps_length;
        }

        // Number of PPS NAL units
        if offset >= avcc_data.len() {
            return Err("Invalid avcC: no PPS data".into());
        }

        let num_pps = avcc_data[offset];
        offset += 1;

        // Extract PPS
        for _ in 0..num_pps {
            if offset + 2 > avcc_data.len() {
                return Err("Invalid avcC: not enough data for PPS length".into());
            }

            let pps_length =
                u16::from_be_bytes([avcc_data[offset], avcc_data[offset + 1]]) as usize;
            offset += 2;

            if offset + pps_length > avcc_data.len() {
                return Err("Invalid avcC: not enough data for PPS".into());
            }

            // Add start code
            extradata.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            extradata.extend_from_slice(&avcc_data[offset..offset + pps_length]);
            offset += pps_length;
        }

        // Send SPS/PPS as a packet to the decoder
        if !extradata.is_empty() {
            let packet = Packet::copy(&extradata);
            match self.video_decoder.send_packet(&packet) {
                Ok(_) => {
                    println!("Successfully sent SPS/PPS to decoder");
                }
                Err(e) => {
                    println!("Failed to send SPS/PPS: {:?}", e);
                }
            }
        }

        Ok(())
    }

    // Convert AVCC format to Annex B format (add start codes)
    fn convert_avcc_to_annexb(&self, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let mut offset = 0;

        while offset + self.nal_length_size as usize <= data.len() {
            // Read NAL unit length
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

            // Add start code
            result.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);

            // Add NAL unit data
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
                "https://www.youtube.com/watch?v=kFsXTaoP2A4",
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

        loop {
            match yt_dlp_stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    accumulated_data.extend_from_slice(&buffer[..bytes_read]);

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

                        match box_title.to_string().as_str() {
                            "ftyp" => {
                                ftyp_box = Some(FTYPBox {
                                    size: box_size,
                                    data: accumulated_data
                                        .drain(..(box_size - 8) as usize)
                                        .collect(),
                                });
                            }
                            "moov" => {
                                match parse_moov(
                                    box_size,
                                    accumulated_data.drain(..(box_size - 8) as usize).collect(),
                                ) {
                                    Ok(ok_box) => {
                                        for trak in &ok_box.traks {
                                            if let Some(ref avcc_data) =
                                                trak.media.minf.stbl.stsd.avcc
                                            {
                                                if let Err(e) =
                                                    self.configure_video_decoder(avcc_data)
                                                {
                                                    println!(
                                                        "Failed to configure video decoder: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        moov_box = Some(ok_box);
                                    }
                                    Err(error) => {
                                        panic!("{}", error);
                                    }
                                }

                                sample_data = Some(extract_sample_data(moov_box.unwrap()).unwrap());
                            }
                            "mdat" => {
                                if ftyp_box.is_none() {
                                    println!("We are f'ed in the B by ftyp");
                                }
                                if sample_data.clone().is_none() {
                                    println!("We are f'ed in the B by moov");
                                }
                                break;
                            }
                            _ => {
                                println!("So this is new, we got a {} box", box_title.to_string());
                            }
                        }
                    }
                    if sample_data.is_some() {
                        while !sample_data.as_ref().unwrap().is_empty()
                            && accumulated_data.len()
                                >= sample_data.as_ref().unwrap().front().unwrap().0 as usize
                        {
                            let current_sample_data =
                                sample_data.as_mut().unwrap().pop_front().unwrap();

                            let sample: Bytes = accumulated_data
                                .drain(..current_sample_data.0 as usize)
                                .collect();

                            if current_sample_data.1 {
                                // Video sample - convert from AVCC to Annex B format
                                let annexb_data = self.convert_avcc_to_annexb(&sample);

                                if !annexb_data.is_empty() {
                                    let packet = Packet::copy(&annexb_data);

                                    match self.video_decoder.send_packet(&packet) {
                                        Ok(_) => {
                                            // Try to receive frames
                                            let mut frame = frame::Video::empty();
                                            while self
                                                .video_decoder
                                                .receive_frame(&mut frame)
                                                .is_ok()
                                            {
                                                // Convert frame to RGB if needed
                                                // For now, just store the raw frame data
                                                let data = frame.data(0);

                                                self.rgb_frames_queue
                                                    .lock()
                                                    .unwrap()
                                                    .push_el(data.to_vec());

                                                println!(
                                                    "Decoded video frame, queue len: {}",
                                                    self.rgb_frames_queue.lock().unwrap().len()
                                                );

                                                frame = frame::Video::empty(); // Reset for next frame
                                            }
                                        }
                                        Err(e) => {
                                            //                                            println!("Failed to send video packet: {:?}", e);
                                        }
                                    }
                                }
                            } else {
                                // Audio sample - AAC decoding
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

                                            println!(
                                                "Decoded audio frame, queue len: {}",
                                                self.audio_samples_queue.lock().unwrap().len()
                                            );

                                            frame = frame::Audio::empty(); // Reset for next frame
                                        }
                                    }
                                    Err(e) => {
                                        //                                       println!("Failed to send audio packet: {:?}", e);
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
