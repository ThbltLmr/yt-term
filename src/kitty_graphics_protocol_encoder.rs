use crate::{
    result::Res,
    ring_buffer::{Frame, RingBuffer},
};
use base64::{engine::general_purpose, Engine as _};
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};

pub struct KittyGraphicsProtocolEncoder {
    video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    kitty_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    width: usize,
    height: usize,
    streaming_done_rx: mpsc::Receiver<()>,
    encoding_done_tx: mpsc::Sender<()>,
}

impl KittyGraphicsProtocolEncoder {
    pub fn new(
        video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        kitty_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        width: usize,
        height: usize,
        streaming_done_rx: mpsc::Receiver<()>,
        encoding_done_tx: mpsc::Sender<()>,
    ) -> Self {
        KittyGraphicsProtocolEncoder {
            video_buffer,
            kitty_buffer,
            width,
            height,
            streaming_done_rx,
            encoding_done_tx,
        }
    }

    // Convert a frame to KittyGraphicsProtocol graphics protocol
    fn encode_frame_kitty(&self, frame: Frame) -> Frame {
        // Base64 encode the frame data
        let (control_data, payload) = (
            HashMap::from([
                ("f".into(), "24".into()),
                ("s".into(), format!("{}", self.width).into()),
                ("v".into(), format!("{}", self.height).into()),
                ("t".into(), "d".into()),
                ("a".into(), "T".into()),
            ]),
            frame.data,
        );

        let encoded_payload = self.encode_rbg(payload);
        let encoded_control_data = self.encode_control_data(control_data);
        let prefix = b"\x1b_G";
        let suffix = b"\x1b\\";
        let delimiter = b";";
        let mut buffer = vec![];

        buffer.extend_from_slice(prefix);
        buffer.extend_from_slice(&encoded_control_data);
        buffer.extend_from_slice(delimiter);
        buffer.extend_from_slice(&encoded_payload);
        buffer.extend_from_slice(suffix);

        Frame::new(buffer, frame.timestamp)
    }

    pub fn encode(&mut self) -> Res<()> {
        loop {
            // Get the video frame from the video buffer
            let mut video_buffer = self.video_buffer.lock().unwrap();
            let frame = video_buffer.get_frame();

            if let Some(frame) = frame {
                // Encode the frame to KittyGraphicsProtocol graphics protocol
                let encoded_frame = self.encode_frame_kitty(frame);

                // Push the encoded frame to the kitty buffer
                let mut encoded_buffer = self.kitty_buffer.lock().unwrap();
                encoded_buffer.push_frame(encoded_frame);
            } else {
                if self.streaming_done_rx.try_recv().is_ok() {
                    // If streaming is done and no more frames are available,
                    // break the loop
                    self.encoding_done_tx.send(()).unwrap();
                    return Ok(());
                }
            }
        }
    }

    fn encode_control_data(&self, control_data: HashMap<String, String>) -> Vec<u8> {
        let mut encoded_data = Vec::new();
        for (key, value) in control_data {
            encoded_data.push(format!("{}={}", key, value));
        }

        encoded_data.join(",").as_bytes().to_vec()
    }

    fn encode_rbg(&self, rgb: Vec<u8>) -> Vec<u8> {
        let encoded = general_purpose::STANDARD.encode(&rgb);
        encoded.as_bytes().to_vec()
    }
}
