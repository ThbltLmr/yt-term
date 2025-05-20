use std::{
    io::Read,
    process::{Command, Stdio},
};

use clap::Parser;
use video_buffer::{VideoBuffer, VideoFrame};

mod result;
mod video_buffer;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long)]
    url: String,
}

fn main() {
    let args = Args::parse();
    let url = args.url;
    let width = 1280;
    let height = 720;

    let frame_size = width * height * 3;

    let mut buffer = VideoBuffer::new();

    let mut yt_dlp_process = Command::new("yt-dlp")
        .args([
            "-o",
            "-",         // Output to stdout
            "--no-part", // Don't create .part files
            "-f",
            "247",
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
    let mut timestamp = 0;
    let mut accumulated_data = Vec::new();
    // Buffer for reading chunks from stdout
    let mut read_buffer = vec![0u8; 32768]; // 32KB chunks

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
                    let frame = VideoFrame::new(frame_data, timestamp);

                    // Push to the buffer
                    buffer.push_frame(frame);

                    // Update timestamp (assuming ~30fps, we increment by ~33ms)
                    timestamp += 33;

                    println!(
                        "Frame buffered: {} (Buffer size: {})",
                        timestamp,
                        buffer.len()
                    );

                    // You can add a small sleep to simulate processing time
                    // Comment this out for maximum processing speed
                    // thread::sleep(Duration::from_millis(30));
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
