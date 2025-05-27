mod helpers {
    pub mod args;
    pub mod structs;
    pub mod types;
}

mod video {
    mod encoder;
    mod streamer;
}

mod audio {
    mod streamer;
}

use std::sync::{mpsc::channel, Arc, Mutex};

use helpers::{
    args::{parse_args, Args},
    structs::{Frame, RingBuffer, Sample},
};

fn main() {
    let Args {
        url,
        width,
        height,
        fps,
    } = parse_args();

    let encoded_video_buffer = Arc::new(Mutex::new(RingBuffer::<Frame>::new()));
    let audio_buffer = Arc::new(Mutex::new(RingBuffer::<Sample>::new()));
}
