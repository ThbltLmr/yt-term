mod helpers {
    pub mod adapter;
    pub mod args;
    pub mod logger;
    pub mod structs;
    pub mod types;
}

mod video {
    pub mod adapter;
    pub mod encoder;
    pub mod streamer;
}

mod audio {
    pub mod adapter;
}

mod demux {
    pub mod demultiplexer;
    mod moov;
    mod sample_data;
}

use std::{
    sync::{mpsc::channel, Arc, Mutex},
    thread,
    time::Duration,
};

use demux::demultiplexer::Demultiplexer;
use helpers::{
    adapter::Adapter,
    args::{parse_args, Args},
    structs::{ContentQueue, ScreenGuard},
};

fn main() {
    ffmpeg_next::init().unwrap();
    let (demultiplexing_done_tx, demultiplexing_done_rx) = channel();

    // let _ = ScreenGuard::new().expect("Failed to initialize screen guard");

    let Args { url, width, height } = parse_args();

    let raw_video_buffer = Arc::new(Mutex::new(ContentQueue::new(30)));
    let encoded_video_buffer = Arc::new(Mutex::new(ContentQueue::new(30)));
    let audio_buffer = Arc::new(Mutex::new(ContentQueue::new(43)));

    let mut demux = Demultiplexer::new(
        raw_video_buffer.clone(),
        audio_buffer.clone(),
        demultiplexing_done_tx,
    );

    thread::spawn(move || {
        demux.demux();
    });

    let (video_encoding_done_tx, video_encoding_done_rx) = channel();

    let (audio_queueing_done_tx, audio_queueing_done_rx) = channel();
    let (video_queueing_done_tx, video_queueing_done_rx) = channel();

    let (playing_done_tx, playing_done_rx) = channel();

    let mut encoder = video::encoder::Encoder::new(
        raw_video_buffer.clone(),
        encoded_video_buffer.clone(),
        width,
        height,
        demultiplexing_done_rx,
        video_encoding_done_tx,
    )
    .expect("Failed to create encoder");

    thread::spawn(move || {
        encoder.encode().expect("Failed to start encoding");
    });

    let ready_audio_buffer = Arc::new(Mutex::new(ContentQueue::new(43)));
    let ready_video_buffer = Arc::new(Mutex::new(ContentQueue::new(30)));

    let audio_adapter = audio::adapter::AudioAdapter::new(
        Duration::from_millis(24),
        audio_buffer.clone(),
        audio_queueing_done_rx,
    )
    .expect("Failed to create audio adapter");

    thread::spawn(move || {
        audio_adapter.run().expect("Failed to start audio playback");
    });

    let video_adapter = video::adapter::TerminalAdapter::new(
        Duration::from_millis(33),
        ready_video_buffer.clone(),
        video_queueing_done_rx,
    )
    .expect("Failed to create video adapter");

    thread::spawn(move || {
        //video_adapter.run().expect("Failed to start video display");
    });

    let mut logger = helpers::logger::Logger::new(
        raw_video_buffer.clone(),
        encoded_video_buffer.clone(),
        audio_buffer.clone(),
        ready_video_buffer.clone(),
        ready_audio_buffer.clone(),
        playing_done_rx,
    )
    .expect("Failed to create logger");

    thread::spawn(move || {
        logger.log().expect("Failed to start logging");
    });

    loop {
        if video_encoding_done_rx.try_recv().is_ok()
            && encoded_video_buffer.lock().unwrap().is_empty()
            && audio_buffer.lock().unwrap().is_empty()
        {
            audio_queueing_done_tx
                .send(())
                .expect("Failed to send audio queueing done signal");
            video_queueing_done_tx
                .send(())
                .expect("Failed to send audio queueing done signal");

            playing_done_tx
                .send(())
                .expect("Failed to send playing done signal");

            break;
        }

        if audio_buffer.lock().unwrap().has_one_second_ready()
            && encoded_video_buffer.lock().unwrap().has_one_second_ready()
        {
            //audio_buffer
            //.lock()
            //.unwrap()
            //.queue_one_second_into(ready_audio_buffer.clone());

            encoded_video_buffer
                .lock()
                .unwrap()
                .queue_one_second_into(ready_video_buffer.clone());
        } else {
            thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
