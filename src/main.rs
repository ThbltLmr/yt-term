use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use clap::Parser;
use kitty_encoder::{KittyBuffer, KittyEncoder};
use ring_buffer::RingBuffer;
use video_buffer::{VideoBuffer, VideoFrame};

mod display_manager;
mod kitty_encoder;
mod result;
mod ring_buffer;
mod video_buffer;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(
        short,
        long,
        default_value = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    )]
    url: String,
}

fn main() {
    let args = Args::parse();
    let url = args.url;
    let width = 1280;
    let height = 720;

    let video_buffer = Arc::new(Mutex::new(VideoBuffer::new()));
    let kitty_buffer = Arc::new(Mutex::new(KittyBuffer::new()));

    let display_manager = display_manager::DisplayManager::new(Arc::clone(&kitty_buffer));
    let frame_size = width * height * 3;

    let kitty_encoder = KittyEncoder::new(
        Arc::clone(&video_buffer),
        Arc::clone(&kitty_buffer),
        width,
        height,
    );

    let mut yt_dlp_process = Command::new("yt-dlp")
        .args([
            "-o",
            "-",         // Output to stdout
            "--no-part", // Don't create .part files
            "-f",
            "232",
            &url,
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start yt-dlp process");

    // Connect yt-dlp's stdout to ffmpeg's stdin
    let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

    let mut ffmpeg_process = Command::new("ffmpeg")
        .args([
            "-i", "pipe:0", // Read from stdin
            "-f", "rawvideo", "-pix_fmt", "rgb24", "-", // Output to stdout
        ])
        .stdin(Stdio::from(yt_dlp_stdout)) // Connect the pipes
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start ffmpeg process");

    let mut ffmpeg_stdout = ffmpeg_process
        .stdout
        .take()
        .expect("Failed to get ffmpeg stdout");
    // Create a buffer for reading one frame at a time
    let mut accumulated_data = Vec::new();
    // Buffer for reading chunks from stdout
    let mut read_buffer = vec![0u8; 32768]; // 32KB chunks

    thread::spawn(move || {
        // Start the Kitty encoder thread
        kitty_encoder.encode();
    });

    thread::spawn(move || {
        display_manager.display();
    });

    // Read frames from ffmpeg and store them in the video buffer
    loop {
        match ffmpeg_stdout.read(&mut read_buffer) {
            Ok(0) => {
                // End of stream
                break;
            }
            Ok(bytes_read) => {
                // Append the newly read data to our accumulated buffer
                accumulated_data.extend_from_slice(&read_buffer[0..bytes_read]);

                // Process complete frames
                while accumulated_data.len() >= frame_size {
                    // Extract a frame
                    let frame_data = accumulated_data.drain(0..frame_size).collect::<Vec<u8>>();

                    // Create a new VideoFrame
                    let frame = VideoFrame::new(frame_data);

                    // Push to the buffer
                    video_buffer.lock().unwrap().push_frame(frame);
                }
            }
            Err(e) => {
                eprintln!("Error reading from ffmpeg: {}", e);
                break;
            }
        }
    }

    // If we have any leftover data that's not a complete frame
    if !accumulated_data.is_empty() {
        println!("Leftover incomplete data: {} bytes", accumulated_data.len());
    }

    // Wait for processes to complete (though they might be terminated by Ctrl-C)
    let _ = ffmpeg_process.wait();
    let _ = yt_dlp_process.wait();
}
