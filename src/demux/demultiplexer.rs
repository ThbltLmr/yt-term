use ffmpeg_next as ffmpeg;
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

        let audio_codec = ffmpeg::codec::decoder::find(ffmpeg_next::codec::Id::AAC).unwrap();
        let audio_context = ffmpeg::codec::context::Context::new_with_codec(audio_codec);
        let audio_decoder = audio_context.decoder().audio().unwrap();

        Self {
            rgb_frames_queue,
            audio_samples_queue,
            demultiplexing_done_tx,
            video_decoder,
            audio_decoder,
        }
    }

    pub fn demux(&self) {
        ffmpeg::init().unwrap();

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
                                        moov_box = Some(ok_box);
                                    }
                                    Err(error) => {
                                        panic!("{}", error);
                                    }
                                }

                                println!(
                                    "Moov box parsed with {} streams",
                                    moov_box.as_ref().unwrap().traks.len(),
                                );

                                sample_data = Some(extract_sample_data(moov_box.unwrap()).unwrap());

                                println!("Got {:?} samples", sample_data.as_ref().unwrap().len());
                            }
                            "mdat" => {
                                if ftyp_box.is_none() {
                                    println!("We are f'ed in the B by ftyp");
                                }
                                if sample_data.clone().is_none() {
                                    println!("We are f'ed in the B by moov");
                                }
                                println!("This is where the fun begins");
                                break;
                            }
                            _ => {
                                println!("So this is new, we got a {} box", box_title.to_string());
                            }
                        }
                    }
                    if sample_data.is_some() {
                        while accumulated_data.len()
                            >= sample_data.as_ref().unwrap().front().unwrap().0 as usize
                        {
                            let current_sample_data =
                                sample_data.as_mut().unwrap().pop_front().unwrap();

                            let sample: Bytes = accumulated_data
                                .drain(..current_sample_data.0 as usize)
                                .collect();

                            if current_sample_data.1 {
                                // send to video decoder
                                // add result to ContentQueue
                                println!("Adding {} bytes to video queue", sample.len());
                            } else {
                                // send to audio decoder
                                // add result to ContentQueue
                                println!("Adding {} bytes to audio queue", sample.len());
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
