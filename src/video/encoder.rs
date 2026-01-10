use crate::demux::demultiplexer::RawVideoMessage;
use crate::helpers::types::{BytesWithTimestamp, Res};
use base64::{engine::general_purpose, Engine as _};
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{collections::HashMap, sync::mpsc, time::Duration};

pub enum EncodedVideoMessage {
    EncodedVideoMessage(BytesWithTimestamp),
    Done,
}

pub struct Encoder {
    width: usize,
    height: usize,
    term_width: u16,
    term_height: u16,
    producer_rx: mpsc::Receiver<RawVideoMessage>,
    producer_tx: mpsc::Sender<EncodedVideoMessage>,
    force_y_offset: Option<usize>,
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl Encoder {
    pub fn new(
        producer_rx: mpsc::Receiver<RawVideoMessage>,
        producer_tx: mpsc::Sender<EncodedVideoMessage>,
        force_y_offset: Option<usize>,
    ) -> Res<Self> {
        let (term_width, term_height) = Self::get_terminal_size().unwrap_or((1280, 720));

        if term_width == 0 || term_height == 0 {
            return Err("Invalid terminal size".into());
        }

        Ok(Encoder {
            width: 640,
            height: 360,
            term_width,
            term_height,
            producer_rx,
            producer_tx,
            force_y_offset,
            cancel_flag: None,
        })
    }

    pub fn set_cancel_flag(&mut self, flag: Arc<AtomicBool>) {
        self.cancel_flag = Some(flag);
    }

    // Convert a frame to the Kitty Graphics Protocol format
    fn encode_frame(
        &self,
        encoded_control_data: &Vec<u8>,
        frame: BytesWithTimestamp,
    ) -> BytesWithTimestamp {
        // Base64 encode the frame data
        let encoded_payload = self.encode_rgb(frame.data);
        let prefix = b"\x1b_G";
        let suffix = b"\x1b\\";
        let delimiter = b";";
        let mut buffer = vec![];

        buffer.extend_from_slice(prefix);
        buffer.extend_from_slice(&encoded_control_data);
        buffer.extend_from_slice(delimiter);
        buffer.extend_from_slice(&encoded_payload);
        buffer.extend_from_slice(suffix);

        BytesWithTimestamp {
            data: buffer,
            timestamp_in_ms: frame.timestamp_in_ms,
        }
    }

    pub fn encode(&mut self) -> Res<()> {
        let x_offset = (self.term_width as usize - self.width) / 2;
        let y_offset = self.force_y_offset.unwrap_or_else(|| {
            (self.term_height as usize - self.height) / 2
        });

        let encoded_control_data = self.encode_control_data(HashMap::from([
            ("f".into(), "24".into()),
            ("s".into(), format!("{}", self.width)),
            ("v".into(), format!("{}", self.height)),
            ("t".into(), "d".into()),
            ("a".into(), "T".into()),
            ("X".into(), format!("{}", x_offset)),
            ("Y".into(), format!("{}", y_offset)),
        ]));

        loop {
            if let Some(ref flag) = self.cancel_flag {
                if flag.load(Ordering::SeqCst) {
                    self.producer_tx.send(EncodedVideoMessage::Done).ok();
                    return Ok(());
                }
            }

            match self.producer_rx.recv_timeout(Duration::from_millis(16)) {
                Ok(message) => match message {
                    RawVideoMessage::VideoMessage(frame) => {
                        let encoded_frame = self.encode_frame(&encoded_control_data, frame);

                        self.producer_tx
                            .send(EncodedVideoMessage::EncodedVideoMessage(encoded_frame))
                            .unwrap();
                    }
                    RawVideoMessage::Done => {
                        self.producer_tx.send(EncodedVideoMessage::Done).unwrap();
                        return Ok(());
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
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

    fn encode_rgb(&self, rgb: Vec<u8>) -> Vec<u8> {
        let encoded = general_purpose::STANDARD.encode(&rgb);
        encoded.as_bytes().to_vec()
    }

    fn get_terminal_size() -> std::io::Result<(u16, u16)> {
        let mut winsize: libc::winsize = unsafe { mem::zeroed() };

        let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize) };

        if result == -1 {
            return Err(std::io::Error::last_os_error());
        }

        Ok((winsize.ws_xpixel, winsize.ws_ypixel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_new_encoder() {
        let (_streaming_done_tx, producer_rx) = mpsc::channel();
        let (producer_tx, _encoding_done_rx) = mpsc::channel();

        let encoder = Encoder::new(producer_rx, producer_tx, None).unwrap();

        assert_eq!(encoder.width, 640);
        assert_eq!(encoder.height, 360);
    }

    #[test]
    fn test_encode_control_data() {
        let encoder = Encoder::new(mpsc::channel().1, mpsc::channel().0, None).unwrap();

        let control_data = HashMap::from([
            ("f".into(), "24".into()),
            ("s".into(), "640".into()),
            ("v".into(), "360".into()),
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
            .contains("v=360"));
    }

    #[test]
    fn test_get_terminal_size() {
        let (_streaming_done_tx, producer_rx) = mpsc::channel();
        let (producer_tx, _encoding_done_rx) = mpsc::channel();

        let encoder = Encoder::new(producer_rx, producer_tx, None).unwrap();

        assert!(
            encoder.term_width > 0 && encoder.term_height > 0,
            "Terminal size should be greater than zero"
        );
    }
}
