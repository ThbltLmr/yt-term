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
    term_cols: u16,
    term_rows: u16,
    producer_rx: mpsc::Receiver<RawVideoMessage>,
    producer_tx: mpsc::Sender<EncodedVideoMessage>,
    force_y_offset: Option<usize>,
    video_rows: Option<u16>,
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl Encoder {
    pub fn new(
        producer_rx: mpsc::Receiver<RawVideoMessage>,
        producer_tx: mpsc::Sender<EncodedVideoMessage>,
        force_y_offset: Option<usize>,
        video_rows: Option<u16>,
    ) -> Res<Self> {
        let (term_width, term_height, term_cols, term_rows) =
            Self::get_terminal_size().unwrap_or((1280, 720, 80, 24));

        if term_width == 0 || term_height == 0 || term_cols == 0 || term_rows == 0 {
            return Err("Invalid terminal size".into());
        }

        Ok(Encoder {
            width: 640,
            height: 360,
            term_width,
            term_height,
            term_cols,
            term_rows,
            producer_rx,
            producer_tx,
            force_y_offset,
            video_rows,
            cancel_flag: None,
        })
    }

    pub fn set_cancel_flag(&mut self, flag: Arc<AtomicBool>) {
        self.cancel_flag = Some(flag);
    }

    /// Returns pixel dimensions per cell (width, height).
    fn cell_pixel_dimensions(&self) -> (f64, f64) {
        (
            self.term_width as f64 / self.term_cols as f64,
            self.term_height as f64 / self.term_rows as f64,
        )
    }

    /// Calculate display dimensions (columns, rows) for Kitty protocol scaling.
    /// Maintains aspect ratio while fitting within available space.
    fn calculate_display_dimensions(&self) -> (u16, u16) {
        let (cell_width_px, cell_height_px) = self.cell_pixel_dimensions();

        // Determine available rows
        let available_rows = self.video_rows.unwrap_or(self.term_rows);
        let available_cols = self.term_cols;

        // Calculate target rows (video height in cells)
        // Start by filling available height
        let target_rows = available_rows;

        // Calculate corresponding width to maintain aspect ratio
        let video_aspect = self.width as f64 / self.height as f64;
        let target_height_px = target_rows as f64 * cell_height_px;
        let target_width_px = target_height_px * video_aspect;
        let mut target_cols = (target_width_px / cell_width_px).round() as u16;

        // Clamp to available width
        if target_cols > available_cols {
            target_cols = available_cols;
            // Recalculate rows based on width constraint
            let constrained_width_px = target_cols as f64 * cell_width_px;
            let constrained_height_px = constrained_width_px / video_aspect;
            let constrained_rows = (constrained_height_px / cell_height_px).round() as u16;
            return (target_cols, constrained_rows.min(available_rows));
        }

        (target_cols, target_rows)
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
        // Calculate display dimensions for Kitty scaling
        let (display_cols, display_rows) = self.calculate_display_dimensions();
        let (cell_width_px, cell_height_px) = self.cell_pixel_dimensions();

        // Calculate the actual pixel size of the scaled video
        let scaled_width_px = display_cols as f64 * cell_width_px;
        let scaled_height_px = display_rows as f64 * cell_height_px;

        // Calculate x offset to center horizontally
        let available_width_px = self.term_cols as f64 * cell_width_px;
        let x_offset = ((available_width_px - scaled_width_px) / 2.0).max(0.0) as usize;

        // Calculate y offset: use force_y_offset if set, otherwise center within available space
        let y_offset = self.force_y_offset.unwrap_or_else(|| {
            let available_rows = self.video_rows.unwrap_or(self.term_rows);
            let available_height_px = available_rows as f64 * cell_height_px;
            ((available_height_px - scaled_height_px) / 2.0).max(0.0) as usize
        });

        let encoded_control_data = self.encode_control_data(HashMap::from([
            ("f".into(), "24".into()),
            ("s".into(), format!("{}", self.width)),
            ("v".into(), format!("{}", self.height)),
            ("c".into(), format!("{}", display_cols)),
            ("r".into(), format!("{}", display_rows)),
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

    fn get_terminal_size() -> std::io::Result<(u16, u16, u16, u16)> {
        let mut winsize: libc::winsize = unsafe { mem::zeroed() };

        let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize) };

        if result == -1 {
            return Err(std::io::Error::last_os_error());
        }

        Ok((winsize.ws_xpixel, winsize.ws_ypixel, winsize.ws_col, winsize.ws_row))
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

        let encoder = Encoder::new(producer_rx, producer_tx, None, None).unwrap();

        assert_eq!(encoder.width, 640);
        assert_eq!(encoder.height, 360);
    }

    #[test]
    fn test_encode_control_data() {
        let encoder = Encoder::new(mpsc::channel().1, mpsc::channel().0, None, None).unwrap();

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

        let encoder = Encoder::new(producer_rx, producer_tx, None, None).unwrap();

        assert!(
            encoder.term_width > 0 && encoder.term_height > 0,
            "Terminal size should be greater than zero"
        );
    }
}
