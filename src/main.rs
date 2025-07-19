mod helpers {
    pub mod args;
    pub mod structs;
    pub mod types;
}

mod video {
    pub mod adapter;
    pub mod encoder;
}

mod audio {
    pub mod adapter;
}

mod demux {
    pub mod demultiplexer;
    mod get_moov_box;
    mod get_sample_map;
}

use std::{sync::mpsc::channel, thread};

use audio::adapter::AudioAdapter;
use demux::demultiplexer::{Demultiplexer, RawAudioMessage, RawVideoMessage};
use helpers::{
    args::{parse_args, Args},
    structs::{ContentQueue, ScreenGuard},
};
use video::encoder::{EncodedVideoMessage, Encoder};

fn main() {
    ffmpeg_next::init().unwrap();

    let (demultiplexer_audio_tx, demultiplexer_audio_rx) = channel::<RawAudioMessage>();
    let (demultiplexer_video_tx, demultiplexer_video_rx) = channel::<RawVideoMessage>();
    let (video_encoding_tx, video_encoding_rx) = channel::<EncodedVideoMessage>();

    let frames_per_second = 30;
    let frame_interval_ms = 1000 / frames_per_second;

    let _ = ScreenGuard::new().expect("Failed to initialize screen guard");

    let Args { url, width, height } = parse_args();

    let mut demux = Demultiplexer::new(demultiplexer_video_tx, demultiplexer_audio_tx, url);

    thread::spawn(move || {
        demux.demux();
    });

    let mut encoder = Encoder::new(width, height, demultiplexer_video_rx, video_encoding_tx)
        .expect("Failed to create encoder");

    thread::spawn(move || {
        encoder.encode().expect("Failed to start encoding");
    });

    let audio_adapter =
        AudioAdapter::new(demultiplexer_audio_rx).expect("Failed to create audio adapter");

    thread::spawn(move || {
        audio_adapter.run().expect("Failed to start audio playback");
    });

    let video_adapter = video::adapter::TerminalAdapter::new(video_encoding_rx)
        .expect("Failed to create video adapter");

    thread::spawn(move || {
        video_adapter.run().expect("Failed to start video display");
    });
}
