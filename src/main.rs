mod helpers {
    pub mod adapter;
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
    adapter::Adapter,
    args::{parse_args, Args},
    structs::{ContentQueue, ScreenGuard},
};
use video::encoder::{EncodedVideoMessage, Encoder};

fn main() {
    ffmpeg_next::init().unwrap();

    let (demultiplexer_audio_tx, demultiplexer_audio_rx) = channel::<RawAudioMessage>();
    let (demultiplexer_video_tx, demultiplexer_video_rx) = channel::<RawVideoMessage>();
    let (video_encoding_tx, video_encoding_rx) = channel::<EncodedVideoMessage>();
    let (playing_tx, playing_rx) = channel::<()>();

    let frames_per_second = 30;
    let frame_interval_ms = 1000 / frames_per_second;

    let audio_bytes_per_second = 44100 * 2 * 4;
    let audio_sample_size = 8192;

    let samples_per_second = audio_bytes_per_second / audio_sample_size;
    let sample_interval_ms = 1000 / samples_per_second;

    let _ = ScreenGuard::new().expect("Failed to initialize screen guard");

    let Args { url, width, height } = parse_args();

    let mut demux = Demultiplexer::new(
        demultiplexer_video_tx,
        demultiplexer_audio_tx,
        url,
        frame_interval_ms,
    );

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

    loop {
        if video_encoding_rx.try_recv().is_ok()
            && encoded_video_buffer.lock().unwrap().is_empty()
            && audio_buffer.lock().unwrap().is_empty()
        {
            audio_queueing_tx
                .send(())
                .expect("Failed to send audio queueing done signal");
            video_queueing_tx
                .send(())
                .expect("Failed to send audio queueing done signal");

            playing_tx
                .send(())
                .expect("Failed to send playing done signal");

            break;
        }

        if audio_buffer.lock().unwrap().has_one_second_ready()
            && encoded_video_buffer.lock().unwrap().has_one_second_ready()
        {
            audio_buffer
                .lock()
                .unwrap()
                .queue_one_second_into(ready_audio_buffer.clone());

            encoded_video_buffer
                .lock()
                .unwrap()
                .queue_one_second_into(ready_video_buffer.clone());
        } else {
            thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
