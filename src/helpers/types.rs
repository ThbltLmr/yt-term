pub type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug)]
pub struct BytesWithTimestamp {
    pub data: Vec<u8>,
    pub timestamp_in_ms: usize,
}
