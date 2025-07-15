use ffmpeg_next::{self as ffmpeg, frame, Packet};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::usize;

use crate::demux::get_moov_box::{get_moov_box, FTYPBox, MOOVBox, Streams};

use crate::demux::get_sample_map::get_sample_map;
use crate::helpers::types::BytesWithTimestamp;

use super::get_sample_map::SampleMap;

pub enum RawAudioMessage {
    AudioMessage(BytesWithTimestamp),
    Done,
}

pub enum RawVideoMessage {
    VideoMessage(BytesWithTimestamp),
    FramesPerSecond(usize),
    Done,
}

pub struct Demultiplexer {
    pub url: String,
    pub raw_video_message_tx: Sender<RawVideoMessage>,
    pub raw_audio_message_tx: Sender<RawAudioMessage>,
    pub video_decoder: ffmpeg::decoder::Video,
    pub audio_decoder: ffmpeg::decoder::Audio,
    pub nal_length_size: u8,
    frame_interval_ms: Option<usize>,
    sample_interval_ms: usize,
}

impl Demultiplexer {
    pub fn new(
        raw_video_message_tx: Sender<RawVideoMessage>,
        raw_audio_message_tx: Sender<RawAudioMessage>,
        url: String,
        sample_interval_ms: usize,
    ) -> Self {
        /*
         * This is a very hacky fix to initiate an AVContext.
         * The simple-ffmpeg does not provide a safe way to instantiate a Context struct from its
         * properties. It only provides a way to instantiate a context from an input file.
         * Here, we use a minimal sample file (1 second video) downloaded from YouTube as an example of AVContext. We are
         * assuming that all MP4 files from YouTube will have the same properties.
         */
        // TODO: Write unsafe block to create AVContext from moov data
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
            raw_video_message_tx,
            raw_audio_message_tx,
            video_decoder,
            audio_decoder,
            nal_length_size: 4,
            url,
            frame_interval_ms: None,
            sample_interval_ms,
        }
    }

    /*
     * Helper function to get the bit at the specified index of a byte
     */
    fn get_bit(&self, byte: u8, bit_index: u8) -> u8 {
        if bit_index >= 8 {
            panic!("Bit index out of bounds: {}", bit_index);
        }

        ((byte & (1 << bit_index)) != 0).try_into().unwrap()
    }

    /*
     * Converts from a NAL with its length at the beginning
     * to a NAL with a start code at the beginning (as expected by decoder)
     */
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
        /*
         * This starts the yt-dlp program for a given url, looking for format 18
         * Format 18 corresponds to a mp4 file with audio and video tracks
         * Video is encoded in H264, at 640x360
         * Audio is encoded in AAC-LC
         */
        let mut yt_dlp_process = Command::new("yt-dlp")
            .args(["-o", "-", "--no-part", "-f", "18", &self.url])
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Could not start yt-dlp process");

        let mut yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

        let mut buffer = vec![0; 1000000];

        let mut accumulated_data: Vec<u8> = vec![];

        let mut ftyp_box = None;
        let mut sample_map: Option<SampleMap> = None;

        let mut mdat_reached = false;

        /*
         * After decoding, the frames are in YUP format
         * We need to convert to RGB to match the specification of the Kitty graphics protocol
         */
        let mut converter = self
            .video_decoder
            .converter(ffmpeg_next::format::Pixel::RGB24)
            .unwrap();

        let mut audio_timestamp_in_ms = 0;
        let mut video_timestamp_in_ms = 0;

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

                            match box_title.to_string().as_str() {
                                "ftyp" => {
                                    ftyp_box = Some(FTYPBox {
                                        size: box_size,
                                        data: accumulated_data
                                            .drain(..(box_size - 8) as usize)
                                            .collect(),
                                    });

                                    assert_eq!(box_size, ftyp_box.as_ref().unwrap().size);
                                }
                                "moov" => {
                                    let moov_box: Option<MOOVBox>;

                                    match get_moov_box(
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

                                    for trak in &moov_box.as_ref().unwrap().traks {
                                        if let Some(ref avcc_data) = trak.media.minf.stbl.stsd.avcc
                                        {
                                            self.nal_length_size = self.get_bit(avcc_data[4], 0)
                                                + self.get_bit(avcc_data[4], 1) * 2
                                                + 1;
                                        }

                                        if let Streams::Video = trak.media.minf.header {
                                            let mdhd = trak.media.mdhd.clone();

                                            let version_byte = mdhd.data[0];

                                            let timescale: u32;

                                            match version_byte {
                                                0 => {
                                                    let timescale_bytes: [u8; 4] =
                                                        mdhd.data[12..=15].try_into().unwrap();

                                                    timescale = u32::from_be_bytes(timescale_bytes);
                                                }
                                                1 => {
                                                    let timescale_bytes: [u8; 4] =
                                                        mdhd.data[20..=23].try_into().unwrap();

                                                    timescale = u32::from_be_bytes(timescale_bytes);
                                                }
                                                _ => {
                                                    panic!("Unknown mdhd version");
                                                }
                                            }

                                            let mut stts = trak.media.minf.stbl.stts.clone();

                                            stts.data.drain(..4);

                                            let entry_count_bytes: [u8; 4] = stts
                                                .data
                                                .drain(..4)
                                                .collect::<Vec<u8>>()
                                                .try_into()
                                                .unwrap();

                                            let entry_count: u32 =
                                                u32::from_be_bytes(entry_count_bytes);

                                            assert_eq!(entry_count, 1);

                                            stts.data.drain(..4);

                                            let sample_delta_bytes: [u8; 4] = stts
                                                .data
                                                .drain(..4)
                                                .collect::<Vec<u8>>()
                                                .try_into()
                                                .unwrap();

                                            let sample_delta: u32 =
                                                u32::from_be_bytes(sample_delta_bytes);

                                            self.frame_interval_ms =
                                                Some(1000 / (timescale / sample_delta) as usize);

                                            let actual_fps = (timescale / sample_delta) as usize;

                                            self.raw_video_message_tx
                                                .send(RawVideoMessage::FramesPerSecond(actual_fps));
                                        }
                                    }

                                    sample_map = Some(get_sample_map(moov_box.unwrap()).unwrap());
                                }
                                "mdat" => {
                                    if ftyp_box.is_none() {
                                        println!("We are f'ed in the B by ftyp");
                                    }
                                    if sample_map.clone().is_none() {
                                        println!("We are f'ed in the B by moov");
                                    }

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

                    if sample_map.is_some() {
                        while !sample_map.as_ref().unwrap().is_empty()
                            && accumulated_data.len()
                                >= sample_map.as_ref().unwrap().front().unwrap().size as usize
                        {
                            let current_sample_data =
                                sample_map.as_mut().unwrap().pop_front().unwrap();

                            let sample: Vec<u8> = accumulated_data
                                .drain(..current_sample_data.size as usize)
                                .collect();

                            if current_sample_data.is_video {
                                let annexb_data = self.convert_avcc_to_annexb(&sample);
                                if !annexb_data.is_empty() {
                                    let packet = Packet::copy(&annexb_data);

                                    match self.video_decoder.send_packet(&packet) {
                                        Ok(_) => {
                                            let mut yup_frame = frame::Video::empty();
                                            let mut rgb_frame = frame::Video::empty();
                                            while self
                                                .video_decoder
                                                .receive_frame(&mut yup_frame)
                                                .is_ok()
                                            {
                                                let _ = converter.run(&yup_frame, &mut rgb_frame);
                                                let data = rgb_frame.data(0);

                                                assert_eq!(data.len(), 640 * 360 * 3);

                                                self.raw_video_message_tx.send(
                                                    RawVideoMessage::VideoMessage(
                                                        BytesWithTimestamp {
                                                            data: data.to_vec(),
                                                            timestamp_in_ms: video_timestamp_in_ms,
                                                        },
                                                    ),
                                                );

                                                video_timestamp_in_ms +=
                                                    self.frame_interval_ms.unwrap();
                                                yup_frame = frame::Video::empty();
                                                rgb_frame = frame::Video::empty();
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

                                            assert_eq!(data.len(), 8192);

                                            self.raw_audio_message_tx.send(
                                                RawAudioMessage::AudioMessage(BytesWithTimestamp {
                                                    data: data.to_vec(),
                                                    timestamp_in_ms: audio_timestamp_in_ms,
                                                }),
                                            );

                                            audio_timestamp_in_ms += self.sample_interval_ms;
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

        self.raw_video_message_tx
            .send(RawVideoMessage::Done)
            .unwrap();
        self.raw_audio_message_tx
            .send(RawAudioMessage::Done)
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_get_bit() {
        let demux = create_test_demux();

        // Test getting different bits from a byte
        let test_byte = 0b10101010; // 170 in decimal

        assert_eq!(demux.get_bit(test_byte, 0), 0); // LSB
        assert_eq!(demux.get_bit(test_byte, 1), 1);
        assert_eq!(demux.get_bit(test_byte, 2), 0);
        assert_eq!(demux.get_bit(test_byte, 3), 1);
        assert_eq!(demux.get_bit(test_byte, 4), 0);
        assert_eq!(demux.get_bit(test_byte, 5), 1);
        assert_eq!(demux.get_bit(test_byte, 6), 0);
        assert_eq!(demux.get_bit(test_byte, 7), 1); // MSB
    }

    #[test]
    #[should_panic(expected = "Bit index out of bounds")]
    fn test_get_bit_out_of_bounds() {
        let demux = create_test_demux();
        demux.get_bit(0xFF, 8); // Should panic
    }

    #[test]
    fn test_convert_avcc_to_annexb_basic() {
        let demux = create_test_demux();

        // Test data: 4-byte length (0x00000004) + 4 bytes of NAL data
        let avcc_data = vec![
            0x00, 0x00, 0x00, 0x04, // Length: 4 bytes
            0x67, 0x42, 0x00, 0x1F, // NAL unit data (SPS header example)
        ];

        let annexb_data = demux.convert_avcc_to_annexb(&avcc_data);

        // Expected: start code (0x00000001) + NAL data
        let expected = vec![
            0x00, 0x00, 0x00, 0x01, // Start code
            0x67, 0x42, 0x00, 0x1F, // NAL unit data
        ];

        assert_eq!(annexb_data, expected);
    }

    #[test]
    fn test_convert_avcc_to_annexb_multiple_nals() {
        let demux = create_test_demux();

        // Test data with two NAL units
        let avcc_data = vec![
            0x00, 0x00, 0x00, 0x02, // Length: 2 bytes
            0x67, 0x42, // First NAL unit
            0x00, 0x00, 0x00, 0x03, // Length: 3 bytes
            0x68, 0x43, 0x44, // Second NAL unit
        ];

        let annexb_data = demux.convert_avcc_to_annexb(&avcc_data);

        let expected = vec![
            0x00, 0x00, 0x00, 0x01, // Start code for first NAL
            0x67, 0x42, // First NAL unit
            0x00, 0x00, 0x00, 0x01, // Start code for second NAL
            0x68, 0x43, 0x44, // Second NAL unit
        ];

        assert_eq!(annexb_data, expected);
    }

    #[test]
    fn test_convert_avcc_to_annexb_empty_data() {
        let demux = create_test_demux();
        let empty_data = vec![];
        let result = demux.convert_avcc_to_annexb(&empty_data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_avcc_to_annexb_invalid_length() {
        let demux = create_test_demux();

        // Test data with length longer than available data
        let invalid_data = vec![
            0x00, 0x00, 0x00, 0x10, // Length: 16 bytes (but only 2 bytes follow)
            0x67, 0x42, // Only 2 bytes of data
        ];

        let result = demux.convert_avcc_to_annexb(&invalid_data);
        assert!(result.is_empty()); // Should return empty due to invalid length
    }

    // Helper function to create a test demux instance
    fn create_test_demux() -> Demultiplexer {
        let (audio_tx, _audio_rx) = channel();
        let (video_tx, _video_rx) = channel();

        Demultiplexer::new(
            audio_tx,
            video_tx,
            "https://example.com/video".to_string(),
            23, // sample_interval_ms
        )
    }
}
