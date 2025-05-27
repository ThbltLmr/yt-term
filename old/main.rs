use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

use args::Args;
use audio_player::AudioPlayer;
use display_manager::DisplayManager;
use encoder::Encoder;
use ring_buffer::{Frame, RingBuffer};
use screen_guard::ScreenGuard;

mod args;
mod audio_player;
mod display_manager;
mod encoder;
mod result;
mod rgb_streamer;
mod ring_buffer;
mod screen_guard;

fn main() {
    let _screen_guard = ScreenGuard::new().expect("Failed to create screen guard");

    let Args {
        url,
        width,
        height,
        fps,
    } = args::parse_args();

    let rgb_buffer = Arc::new(Mutex::new(RingBuffer::<Frame>::new()));
    let encoded_buffer = Arc::new(Mutex::new(RingBuffer::<Frame>::new()));

    let (display_started_tx, display_started_rx) = mpsc::channel::<()>();
    let (streaming_done_tx, streaming_done_rx) = mpsc::channel::<()>();
    let (encoding_done_tx, encoding_done_rx) = mpsc::channel::<()>();

    let mut encoder = Encoder::new(
        Arc::clone(&rgb_buffer),
        Arc::clone(&encoded_buffer),
        width,
        height,
        streaming_done_rx,
        encoding_done_tx,
    )
    .expect("Failed to create encoder");

    let encode_thread = thread::spawn(move || {
        encoder.encode().expect("Failed to encode frames");
    });

    let display_manager = DisplayManager::new(
        Arc::clone(&encoded_buffer),
        encoding_done_rx,
        display_started_tx,
    )
    .expect("Failed to create display manager");

    let display_thread = thread::spawn(move || {
        display_manager.display().expect("Failed to display frames");
    });

    let audio_player = AudioPlayer::new(display_started_rx, url.clone())
        .expect("Failed to create display manager");

    let audio_thread = thread::spawn(move || {
        audio_player.play();
    });

    let rgb_streamer = rgb_streamer::RGBStreamer::new(
        Arc::clone(&rgb_buffer),
        streaming_done_tx,
        url.clone(),
        width,
        height,
        fps,
    )
    .unwrap();

    rgb_streamer.start_streaming();

    // Wait for threads to finish
    let _ = encode_thread.join();
    let _ = display_thread.join();
    let _ = audio_thread.join();
}
