use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use args::Args;
use display_manager::DisplayManager;
use encoder::Encoder;
use ring_buffer::{Frame, RingBuffer};
use screen_guard::ScreenGuard;

mod args;
mod display_manager;
mod encoder;
mod result;
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

    let frame_size = width * height * 3;
    let interval = 1000 / fps;

    let video_buffer = Arc::new(Mutex::new(RingBuffer::<Frame>::new()));
    let encoded_buffer = Arc::new(Mutex::new(RingBuffer::<Frame>::new()));

    let (display_started_tx, display_started_rx) = mpsc::channel::<()>();
    let (streaming_done_tx, streaming_done_rx) = mpsc::channel::<()>();
    let (encoding_done_tx, encoding_done_rx) = mpsc::channel::<()>();

    let mut encoder = Encoder::new(
        Arc::clone(&video_buffer),
        Arc::clone(&encoded_buffer),
        width,
        height,
        streaming_done_rx,
        encoding_done_tx,
    )
    .expect("Failed to create encoder");

    let display_manager = DisplayManager::new(
        Arc::clone(&encoded_buffer),
        encoding_done_rx,
        display_started_tx,
    )
    .expect("Failed to create display manager");

    let mut yt_dlp_process = Command::new("yt-dlp")
        .args([
            "-o",
            "-",
            "--no-part",
            "-f",
            format!("bestvideo[height={height}][width={width}]").as_str(),
            &url,
        ])
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start yt-dlp process");

    let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

    let mut ffmpeg_process = Command::new("ffmpeg")
        .args(["-i", "pipe:0", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .stdin(Stdio::from(yt_dlp_stdout))
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start ffmpeg process");

    let mut ffmpeg_stdout = ffmpeg_process
        .stdout
        .take()
        .expect("Failed to get ffmpeg stdout");

    let mut timestamp = 0;
    let mut accumulated_data = Vec::new();

    // 32KB chunks, chunks that yt-dlp outputs
    let yt_dlp_chunk_size = 32768;
    let mut read_buffer = vec![0u8; yt_dlp_chunk_size];

    let audio_thread = thread::spawn(move || {
        let mut yt_dlp_process = Command::new("yt-dlp")
            .args([
                "-o",
                "-",
                "--no-part",
                "-f",
                format!("bestaudio").as_str(),
                &url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Could not start yt-dlp process");

        let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

        while !display_started_rx.try_recv().is_ok() {
            thread::sleep(std::time::Duration::from_millis(100));
        }

        let mut ffmpeg_process = Command::new("ffmpeg")
            .args(["-i", "pipe:0", "-vn", "-f", "pulse", "default"])
            .stdin(Stdio::from(yt_dlp_stdout))
            .stderr(Stdio::null())
            .spawn()
            .expect("Could not start ffmpeg process");

        let _ = yt_dlp_process.wait();
        let _ = ffmpeg_process.wait();
    });

    let encode_thread = thread::spawn(move || {
        encoder.encode().expect("Failed to encode frames");
    });

    let display_thread = thread::spawn(move || {
        display_manager.display().expect("Failed to display frames");
    });

    loop {
        match ffmpeg_stdout.read(&mut read_buffer) {
            Ok(0) => {
                streaming_done_tx.send(()).unwrap();
                break;
            }
            Ok(bytes_read) => {
                accumulated_data.extend_from_slice(&read_buffer[0..bytes_read]);

                while accumulated_data.len() >= frame_size {
                    let frame_data = accumulated_data.drain(0..frame_size).collect::<Vec<u8>>();
                    let frame = Frame::new(frame_data, timestamp);

                    video_buffer.lock().unwrap().push_frame(frame);

                    timestamp += interval as u64;
                }
            }
            Err(e) => {
                eprintln!("Error reading from ffmpeg: {}", e);
                break;
            }
        }
    }

    if !accumulated_data.is_empty() {
        println!("Leftover incomplete data: {} bytes", accumulated_data.len());
    }

    // Wait for processes to complete
    let _ = ffmpeg_process.wait();
    let _ = yt_dlp_process.wait();

    // Wait for threads to finish
    let _ = encode_thread.join();
    let _ = display_thread.join();
    let _ = audio_thread.join();
}
