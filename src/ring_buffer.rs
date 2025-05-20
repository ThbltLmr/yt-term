pub const MAX_BUFFER_SIZE: usize = 100;

pub trait RingBuffer<T> {
    fn new() -> Self;

    fn push_frame(&mut self, frame: T);

    fn get_frame(&mut self) -> Option<T>;

    fn len(&self) -> usize;
}
