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
    structs::ScreenGuard,
};
use video::{
    adapter::TerminalAdapter,
    encoder::{EncodedVideoMessage, Encoder},
};

fn main() {
    ffmpeg_next::init().unwrap();

    let (demultiplexer_audio_tx, demultiplexer_audio_rx) = channel::<RawAudioMessage>();
    let (demultiplexer_video_tx, demultiplexer_video_rx) = channel::<RawVideoMessage>();
    let (video_encoding_tx, video_encoding_rx) = channel::<EncodedVideoMessage>();

    let screen_guard = ScreenGuard::new().expect("Failed to initialize screen guard");

    let Args { url } = parse_args();

    let mut demux = Demultiplexer::new(demultiplexer_video_tx, demultiplexer_audio_tx, url);

    let demux_handle = thread::spawn(move || {
        demux.demux().expect("Failed to start demultiplexer");
    });

    let mut encoder =
        Encoder::new(demultiplexer_video_rx, video_encoding_tx).expect("Failed to create encoder");

    let encode_handle = thread::spawn(move || {
        encoder.encode().expect("Failed to start encoding");
    });

    let mut audio_adapter =
        AudioAdapter::new(demultiplexer_audio_rx).expect("Failed to create audio adapter");

    let audio_handle = thread::spawn(move || {
        audio_adapter.run().expect("Failed to start audio playback");
    });

    let mut video_adapter =
        TerminalAdapter::new(video_encoding_rx).expect("Failed to create video adapter");

    let video_handle = thread::spawn(move || {
        video_adapter.run().expect("Failed to start video display");
    });

    let _ = demux_handle.join();
    let _ = encode_handle.join();
    let _ = audio_handle.join();
    let _ = video_handle.join();

    drop(screen_guard);
}
