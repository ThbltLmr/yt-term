pub struct VideoFrame {
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl VideoFrame {
    pub fn new(data: Vec<u8>, timestamp: u64) -> Self {
        VideoFrame { data, timestamp }
    }
}
