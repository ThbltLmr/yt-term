use crate::{
    result::Res,
    ring_buffer::{Frame, RingBuffer},
};
use base64::{engine::general_purpose, Engine as _};
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};

pub struct Encoder {
    video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    encoded_buffer: Arc<Mutex<RingBuffer<Frame>>>,
    width: usize,
    height: usize,
    streaming_done_rx: mpsc::Receiver<()>,
    encoding_done_tx: mpsc::Sender<()>,
}

impl Encoder {
    pub fn new(
        video_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        encoded_buffer: Arc<Mutex<RingBuffer<Frame>>>,
        width: usize,
        height: usize,
        streaming_done_rx: mpsc::Receiver<()>,
        encoding_done_tx: mpsc::Sender<()>,
    ) -> Self {
        Encoder {
            video_buffer,
            encoded_buffer,
            width,
            height,
            streaming_done_rx,
            encoding_done_tx,
        }
    }

    // Convert a frame to the Kitty Graphics Protocol format
    fn encode_frame(&self, frame: Frame) -> Frame {
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
            let mut video_buffer = self.video_buffer.lock().unwrap();
            let frame = video_buffer.get_frame();

            if let Some(frame) = frame {
                let encoded_frame = self.encode_frame(frame);
                let mut encoded_buffer = self.encoded_buffer.lock().unwrap();

                encoded_buffer.push_frame(encoded_frame);
            } else {
                if self.streaming_done_rx.try_recv().is_ok() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ring_buffer::RingBuffer;
    use std::sync::{mpsc, Arc, Mutex};

    #[test]
    fn test_new_encoder() {
        let video_buffer = Arc::new(Mutex::new(RingBuffer::new()));
        let encoded_buffer = Arc::new(Mutex::new(RingBuffer::new()));
        let (_streaming_done_tx, streaming_done_rx) = mpsc::channel();
        let (encoding_done_tx, _encoding_done_rx) = mpsc::channel();

        let encoder = Encoder::new(
            video_buffer.clone(),
            encoded_buffer.clone(),
            640,
            480,
            streaming_done_rx,
            encoding_done_tx,
        );

        assert_eq!(encoder.width, 640);
        assert_eq!(encoder.height, 480);
    }

    #[test]
    fn test_encode_control_data() {
        let encoder = Encoder::new(
            Arc::new(Mutex::new(RingBuffer::new())),
            Arc::new(Mutex::new(RingBuffer::new())),
            640,
            480,
            mpsc::channel().1,
            mpsc::channel().0,
        );

        let control_data = HashMap::from([
            ("f".into(), "24".into()),
            ("s".into(), "640".into()),
            ("v".into(), "480".into()),
        ]);

        let encoded_data = encoder.encode_control_data(control_data);
        assert!(String::from_utf8(encoded_data.clone()).is_ok());
        assert!(String::from_utf8(encoded_data.clone())
            .unwrap()
            .contains("f=24"));
        assert!(String::from_utf8(encoded_data.clone())
            .unwrap()
            .contains("s=640"));
        assert!(String::from_utf8(encoded_data.clone())
            .unwrap()
            .contains("v=480"));
    }

    #[test]
    fn test_encode_frame() {
        let video_buffer = Arc::new(Mutex::new(RingBuffer::new()));
        let encoded_buffer = Arc::new(Mutex::new(RingBuffer::new()));
        let (_streaming_done_tx, streaming_done_rx) = mpsc::channel();
        let (encoding_done_tx, _encoding_done_rx) = mpsc::channel();

        let mut encoder = Encoder::new(
            video_buffer.clone(),
            encoded_buffer.clone(),
            640,
            480,
            streaming_done_rx,
            encoding_done_tx,
        );

        let test_frame = Frame::new(vec![0; 640 * 480 * 3], 0);
        video_buffer.lock().unwrap().push_frame(test_frame);

        encoder.encode().unwrap();

        assert_eq!(encoded_buffer.lock().unwrap().len(), 1);
    }
}
