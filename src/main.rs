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
    pub mod codec_context;
    pub mod demultiplexer;
    mod get_moov_box;
    mod get_sample_map;
}

mod tui;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{sync::mpsc::channel, thread};

use audio::adapter::AudioAdapter;
use demux::demultiplexer::{Demultiplexer, RawAudioMessage, RawVideoMessage};
use helpers::{args::parse_args, structs::ScreenGuard};
use video::{
    adapter::TerminalAdapter,
    encoder::{EncodedVideoMessage, Encoder},
};

pub struct PlaybackHandle {
    cancel_flag: Arc<AtomicBool>,
    demux_handle: thread::JoinHandle<()>,
    encode_handle: thread::JoinHandle<()>,
    audio_handle: thread::JoinHandle<()>,
    video_handle: thread::JoinHandle<()>,
}

impl PlaybackHandle {
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn is_finished(&self) -> bool {
        self.demux_handle.is_finished()
            && self.encode_handle.is_finished()
            && self.audio_handle.is_finished()
            && self.video_handle.is_finished()
    }

    pub fn join(self) {
        let _ = self.demux_handle.join();
        let _ = self.encode_handle.join();
        let _ = self.audio_handle.join();
        let _ = self.video_handle.join();
    }
}

fn main() {
    ffmpeg_next::init().unwrap();

    let args = parse_args();

    if args.url.is_some() || args.search.is_some() {
        let input = if let Some(url) = args.url {
            url
        } else if let Some(search) = args.search {
            format!("ytsearch:{}", search)
        } else {
            unreachable!()
        };
        run_direct_playback(&input, true, true);
    } else {
        tui::run().expect("TUI error");
    }
}

fn run_direct_playback(input: &str, use_screen_guard: bool, center_video: bool) {
    let (demultiplexer_audio_tx, demultiplexer_audio_rx) = channel::<RawAudioMessage>();
    let (demultiplexer_video_tx, demultiplexer_video_rx) = channel::<RawVideoMessage>();
    let (video_encoding_tx, video_encoding_rx) = channel::<EncodedVideoMessage>();

    let _screen_guard = if use_screen_guard {
        Some(ScreenGuard::new().expect("Failed to initialize screen guard"))
    } else {
        None
    };

    let y_offset = if center_video { None } else { Some(0) };

    let mut demux =
        Demultiplexer::new(demultiplexer_video_tx, demultiplexer_audio_tx, input.to_string());

    let demux_handle = thread::spawn(move || {
        demux.demux().expect("Failed to start demultiplexer");
    });

    let mut encoder =
        Encoder::new(demultiplexer_video_rx, video_encoding_tx, y_offset).expect("Failed to create encoder");

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
}

pub fn start_playback_async(input: &str, center_video: bool) -> PlaybackHandle {
    let cancel_flag = Arc::new(AtomicBool::new(false));

    let (demultiplexer_audio_tx, demultiplexer_audio_rx) = channel::<RawAudioMessage>();
    let (demultiplexer_video_tx, demultiplexer_video_rx) = channel::<RawVideoMessage>();
    let (video_encoding_tx, video_encoding_rx) = channel::<EncodedVideoMessage>();

    let y_offset = if center_video { None } else { Some(0) };

    let cancel = cancel_flag.clone();
    let url = input.to_string();
    let demux_handle = thread::spawn(move || {
        let mut demux = Demultiplexer::new(demultiplexer_video_tx, demultiplexer_audio_tx, url);
        demux.set_cancel_flag(cancel);
        let _ = demux.demux();
    });

    let cancel = cancel_flag.clone();
    let encode_handle = thread::spawn(move || {
        let mut encoder = Encoder::new(demultiplexer_video_rx, video_encoding_tx, y_offset)
            .expect("Failed to create encoder");
        encoder.set_cancel_flag(cancel);
        let _ = encoder.encode();
    });

    let cancel = cancel_flag.clone();
    let audio_handle = thread::spawn(move || {
        let mut audio_adapter =
            AudioAdapter::new(demultiplexer_audio_rx).expect("Failed to create audio adapter");
        audio_adapter.set_cancel_flag(cancel);
        let _ = audio_adapter.run();
    });

    let cancel = cancel_flag.clone();
    let video_handle = thread::spawn(move || {
        let mut video_adapter =
            TerminalAdapter::new(video_encoding_rx).expect("Failed to create video adapter");
        video_adapter.set_cancel_flag(cancel);
        let _ = video_adapter.run();
    });

    PlaybackHandle {
        cancel_flag,
        demux_handle,
        encode_handle,
        audio_handle,
        video_handle,
    }
}
